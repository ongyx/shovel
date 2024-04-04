use clap;
use eyre;
use eyre::WrapErr;
use owo_colors::OwoColorize;
use shovel;

use crate::run::Run;

#[derive(clap::Args)]
pub struct RemoveCommand {
	/// The existing bucket's name.
	name: String,
}

impl Run for RemoveCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		shovel
			.buckets
			.remove(&self.name)
			.wrap_err_with(|| format!("Failed to remove bucket {}", self.name))?;

		println!("Removed bucket {}", self.name.bold());

		Ok(())
	}
}
