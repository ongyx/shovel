use std::fs;
use std::path::PathBuf;

use clap;
use eyre;
use shovel;

use crate::run::Run;
use crate::util;

#[derive(clap::Args)]
pub struct CatCommand {
    /// The app's name. To specify a bucket, use the syntax `bucket/app`.
    app: String,
}

impl CatCommand {
    fn path(&self, shovel: &mut shovel::Shovel) -> shovel::Result<PathBuf> {
        let (bucket_name, manifest_name) = util::parse_app(&self.app);

        let items: Vec<_> = shovel
            .buckets
            .search(|bucket, manifest| {
                (bucket_name.is_empty() || bucket == bucket_name) && manifest == manifest_name
            })?
            .collect();

        match items.len() {
            0 => Err(shovel::Error::AppNotFound),
            _ => {
                let item = &items[0];
                let path = item.bucket.manifest_path(&item.name);
                Ok(path)
            }
        }
    }
}

impl Run for CatCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let path = self.path(shovel)?;

        print!("{}", fs::read_to_string(path)?);

        Ok(())
    }
}
