use anyhow::{anyhow, Context};
use regex;
use shovel::app::manifest::Bin;
use shovel::{Manifest, Shovel};
use tabled::Tabled;

use crate::bucket::BucketCommands;
use crate::run::Run;
use crate::util::tableify;

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

#[derive(Tabled)]
#[tabled(rename_all = "pascal")]
struct AppInfo {
    name: String,
    version: String,
    bucket: String,
    binaries: String,
}

impl AppInfo {
    fn new(bucket: String, name: String, manifest: &Manifest) -> Self {
        let version = manifest.version.clone();
        let binaries = match &manifest.common.bin {
            Some(list) => {
                let bins: Vec<String> = list
                    .items
                    .iter()
                    .map(|b| match b {
                        Bin::Path(p) => p.clone(),
                        Bin::Shim(s) => {
                            format!("{} => {} {}", s.name, s.executable, s.arguments.join(" "))
                        }
                    })
                    .collect();

                bins.join(", ")
            }
            None => "".to_owned(),
        };

        AppInfo {
            name,
            version,
            bucket,
            binaries,
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

        let manifests: anyhow::Result<Vec<_>> = shovel
            .buckets
            .search(|b, a| {
                // If bucket is not None, check the bucket name.
                if let Some(bk) = &self.bucket {
                    if b != bk {
                        return false;
                    }
                }

                regex.is_match(a)
            })
            .context("Search failed")?
            .map(|(b, a)| {
                let bucket = shovel.buckets.get(&b)?;
                let manifest = bucket.manifest(&a)?;

                Ok(AppInfo::new(b, a, &manifest))
            })
            .collect();

        let manifests = manifests?;

        match manifests.len() {
            0 => Err(anyhow!("No app(s) found.")),
            _ => {
                println!("\n{}\n", tableify(manifests));

                Ok(())
            }
        }
    }
}
