/// A runnable subcommand.
pub trait Run {
	/// Runs the subcommand using the given `shovel`.
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()>;
}
