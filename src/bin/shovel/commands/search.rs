use clap;
use eyre;
use eyre::WrapErr;
use shovel;
use tabled;

use crate::run::Run;
use crate::util;

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
        let (bucket, app) = util::parse_app(&self.query);

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
                println!("\n{}\n", util::tableify(apps, false));

                Ok(())
            }
        }
    }
}
