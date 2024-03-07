use anyhow::{anyhow, Context};
use regex;

use crate::cli::bucket::BucketCommands;
use crate::cli::run::Run;
use crate::shovel::Shovel;

#[derive(clap::Subcommand)]
pub enum GlobalCommands {
    /// Search for an app
    Search(SearchCommand),

    /// Manage buckets
    #[command(subcommand)]
    Bucket(BucketCommands),
}

impl Run for GlobalCommands {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        match self {
            Self::Search(cmd) => cmd.run(shovel),
            Self::Bucket(cmds) => cmds.run(shovel),
        }
    }
}

#[derive(clap::Args)]
pub struct SearchCommand {
    /// The search pattern as a regex.
    pattern: String,

    /// The bucket to search.
    #[arg(short, long)]
    bucket: Option<String>,
}

impl Run for SearchCommand {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        let regex = regex::Regex::new(&self.pattern).context("Invalid pattern")?;

        let mut manifests = shovel
            .search(|b, m| {
                // If bucket is not None, check the bucket name.
                if let Some(bk) = &self.bucket {
                    if b != bk {
                        return false;
                    }
                }

                regex.is_match(m)
            })
            .context("Search failed")?
            .peekable();

        if manifests.peek().is_none() {
            return Err(anyhow!("No app(s) found."));
        }

        for (bucket, manifest) in manifests {
            println!("{}/{}", bucket, manifest);
        }

        Ok(())
    }
}
