use std::collections::HashSet;

use owo_colors::OwoColorize;

use crate::run::Run;

#[derive(Clone, clap::Args)]
pub struct RemoveCommand {
	/// The apps to remove in the cache
	#[arg(required = true)]
	apps: Vec<String>,
}

impl Run for RemoveCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let old_stat = shovel.cache.stat()?;

		// SAFETY: apps is required to have at least one element.
		if self.apps[0] == "*" {
			shovel.cache.remove_all()?;
		} else {
			// Get a hashset of the apps.
			let apps: HashSet<String> = self.apps.iter().cloned().collect();

			// Filter out the keys to be removed.
			let keys = shovel
				.cache
				.iter()?
				.filter(|key| apps.is_empty() || apps.get(&key.name).is_some());

			for key in keys {
				shovel.cache.remove(&key.name)?;
			}
		}

		let new_stat = shovel.cache.stat()?;

		let count = old_stat.count - new_stat.count;
		let length = bytesize::ByteSize(old_stat.length - new_stat.length);

		println!(
			"{}",
			format!(
				"Deleted: {} {}, {}",
				count,
				if count == 1 { "file" } else { "files" },
				length.to_string_as(true)
			)
			.bright_yellow()
		);

		Ok(())
	}
}
