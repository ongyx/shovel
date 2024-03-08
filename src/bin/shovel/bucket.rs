use anyhow::{Context, Result};
use chrono::{DateTime, Local};
use clap::{Args, Subcommand};
use shovel::{Bucket, Shovel, ShovelResult};

use tabled::Tabled;

use crate::run::Run;
use crate::util::tableify;

#[derive(Subcommand)]
pub enum BucketCommands {
    /// Add a bucket
    Add(AddCommand),

    /// Remove a bucket
    #[clap(visible_alias("rm"))]
    Remove(RemoveCommand),

    /// List all buckets
    List(ListCommand),

    /// Verify apps in a bucket
    Verify(VerifyCommand),
}

impl Run for BucketCommands {
    fn run(&self, shovel: &mut Shovel) -> Result<()> {
        match self {
            Self::Add(cmd) => cmd.run(shovel),
            Self::Remove(cmd) => cmd.run(shovel),
            Self::List(cmd) => cmd.run(shovel),
            Self::Verify(cmd) => cmd.run(shovel),
        }
    }
}

#[derive(Args)]
pub struct AddCommand {
    /// The new bucket's name.
    name: String,

    /// The new bucket's URL.
    url: String,
}

impl Run for AddCommand {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        shovel
            .add_bucket(&self.name, &self.url)
            .with_context(|| format!("Failed to add bucket {}", self.name))?;

        println!("Added bucket {} from {}.", self.name, self.url);

        Ok(())
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
            .remove_bucket(&self.name)
            .with_context(|| format!("Failed to remove bucket {}", self.name))?;

        println!("Removed bucket {}.", self.name);

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
        let updated = DateTime::from_timestamp(bucket.timestamp()?, 0)
            .unwrap()
            .with_timezone(&Local)
            .format("%d/%m/%Y %H:%M:%S %P")
            .to_string();
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
        let infos: ShovelResult<Vec<_>> = shovel
            .buckets()?
            .map(|n| shovel.bucket(&n).and_then(|b| BucketInfo::new(b)))
            .collect();

        println!("\n{}\n", tableify(infos?));

        Ok(())
    }
}

#[derive(Args)]
pub struct VerifyCommand {
    /// The bucket to verify apps for.
    bucket: String,
}

impl Run for VerifyCommand {
    fn run(&self, shovel: &mut Shovel) -> Result<()> {
        let bucket = shovel.bucket(&self.bucket)?;

        let mut count = 0;

        for manifest_name in bucket.manifests()? {
            bucket.manifest(&manifest_name).with_context(|| {
                format!(
                    "Failed parsing manifest {}",
                    bucket
                        .manifest_path(&manifest_name)
                        .unwrap()
                        .to_string_lossy()
                )
            })?;

            count += 1;
        }

        println!("Ok: {} manifests parsed", count);

        Ok(())
    }
}
