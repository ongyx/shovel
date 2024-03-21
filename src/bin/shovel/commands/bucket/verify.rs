use std::sync;

use clap;
use eyre;
use jsonschema;
use owo_colors::OwoColorize;
use shovel;

use crate::run::Run;

static SCHEMA_JSON: &'static str =
    include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema.json"));
static SCHEMA: sync::OnceLock<jsonschema::JSONSchema> = sync::OnceLock::new();

/// Returns the JSON schema for verifying manfiests.
pub fn schema() -> &'static jsonschema::JSONSchema {
    SCHEMA.get_or_init(|| {
        let value = serde_json::from_str(SCHEMA_JSON).unwrap();

        jsonschema::JSONSchema::compile(&value).unwrap()
    })
}

/// Formats an iterator over JSON Schema errors.
fn format_schema_errors(errors: jsonschema::ErrorIterator<'_>) -> Vec<String> {
    errors
        .map(|err| format!("{:?} for {}", err.kind.red(), err.instance_path.blue(),))
        .collect()
}

enum Verified {
    Success,
    Failure {
        name: String,
        error: eyre::Error,
        schema_errors: Vec<String>,
    },
}

#[derive(clap::Args)]
pub struct VerifyCommand {
    /// The bucket to verify manifests for. If not specified, all buckets are verified.
    bucket: Option<String>,

    /// Whether or not to validate the manifests against the JSON schema.
    #[arg(short, long, default_value_t = false)]
    schema: bool,
}

impl VerifyCommand {
    fn verify<'sh>(
        &'sh self,
        shovel: &'sh mut shovel::Shovel,
        bucket_name: &str,
    ) -> eyre::Result<impl Iterator<Item = Verified> + 'sh> {
        use Verified::*;

        let bucket = shovel.buckets.open(bucket_name)?;
        let manifests = bucket.manifests()?;

        let verified = manifests.map(move |name| -> Verified {
            // Attempt to parse the manifest.
            match bucket.manifest(&name) {
                Ok(_) => {
                    // Only verify against the schema if successfully parsed.
                    if self.schema {
                        let path = bucket.manifest_path(&name);
                        let value = shovel::json::from_file(path).unwrap();

                        if let Err(errors) = schema().validate(&value) {
                            return Failure {
                                name,
                                error: eyre::eyre!("Failed to validate against JSON Schema"),
                                schema_errors: format_schema_errors(errors),
                            };
                        };
                    }

                    Success
                }
                Err(error) => Failure {
                    name,
                    error: error.into(),
                    schema_errors: vec![],
                },
            }
        });

        Ok(verified)
    }
}

impl Run for VerifyCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        use Verified::*;

        let bucket_names = match &self.bucket {
            Some(name) => vec![name.to_owned()],
            None => shovel.buckets.iter()?.collect(),
        };

        for bucket_name in bucket_names {
            println!();

            let mut success = 0;
            let mut failures = vec![];

            for verified in self.verify(shovel, &bucket_name)? {
                match verified {
                    Success => success += 1,
                    Failure {
                        name,
                        error,
                        schema_errors,
                    } => failures.push((name, error, schema_errors)),
                }
            }

            match failures.len() {
                0 => println!(
                    "{}: parsed {} manifests",
                    bucket_name.bold(),
                    success.to_string().green()
                ),
                n => {
                    println!(
                        "{}: parsed {} manifests, {} failed:",
                        bucket_name.bold(),
                        success.to_string().green(),
                        n.to_string().red(),
                    );

                    // Print out all errors.
                    for (name, error, schema_errors) in failures {
                        println!("* {}: {}", name.bold(), error);

                        for schema_error in schema_errors {
                            println!("  * {}", schema_error);
                        }
                    }
                }
            }
        }

        println!();

        Ok(())
    }
}
