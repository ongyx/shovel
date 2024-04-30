use std::io;

use crate::run::Run;
use crate::util;

#[derive(clap::Args)]
pub struct CatCommand {
	/// The manifest's name. To specify a bucket, use the syntax `bucket/manifest`.
	manifest: String,
}

impl Run for CatCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let (bucket_name, manifest_name) = util::parse_app(&self.manifest);

		let mut opts = shovel::CatOptions::new(manifest_name, io::stdout());

		if !bucket_name.is_empty() {
			opts.bucket(bucket_name);
		}

		if shovel.cat(&mut opts)? {
			Ok(())
		} else {
			Err(eyre::eyre!("Manifest not found"))
		}
	}
}
