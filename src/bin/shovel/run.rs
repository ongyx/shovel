use anyhow;

use shovel::Shovel;

/// A runnable subcommand.
pub trait Run {
    /// Runs the subcommand using the given `shovel`.
    fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()>;
}
