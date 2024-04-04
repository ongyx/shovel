mod show;

use clap;

use crate::run::Run;

#[derive(clap::Subcommand)]
pub enum CacheCommands {
	/// Show the cache
	Show(show::ShowCommand),
}

impl Run for CacheCommands {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		match self {
			Self::Show(cmd) => cmd.run(shovel),
		}
	}
}
