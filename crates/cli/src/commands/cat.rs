use std::fs;
use std::path::PathBuf;

use crate::run::Run;
use crate::util;

#[derive(clap::Args)]
pub struct CatCommand {
	/// The app's name. To specify a bucket, use the syntax `bucket/app`.
	app: String,
}

impl CatCommand {
	fn path(&self, shovel: &mut shovel::Shovel) -> eyre::Result<PathBuf> {
		let (bucket_name, manifest_name) = util::parse_app(&self.app);

		let mut search = shovel.buckets.search_all(
			|bucket| bucket_name.is_empty() || (bucket.name() == bucket_name),
			|manifest| manifest == manifest_name,
		)?;

		let (bucket, item) = search
			.next()
			.ok_or_else(|| eyre::eyre!("Search results are empty"))?;

		Ok(bucket.manifest_path(&item.name))
	}
}

impl Run for CatCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let path = self.path(shovel)?;

		print!("{}", fs::read_to_string(path)?);

		Ok(())
	}
}
