use std::collections::HashSet;
use std::fs;

use bytesize;
use clap;
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
		// Get a hashset of the apps.
		// SAFETY: apps is always at least one element long.
		let apps: HashSet<String> = if self.apps[0] == "*" {
			// An empty hashset means all apps should be removed.
			HashSet::new()
		} else {
			self.apps.iter().cloned().collect()
		};

		// Filter out the keys to be removed.
		let keys = shovel
			.cache
			.iter()?
			.filter(|key| apps.is_empty() || apps.get(&key.name).is_some());

		let mut count = 0usize;
		let mut size = bytesize::ByteSize(0);

		for key in keys {
			let len = fs::metadata(shovel.cache.path(&key))?.len();

			shovel.cache.remove(&key.name)?;

			count += 1;
			size += len;
		}

		println!(
			"{}",
			format!(
				"Deleted: {} {}, {}",
				count,
				if count == 1 { "file" } else { "files" },
				size.to_string_as(true)
			)
			.bright_yellow()
		);

		Ok(())
	}
}
