mod remove;
mod show;

use clap;

use crate::run::Run;

#[derive(clap::Args)]
#[command(args_conflicts_with_subcommands = true)]
pub struct CacheCommands {
	#[command(subcommand)]
	commands: Option<Commands>,

	#[command(flatten)]
	show: show::ShowCommand,
}

impl Run for CacheCommands {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let commands = self
			.commands
			.clone()
			.unwrap_or_else(|| Commands::Show(self.show.clone()));

		match commands {
			Commands::Show(cmd) => cmd.run(shovel),
			Commands::Remove(cmd) => cmd.run(shovel),
		}
	}
}

#[derive(Clone, clap::Subcommand)]
pub enum Commands {
	/// Show files in the cache
	Show(show::ShowCommand),

	/// Remove files from the cache
	#[clap(visible_alias("rm"))]
	Remove(remove::RemoveCommand),
}
