use eyre::WrapErr;

use shovel::bucket::Bucket;
use shovel::bucket::Name;
use shovel::bucket::Predicate;
use shovel::bucket::SearchItem;

use crate::run::Run;
use crate::util;

#[derive(tabled::Tabled)]
#[tabled(rename_all = "pascal")]
struct SearchInfo {
	name: String,
	version: String,
	bucket: String,
	binaries: String,
}

impl SearchInfo {
	fn new(bucket: &Bucket, item: SearchItem) -> shovel::Result<Self> {
		let manifest = item.manifest?;
		let arch = manifest.compatible();

		let version = manifest.version.clone();
		let binaries = manifest
			.bin(arch)
			.map(ToString::to_string)
			.unwrap_or_default();

		Ok(SearchInfo {
			name: item.name,
			version,
			bucket: bucket.name(),
			binaries,
		})
	}
}

#[derive(clap::Args)]
pub struct SearchCommand {
	/// The apps to search as a regex. To specify a bucket, use the syntax `bucket/pattern`.
	query: String,
}

impl Run for SearchCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let name = Name::new(self.query.clone());

		let regex = regex::Regex::new(name.manifest()).wrap_err("Invalid pattern")?;

		let predicate = Predicate::new(
			|b| name.bucket().map_or(true, |nb| b.name() == nb),
			|m| regex.is_match(m),
		);

		let apps: shovel::Result<Vec<_>> = shovel
			.buckets
			.search_all(&predicate)
			.wrap_err("Search failed")?
			.map(|(bucket, item)| SearchInfo::new(&bucket, item))
			.collect();

		let apps = apps?;

		if apps.is_empty() {
			eyre::bail!("No app(s) found.");
		}

		println!("\n{}\n", util::tableify(apps, false));

		Ok(())
	}
}
