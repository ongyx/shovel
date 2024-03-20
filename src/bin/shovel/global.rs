use std::fs;
use std::iter;

use clap;
use eyre::{self, WrapErr};
use regex;
use shovel;
use tabled;

use crate::bucket::BucketCommands;
use crate::run::Run;
use crate::util::{parse_app, tableify};

#[derive(clap::Subcommand)]
pub enum GlobalCommands {
    /// Manage buckets
    #[command(subcommand)]
    Bucket(BucketCommands),

    /// Show an app's manifest
    Cat(CatCommand),

    /// Show an app's info
    Info(InfoCommand),

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
            Self::Info(cmd) => cmd.run(shovel),
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

                let (bucket, app) = &apps[0];
                let bucket = shovel.buckets.open(bucket)?;
                let manifest_path = bucket.manifest_path(app);

                print!("{}", fs::read_to_string(manifest_path)?);

                Ok(())
            }
        }
    }
}

#[derive(tabled::Tabled, Debug)]
#[tabled(rename_all = "pascal")]
struct Info {
    name: String,
    description: String,
    version: String,
    bucket: String,
    website: String,
    license: String,
    #[tabled(rename = "Updated at")]
    updated_at: String,
    #[tabled(rename = "Updated by")]
    updated_by: String,
    installed: String,
    binaries: String,
    shortcuts: String,
}

impl Info {
    fn new(shovel: &mut shovel::Shovel, name: &str) -> shovel::Result<Self> {
        use shovel::Error::AppNotFound;

        let (bucket, manifest) = shovel.buckets.manifest(name)?;

        let license = manifest.license.to_string();

        let commit = bucket.manifest_commit(name)?;

        let updated_at = shovel::Timestamp::from(commit.time()).to_string();

        let updated_by = commit.author().name().unwrap().to_owned();

        let app = shovel.apps.open_current(name);

        let installed = match app {
            Ok(app) => Ok(app.manifest()?.version),
            // If the app is not found, do not propagate the error.
            Err(AppNotFound) => Ok("(not installed)".to_owned()),
            Err(err) => Err(err),
        }?;

        let binaries = manifest
            .bin()
            .map(|bins| bins.to_string())
            .unwrap_or_default();

        let shortcuts = manifest
            .shortcuts()
            .map(|shortcuts| {
                let shortcuts: Vec<_> = shortcuts
                    .iter()
                    .map(|shortcut| shortcut.to_string())
                    .collect();

                shortcuts.join(" | ")
            })
            .unwrap_or_default();

        Ok(Self {
            name: name.to_owned(),
            description: manifest.description.unwrap_or_default(),
            version: manifest.version,
            bucket: bucket.name(),
            website: manifest.homepage,
            license,
            updated_at,
            updated_by,
            installed,
            binaries,
            shortcuts,
        })
    }
}

#[derive(clap::Args)]
pub struct InfoCommand {
    pub app: String,
}

impl Run for InfoCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let info = Info::new(shovel, &self.app)?;

        let table = tableify(iter::once(info), true);

        println!("\n{}\n", table);

        Ok(())
    }
}

#[derive(tabled::Tabled, Default)]
#[tabled(rename_all = "pascal")]
struct ListInfo {
    name: String,
    version: String,
    bucket: String,
    updated: String,
    info: String,
}

impl ListInfo {
    fn new(name: &str, app: &shovel::App) -> shovel::Result<Self> {
        let manifest = app.manifest()?;
        let metadata = app.metadata()?;

        let version = manifest.version;
        let bucket = metadata.bucket;
        let updated = app.timestamp()?.to_string();

        Ok(Self {
            name: name.to_owned(),
            version,
            bucket,
            updated,
            ..Default::default()
        })
    }
}

#[derive(clap::Args)]
pub struct ListCommand {
    /// The apps to list as a regex. To specify a bucket, use the syntax `bucket/pattern`.
    query: Option<String>,
}

impl ListCommand {
    fn app_info(&self, shovel: &shovel::Shovel, name: &str) -> ListInfo {
        match shovel.apps.open_current(name) {
            Ok(app) => ListInfo::new(name, &app).unwrap_or_else(|_| ListInfo {
                name: name.to_owned(),
                // Use placeholders if the app's manifest/metadata cannot be read.
                ..Default::default()
            }),
            Err(err) => ListInfo {
                name: name.to_owned(),
                info: err.to_string(),
                ..Default::default()
            },
        }
    }
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
            .map(|name| self.app_info(shovel, &name))
            .filter_map(|info| {
                // check the bucket and name.
                if (bucket.is_empty() || info.bucket == bucket)
                    && (app.is_empty() || regex.is_match(&info.name))
                {
                    Some(info)
                } else {
                    None
                }
            })
            .collect();

        match apps.len() {
            0 => eyre::bail!("No app(s) found."),
            _ => {
                println!("\n{}\n", tableify(apps, false));

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
        let version = manifest.version.clone();
        let binaries = manifest
            .bin()
            .map(|bins| bins.to_string())
            .unwrap_or_default();

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
                let bucket = shovel.buckets.open(&b)?;
                let manifest = bucket.manifest(&a)?;

                Ok(SearchInfo::new(b, a, &manifest))
            })
            .collect();

        let apps = apps?;

        match apps.len() {
            0 => eyre::bail!("No app(s) found."),
            _ => {
                println!("\n{}\n", tableify(apps, false));

                Ok(())
            }
        }
    }
}
