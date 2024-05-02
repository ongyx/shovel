use std::io;

use shovel::bucket::Name;

use crate::run::Run;

#[derive(clap::Args)]
pub struct CatCommand {
	/// The manifest's name. To specify a bucket, use the syntax `bucket/manifest`.
	manifest: String,
}

impl Run for CatCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let name = Name::new(self.manifest.clone());

		let mut opts = shovel::CatOptions::new(name, io::stdout());

		shovel.cat(&mut opts)?;

		Ok(())
	}
}
