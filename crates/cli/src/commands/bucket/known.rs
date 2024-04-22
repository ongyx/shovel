use crate::run::Run;
use crate::util;

include!(concat!(env!("OUT_DIR"), "/buckets.rs"));

/// Returns the URL of the known bucket by name.
pub fn bucket(name: &str) -> Option<&'static str> {
	KNOWN_BUCKETS.get(name).copied()
}

#[derive(tabled::Tabled)]
#[tabled(rename_all = "pascal")]
struct KnownInfo {
	name: &'static str,
	source: &'static str,
}

#[derive(clap::Args)]
pub struct KnownCommand {}

impl Run for KnownCommand {
	fn run(&self, _shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let known = KNOWN_BUCKETS
			.into_iter()
			.map(|(name, source)| KnownInfo { name, source });

		println!("\n{}\n", util::tableify(known, false));

		Ok(())
	}
}
