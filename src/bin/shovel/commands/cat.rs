use std::fs;

use clap;
use eyre;
use eyre::WrapErr;
use shovel;

use crate::run::Run;
use crate::util;

#[derive(clap::Args)]
pub struct CatCommand {
    /// The app's name. To specify a bucket, use the syntax `bucket/app`.
    app: String,
}

impl Run for CatCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let (bucket, app) = util::parse_app(&self.app);

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
