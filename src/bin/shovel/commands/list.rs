use clap;
use eyre::WrapErr;
use shovel;
use tabled;

use crate::run::Run;
use crate::util;

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
    fn app_info(&self, name: &str, app: shovel::Result<shovel::App>) -> ListInfo {
        match app {
            Ok(app) => ListInfo::new(&name, &app).unwrap_or_else(|_| ListInfo {
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

        let (bucket, app) = util::parse_app(query);

        let regex = regex::Regex::new(app).wrap_err("Invalid pattern")?;

        let apps: Vec<_> = shovel
            .apps
            .iter()?
            .map(|(name, app)| self.app_info(&name, app))
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
                println!("\n{}\n", util::tableify(apps, false));

                Ok(())
            }
        }
    }
}
