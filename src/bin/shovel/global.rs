use std::fs;
use std::time;

use clap;
use color_eyre::Section;
use eyre;
use eyre::WrapErr;
use regex;
use shovel;
use shovel::app::manifest;
use tabled;

use crate::bucket::BucketCommands;
use crate::run::Run;
use crate::util::{parse_app, tableify, unix_to_human};

#[derive(clap::Subcommand)]
pub enum GlobalCommands {
    /// Manage buckets
    #[command(subcommand)]
    Bucket(BucketCommands),

    /// Show an app's manifest
    Cat(CatCommand),

    /// List installed apps
    List(ListCommand),

    /// Search for an app
    Search(SearchCommand),
}

impl Run for GlobalCommands {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        match self {
            Self::Bucket(cmds) => cmds.run(shovel),
            Self::Cat(cmd) => cmd.run(shovel),
            Self::List(cmd) => cmd.run(shovel),
            Self::Search(cmd) => cmd.run(shovel),
        }
    }
}

#[derive(clap::Args)]
pub struct CatCommand {
    /// The app's name. To specify a bucket, use the syntax `bucket/app`.
    app: String,
}

impl Run for CatCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let (bucket, app) = parse_app(&self.app);

        let apps: Vec<_> = shovel
            .buckets
            .search(|b, a| (bucket.is_empty() || b == bucket) && a == app)
            .wrap_err("Search failed")?
            .collect();

        match apps.len() {
            0 => eyre::bail!("App not found."),
            _ => {
                if apps.len() > 1 {
                    println!(
                        "Warning: One or more apps have the same name. Using the first result"
                    );
                }

                let app = &apps[0];
                let bucket = shovel.buckets.get(&app.0)?;
                let manifest_path = bucket.manifest_path(&app.1);

                print!("{}", fs::read_to_string(manifest_path)?);

                Ok(())
            }
        }
    }
}

#[derive(tabled::Tabled)]
#[tabled(rename_all = "pascal")]
pub struct ListInfo {
    name: String,
    version: String,
    bucket: String,
    updated: String,
}

impl ListInfo {
    fn new(name: &str, app: &shovel::App) -> shovel::Result<Self> {
        let manifest = app.manifest()?;
        let metadata = app.metadata()?;

        let version = manifest.version;
        let bucket = metadata.bucket;
        // https://doc.rust-lang.org/std/time/struct.SystemTime.html#associatedconstant.UNIX_EPOCH
        let updated_ts = app
            .dir()
            .metadata()?
            .modified()?
            .duration_since(time::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let updated = unix_to_human(updated_ts as i64);

        Ok(Self {
            name: name.to_owned(),
            version,
            bucket,
            updated,
        })
    }
}

#[derive(clap::Args)]
pub struct ListCommand {
    /// The apps to list as a regex. To specify a bucket, use the syntax `bucket/pattern`.
    query: Option<String>,
}

impl Run for ListCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let query = match &self.query {
            Some(q) => q,
            None => "",
        };

        let (bucket, app) = parse_app(query);

        let regex = regex::Regex::new(app).wrap_err("Invalid pattern")?;

        let apps: Vec<_> = shovel
            .apps
            .iter()?
            .map(|a| -> eyre::Result<_> {
                let app = shovel
                    .apps
                    // If the app's current version is missing, the installation is corrupt.
                    .get_current(&a)
                    .wrap_err_with(|| format!("Failed to open app {:?}", &a))?;
                let info = ListInfo::new(&a, &app)
                    .wrap_err_with(|| format!("Failed to read app {:?}", &a))?;

                Ok(info)
            })
            .filter_map(|r| match r {
                // If there is info, check the bucket and name.
                Ok(info) => {
                    if (bucket.is_empty() || info.bucket == bucket)
                        && (app.is_empty() || regex.is_match(&info.name))
                    {
                        Some(info)
                    } else {
                        None
                    }
                }
                Err(err) => {
                    // Print error and move on.
                    println!("{:?}", err.warning("Skipping app"));

                    None
                }
            })
            .collect();

        match apps.len() {
            0 => eyre::bail!("No app(s) found."),
            _ => {
                println!("\n{}\n", tableify(apps));

                Ok(())
            }
        }
    }
}

#[derive(tabled::Tabled)]
#[tabled(rename_all = "pascal")]
struct SearchInfo {
    name: String,
    version: String,
    bucket: String,
    binaries: String,
}

impl SearchInfo {
    fn new(bucket: String, name: String, manifest: &shovel::Manifest) -> Self {
        use manifest::{Bin, Bins};

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

        SearchInfo {
            name,
            version,
            bucket,
            binaries,
        }
    }
}

#[derive(clap::Args)]
pub struct SearchCommand {
    /// The apps to search as a regex. To specify a bucket, use the syntax `bucket/pattern`.
    query: String,
}

impl Run for SearchCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let (bucket, app) = parse_app(&self.query);

        let regex = regex::Regex::new(app).wrap_err("Invalid pattern")?;

        let apps: shovel::Result<Vec<_>> = shovel
            .buckets
            .search(|b, a| (bucket.is_empty() || b == bucket) && regex.is_match(a))
            .wrap_err("Search failed")?
            .map(|(b, a)| {
                let bucket = shovel.buckets.get(&b)?;
                let manifest = bucket.manifest(&a)?;

                Ok(SearchInfo::new(b, a, &manifest))
            })
            .collect();

        let apps = apps?;

        match apps.len() {
            0 => eyre::bail!("No app(s) found."),
            _ => {
                println!("\n{}\n", tableify(apps));

                Ok(())
            }
        }
    }
}
