use std::fs;
use std::sync::OnceLock;

use eyre::WrapErr;
use owo_colors::OwoColorize;
use shovel::bucket;
use shovel::json;

use crate::run::Run;

static SCHEMA_JSON: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/schema.json"));
static SCHEMA: OnceLock<jsonschema::JSONSchema> = OnceLock::new();

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
	fn validate(name: &str, bucket: &bucket::Bucket) -> eyre::Result<Verified> {
		use Verified::*;

		let path = bucket.manifest_path(name);
		let file = fs::File::open(&path)
			.wrap_err_with(|| format!("Failed to open manifest at {}", path.display()))?;

		let value = json::from_reader(file)?;
		let valid = schema().validate(&value);

		let verified = valid.map_or_else(
			|errs| Failure {
				name: name.to_owned(),
				error: eyre::eyre!("Failed to validate against JSON Schema"),
				schema_errors: format_schema_errors(errs),
			},
			|()| Success,
		);

		Ok(verified)
	}

	fn verify<'b>(
		&'b self,
		bucket: &'b bucket::Bucket,
	) -> eyre::Result<impl Iterator<Item = eyre::Result<Verified>> + 'b> {
		use Verified::*;

		let name = bucket.name();
		let verified = bucket.manifests()?.map(move |item| {
			let manifest = item.manifest.map(|m| m.validate().map(|()| m));

			// Check if the manifest parsed successfully.
			let verified = match manifest {
				Ok(_) => {
					// Only verify against the schema if successfully parsed.
					if self.schema {
						Self::validate(&item.name, bucket)?
					} else {
						Success
					}
				}
				Err(error) => Failure {
					name: name.clone(),
					error: error.into(),
					schema_errors: vec![],
				},
			};

			Ok(verified)
		});

		Ok(verified)
	}
}

impl Run for VerifyCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		use Verified::*;

		let buckets = match &self.bucket {
			Some(name) => vec![shovel.buckets.open(name)],
			None => shovel.buckets.iter()?.collect(),
		};

		for bucket in buckets {
			println!();

			let bucket = bucket?;
			let name = bucket.name();

			let mut success = 0;
			let mut failures = vec![];

			for verified in self.verify(&bucket)? {
				// TODO: Show error instead of bailing?
				let verified = verified?;

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
					name.bold(),
					success.to_string().green()
				),
				n => {
					println!(
						"{}: parsed {} manifests, {} failed:",
						name.bold(),
						success.to_string().green(),
						n.to_string().red(),
					);

					// Print out all errors.
					for (name, error, schema_errors) in failures {
						println!("* {}: {}", name.bold(), error);

						for schema_error in schema_errors {
							println!("  * {schema_error}");
						}
					}
				}
			}
		}

		println!();

		Ok(())
	}
}
