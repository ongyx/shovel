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
    fn new(item: shovel::Item) -> shovel::Result<Self> {
        let manifest = item.manifest?;

        let version = manifest.version.clone();
        let binaries = manifest
            .bin()
            .map(|bins| bins.to_string())
            .unwrap_or_default();

        Ok(SearchInfo {
            name: item.name,
            version,
            bucket: item.bucket.name(),
            binaries,
        })
    }
}

#[derive(clap::Args)]
pub struct SearchCommand {
    /// The apps to search as a regex. To specify a bucket, use the syntax `bucket/pattern`.
    query: String,
}

impl Run for SearchCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let (bucket_name, manifest_name) = util::parse_app(&self.query);

        let regex = regex::Regex::new(manifest_name).wrap_err("Invalid pattern")?;

        let apps: shovel::Result<Vec<_>> = shovel
            .buckets
            .search(|bucket, manifest| {
                (bucket_name.is_empty() || bucket == bucket_name) && regex.is_match(manifest)
            })
            .wrap_err("Search failed")?
            .map(|item| SearchInfo::new(item))
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
