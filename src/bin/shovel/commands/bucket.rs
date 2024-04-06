mod add;
mod known;
mod list;
mod remove;
mod verify;

use crate::run::Run;

#[derive(clap::Subcommand)]
pub enum BucketCommands {
	/// Add a bucket
	Add(add::AddCommand),

	/// Remove a bucket
	#[clap(visible_alias("rm"))]
	Remove(remove::RemoveCommand),

	/// List all buckets
	List(list::ListCommand),

	/// List all known buckets
	Known(known::KnownCommand),

	/// Verify apps in a bucket
	Verify(verify::VerifyCommand),
}

impl Run for BucketCommands {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		match self {
			Self::Add(cmd) => cmd.run(shovel),
			Self::Remove(cmd) => cmd.run(shovel),
			Self::List(cmd) => cmd.run(shovel),
			Self::Known(cmd) => cmd.run(shovel),
			Self::Verify(cmd) => cmd.run(shovel),
		}
	}
}
