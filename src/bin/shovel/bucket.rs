use anyhow::{bail, Context, Result};
use clap::{Args, Subcommand};
use colored::Colorize;
use phf::phf_map;
use shovel::{Bucket, Result as ShovelResult, Shovel};

use tabled::Tabled;

use crate::run::Run;
use crate::util::{tableify, unix_to_human};

/// Map of known bucket names to their URLs.
/// Derived from https://github.com/ScoopInstaller/Scoop/blob/master/buckets.json
static KNOWN_BUCKETS: phf::Map<&'static str, &'static str> = phf_map! {
    "main" => "https://github.com/ScoopInstaller/Main",
    "extras" => "https://github.com/ScoopInstaller/Extras",
    "versions" => "https://github.com/ScoopInstaller/Versions",
    "nirsoft" => "https://github.com/kodybrown/scoop-nirsoft",
    "sysinternals" => "https://github.com/niheaven/scoop-sysinternals",
    "php" => "https://github.com/ScoopInstaller/PHP",
    "nerd-fonts" => "https://github.com/matthewjberger/scoop-nerd-fonts",
    "nonportable" => "https://github.com/ScoopInstaller/Nonportable",
    "java" => "https://github.com/ScoopInstaller/Java",
    "games" => "https://github.com/Calinou/scoop-games"
};

/// Returns the URL of the known bucket by name.
fn known_bucket(name: &str) -> Option<&'static str> {
    KNOWN_BUCKETS.get(name).map(|u| *u)
}

#[derive(Subcommand)]
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
    fn run(&self, shovel: &mut Shovel) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.run(shovel),
            Self::Remove(cmd) => cmd.run(shovel),
            Self::List(cmd) => cmd.run(shovel),
            Self::Known(cmd) => cmd.run(shovel),
            Self::Verify(cmd) => cmd.run(shovel),
        }
    }
}

#[derive(Args)]
pub struct AddCommand {
    /// The bucket name.
    name: String,

    /// The bucket URL.
    /// Required if the bucket name is not known - run `shovel bucket known` for details.
    url: Option<String>,
}

impl Run for AddCommand {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        let url = self
            .url
            .as_ref()
            .map(|u| u.as_str())
            .or_else(|| known_bucket(&self.name));

        match url {
            Some(url) => {
                shovel
                    .buckets
                    .add(&self.name, &url)
                    .with_context(|| format!("Failed to add bucket {}", self.name))?;

                println!("Added bucket {} from {}", self.name.bold(), url.green());

                Ok(())
            }
            None => bail!("URL was not specified"),
        }
    }
}

#[derive(Args)]
pub struct RemoveCommand {
    /// The existing bucket's name.
    name: String,
}

impl Run for RemoveCommand {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        shovel
            .buckets
            .remove(&self.name)
            .with_context(|| format!("Failed to remove bucket {}", self.name))?;

        println!("Removed bucket {}", self.name.bold());

        Ok(())
    }
}

#[derive(Tabled)]
#[tabled(rename_all = "pascal")]
struct BucketInfo {
    name: String,
    source: String,
    updated: String,
    manifests: usize,
}

impl BucketInfo {
    fn new(bucket: &Bucket) -> ShovelResult<Self> {
        let name = bucket.name();
        let source = bucket.origin()?;
        let updated = unix_to_human(bucket.timestamp()?);
        let manifests = bucket.manifests()?.count();

        Ok(Self {
            name,
            source,
            updated,
            manifests,
        })
    }
}

#[derive(Args)]
pub struct ListCommand {}

impl ListCommand {}

impl Run for ListCommand {
    fn run(&self, shovel: &mut Shovel) -> Result<()> {
        let info: ShovelResult<Vec<_>> = shovel
            .buckets
            .iter()?
            .map(|n| shovel.buckets.get(&n).and_then(|b| BucketInfo::new(b)))
            .collect();

        println!("\n{}\n", tableify(info?));

        Ok(())
    }
}

#[derive(Tabled)]
#[tabled(rename_all = "pascal")]
struct KnownInfo {
    name: &'static str,
    source: &'static str,
}

#[derive(Args)]
pub struct KnownCommand {}

impl Run for KnownCommand {
    fn run(&self, _shovel: &mut Shovel) -> anyhow::Result<()> {
        let known = KNOWN_BUCKETS
            .into_iter()
            .map(|(name, source)| KnownInfo { name, source });

        println!("\n{}\n", tableify(known));

        Ok(())
    }
}

#[derive(Args)]
pub struct VerifyCommand {
    /// The bucket to verify apps for. If not specified, all buckets are verified.
    bucket: Option<String>,
}

impl VerifyCommand {
    fn verify(&self, shovel: &mut Shovel, bucket_name: &str) -> Result<(i32, i32)> {
        let bucket = shovel.buckets.get(bucket_name)?;

        let mut success = 0;
        let mut failure = 0;

        for manifest_name in bucket.manifests()? {
            let result = bucket.manifest(&manifest_name).context(format!(
                "Failed parsing manifest {}",
                bucket.manifest_path(&manifest_name).to_string_lossy()
            ));

            match result {
                Ok(_) => success += 1,
                Err(err) => {
                    println!("{:?}", err);

                    failure += 1;
                }
            }
        }

        Ok((success, failure))
    }
}

impl Run for VerifyCommand {
    fn run(&self, shovel: &mut Shovel) -> Result<()> {
        let bucket_names = match &self.bucket {
            Some(name) => vec![name.to_owned()],
            None => shovel.buckets.iter()?.collect(),
        };

        for bucket_name in bucket_names {
            let (success, failure) = self.verify(shovel, &bucket_name)?;

            match failure {
                0 => println!(
                    "{}: parsed {} manifests",
                    bucket_name.bold(),
                    success.to_string().green()
                ),
                _ => println!(
                    "{}: parsed {} manifests, {} failed",
                    bucket_name.bold(),
                    success.to_string().green(),
                    failure.to_string().red(),
                ),
            }
        }

        Ok(())
    }
}
