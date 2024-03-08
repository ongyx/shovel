use anyhow::Context;
use clap;
use shovel::Shovel;

use crate::run::Run;

#[derive(clap::Subcommand)]
pub enum BucketCommands {
    /// List all buckets
    List(ListCommand),

    /// Verify apps in a bucket
    Verify(VerifyCommand),
}

impl Run for BucketCommands {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        match self {
            Self::List(cmd) => cmd.run(shovel),
            Self::Verify(cmd) => cmd.run(shovel),
        }
    }
}

#[derive(clap::Args)]
pub struct ListCommand {}

impl Run for ListCommand {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        for bucket in shovel.buckets()? {
            println!("{}", bucket);
        }

        Ok(())
    }
}

#[derive(clap::Args)]
pub struct VerifyCommand {
    /// The bucket to verify apps for.
    bucket: String,
}

impl Run for VerifyCommand {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
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
