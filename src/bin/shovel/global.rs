use std::fs;

use anyhow::{anyhow, Context};
use clap::{Args, Subcommand};
use regex;
use shovel::app::manifest::{Bin, Bins};
use shovel::{Manifest, Shovel};
use tabled::Tabled;

use crate::bucket::BucketCommands;
use crate::run::Run;
use crate::util::{parse_app, tableify};

#[derive(Subcommand)]
pub enum GlobalCommands {
    /// Manage buckets
    #[command(subcommand)]
    Bucket(BucketCommands),

    /// Show an app's manifest
    Cat(CatCommand),

    /// Search for an app
    Search(SearchCommand),
}

impl Run for GlobalCommands {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        match self {
            Self::Bucket(cmds) => cmds.run(shovel),
            Self::Cat(cmd) => cmd.run(shovel),
            Self::Search(cmd) => cmd.run(shovel),
        }
    }
}

#[derive(Args)]
pub struct CatCommand {
    /// The app's name. To specify a bucket, use the syntax `bucket/app`.
    app: String,
}

impl Run for CatCommand {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        let (bucket, app) = parse_app(&self.app);

        let apps: Vec<_> = shovel
            .buckets
            .search(|b, a| (bucket.is_empty() || b == bucket) && a == app)
            .context("Search failed")?
            .collect();

        match apps.len() {
            0 => Err(anyhow!("App not found.")),
            _ => {
                if apps.len() > 1 {
                    println!(
                        "Warning: One or more apps have the same name. Using the first result"
                    );
                }

                let app = &apps[0];
                let bucket = shovel.buckets.get(&app.0)?;
                // Since the app was returned via search, it should exist.
                let manifest_path = bucket.manifest_path(&app.1).unwrap();

                print!("{}", fs::read_to_string(manifest_path)?);

                Ok(())
            }
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
            Some(bins) => match bins {
                Bins::One(p) => p.clone(),
                Bins::Many(ps) => {
                    let bins: Vec<String> = ps
                        .iter()
                        .map(|b| match b {
                            Bin::Path(p) => p.clone(),
                            Bin::Shim(s) => {
                                let cmd = [vec![s.executable.clone()], s.arguments.clone()]
                                    .concat()
                                    .join(" ");

                                format!("{} => {}", s.name, cmd)
                            }
                        })
                        .collect();

                    bins.join(" | ")
                }
            },
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

#[derive(Args)]
pub struct SearchCommand {
    /// The search pattern as a regex. To specify a bucket, use the syntax `bucket/pattern`.
    pattern: String,
}

impl Run for SearchCommand {
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        let (bucket, app) = parse_app(&self.pattern);

        let regex = regex::Regex::new(app).context("Invalid pattern")?;

        let apps: anyhow::Result<Vec<_>> = shovel
            .buckets
            .search(|b, a| (bucket.is_empty() || b == bucket) && regex.is_match(a))
            .context("Search failed")?
            .map(|(b, a)| {
                let bucket = shovel.buckets.get(&b)?;
                let manifest = bucket.manifest(&a)?;

                Ok(AppInfo::new(b, a, &manifest))
            })
            .collect();

        let apps = apps?;

        match apps.len() {
            0 => Err(anyhow!("No app(s) found.")),
            _ => {
                println!("\n{}\n", tableify(apps));

                Ok(())
            }
        }
    }
}
