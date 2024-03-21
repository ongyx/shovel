use std::sync;

use clap;
use eyre;
use eyre::{bail, WrapErr};
use git2::build;
use jsonschema;
use linya;
use owo_colors::OwoColorize;
use serde_json;
use shovel;
use tabled;

use crate::run::Run;
use crate::util::tableify;

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

include!(concat!(env!("OUT_DIR"), "/buckets.rs"));

/// Returns the URL of the known bucket by name.
fn known_bucket(name: &str) -> Option<&'static str> {
    KNOWN_BUCKETS.get(name).map(|u| *u)
}

#[derive(clap::Subcommand)]
pub enum BucketCommands {
    /// Add a bucket
    Add(AddCommand),

    /// Remove a bucket
    #[clap(visible_alias("rm"))]
    Remove(RemoveCommand),

    /// List all buckets
    List(ListCommand),

    /// List all known buckets
    Known(KnownCommand),

    /// Verify apps in a bucket
    Verify(VerifyCommand),
}

impl Run for BucketCommands {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        match self {
            Self::Add(cmd) => cmd.run(shovel),
            Self::Remove(cmd) => cmd.run(shovel),
            Self::List(cmd) => cmd.run(shovel),
            Self::Known(cmd) => cmd.run(shovel),
            Self::Verify(cmd) => cmd.run(shovel),
        }
    }
}

/// A progress tracker for clone operations in Git.
struct CloneTracker {
    progress: sync::Mutex<linya::Progress>,
    // These bars are initialized on first use.
    recv_bar: Option<linya::Bar>,
    cout_bar: Option<linya::Bar>,
}

impl CloneTracker {
    /// Returns a new clone tracker.
    pub fn new() -> Self {
        Self {
            progress: sync::Mutex::new(linya::Progress::new()),
            recv_bar: None,
            cout_bar: None,
        }
    }

    /// Returns a RepoBuilder that shows progress bars.
    pub fn builder<'p>(&'p mut self) -> build::RepoBuilder<'p> {
        let mut callbacks = git2::RemoteCallbacks::new();

        // Register a callback for receiving remote objects.
        callbacks.transfer_progress(|stats| {
            let current = stats.received_objects();
            let total = stats.total_objects();

            if self.recv_bar.is_none() {
                self.recv_bar = Some(
                    self.progress
                        .lock()
                        .unwrap()
                        .bar(total, "Receiving objects"),
                );
            }

            self.progress
                .lock()
                .unwrap()
                .set_and_draw(self.recv_bar.as_ref().unwrap(), current);

            true
        });

        let mut options = git2::FetchOptions::new();
        options.remote_callbacks(callbacks);

        let mut checkout = build::CheckoutBuilder::new();

        // Register a callback for checking out objects.
        checkout.progress(|_, current, total| {
            if self.cout_bar.is_none() {
                self.cout_bar = Some(
                    self.progress
                        .lock()
                        .unwrap()
                        .bar(total, "Checking out objects"),
                );
            }

            self.progress
                .lock()
                .unwrap()
                .set_and_draw(self.cout_bar.as_ref().unwrap(), current);
        });

        let mut builder = build::RepoBuilder::new();
        builder.fetch_options(options).with_checkout(checkout);

        builder
    }
}

#[derive(clap::Args)]
pub struct AddCommand {
    /// The bucket name.
    name: String,

    /// The bucket URL.
    /// Required if the bucket name is not known - run `shovel bucket known` for details.
    url: Option<String>,
}

impl Run for AddCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let url = self
            .url
            .as_ref()
            .map(|u| u.as_str())
            .or_else(|| known_bucket(&self.name));

        let mut tracker = CloneTracker::new();

        match url {
            Some(url) => {
                let mut builder = tracker.builder();

                shovel
                    .buckets
                    .add(&self.name, url, Some(&mut builder))
                    .wrap_err_with(|| format!("Failed to add bucket {}", self.name))?;

                println!("Added bucket {} from {}", self.name.bold(), url.green());

                Ok(())
            }
            None => bail!("URL was not specified"),
        }
    }
}

#[derive(clap::Args)]
pub struct RemoveCommand {
    /// The existing bucket's name.
    name: String,
}

impl Run for RemoveCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        shovel
            .buckets
            .remove(&self.name)
            .wrap_err_with(|| format!("Failed to remove bucket {}", self.name))?;

        println!("Removed bucket {}", self.name.bold());

        Ok(())
    }
}

#[derive(tabled::Tabled)]
#[tabled(rename_all = "pascal")]
struct BucketInfo {
    name: String,
    source: String,
    updated: String,
    manifests: usize,
}

impl BucketInfo {
    fn new(bucket: &shovel::Bucket) -> shovel::Result<Self> {
        let name = bucket.name();
        let source = bucket.origin()?;
        let commit = bucket.commit()?;
        let updated = shovel::Timestamp::from(commit.time()).to_string();
        let manifests = bucket.manifests()?.count();

        Ok(Self {
            name,
            source,
            updated,
            manifests,
        })
    }
}

#[derive(clap::Args)]
pub struct ListCommand {}

impl ListCommand {}

impl Run for ListCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let info: shovel::Result<Vec<_>> = shovel
            .buckets
            .iter()?
            .map(|n| shovel.buckets.open(&n).and_then(|b| BucketInfo::new(&b)))
            .collect();

        println!("\n{}\n", tableify(info?, false));

        Ok(())
    }
}

#[derive(tabled::Tabled)]
#[tabled(rename_all = "pascal")]
struct KnownInfo {
    name: &'static str,
    source: &'static str,
}

#[derive(clap::Args)]
pub struct KnownCommand {}

impl Run for KnownCommand {
    fn run(&self, _shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let known = KNOWN_BUCKETS
            .into_iter()
            .map(|(name, source)| KnownInfo { name, source });

        println!("\n{}\n", tableify(known, false));

        Ok(())
    }
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
    /// Formats an iterator over JSON Schema errors.
    fn format_schema_errors(errors: jsonschema::ErrorIterator<'_>) -> Vec<String> {
        errors
            .map(|err| format!("{:?} for {}", err.kind.red(), err.instance_path.blue(),))
            .collect()
    }

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
                                schema_errors: Self::format_schema_errors(errors),
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
