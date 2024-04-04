use std::collections::HashSet;
use std::fs;
use std::io;

use bytesize;
use clap;
use eyre;
use owo_colors::OwoColorize;
use shovel;
use shovel::cache;
use tabled;

use crate::run::Run;
use crate::util;

#[derive(tabled::Tabled, Debug)]
#[tabled(rename_all = "pascal")]
pub struct ShowInfo {
	name: String,
	version: String,
	length: u64,
	#[tabled(rename = "URL")]
	url: String,
}

impl ShowInfo {
	fn new(shovel: &mut shovel::Shovel, key: cache::Key) -> io::Result<Self> {
		let path = shovel.cache.path(&key);
		let metadata = fs::metadata(path)?;

		Ok(Self {
			name: key.name,
			version: key.version,
			length: metadata.len(),
			url: key.url,
		})
	}
}

#[derive(Clone, clap::Args)]
pub struct ShowCommand {
	/// The apps to show in the cache
	apps: Vec<String>,
}

impl Run for ShowCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let apps: HashSet<String> = self.apps.iter().cloned().collect();

		let keys: Vec<_> = shovel
			.cache
			.iter()?
			// Filter apps not in the list.
			.filter(|key| apps.is_empty() || apps.get(&key.name).is_some())
			// Ignore cache infos with errors.
			.filter_map(|key| ShowInfo::new(shovel, key).ok())
			.collect();

		let count = keys.len();
		let size = bytesize::ByteSize(keys.iter().map(|info| info.length).sum());
		let table = util::tableify(keys, false);

		let title = format!(
			"Total: {} {}, {}",
			count,
			if count == 1 { "file" } else { "files" },
			size.to_string_as(true),
		);

		println!("\n{}\n{}\n", title.bright_yellow(), table);

		Ok(())
	}
}
