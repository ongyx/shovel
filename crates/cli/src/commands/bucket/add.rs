use eyre::WrapErr;
use owo_colors::OwoColorize;

use crate::commands::bucket::known;
use crate::run::Run;
use crate::tracker::Tracker;

fn add_bucket(shovel: &mut shovel::Shovel, name: &str, url: &str) -> shovel::Result<()> {
	let multi_progress = indicatif::MultiProgress::new();

	let tracker = Tracker::new(multi_progress);
	let mut builder = tracker.repo_builder(name.to_owned());

	// Add the bucket.
	shovel.buckets.add(name, url, Some(&mut builder))?;

	Ok(())
}

#[derive(clap::Args)]
pub struct AddCommand {
	/// The bucket name.
	name: String,

	/// The bucket URL.
	/// Required if the bucket name is not known - run `shovel bucket known` for details.
	url: Option<String>,
}

impl Run for AddCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let url = self
			.url
			.as_deref()
			// Attempt to get the bucket URL if it's known.
			.or_else(|| known::bucket(&self.name))
			.ok_or_else(|| eyre::eyre!("URL was not specified, or bucket name is unknown"))?;

		// Add the bucket.
		add_bucket(shovel, &self.name, url)
			.wrap_err_with(|| format!("Failed to add bucket {}", self.name))?;

		println!("Added bucket {} from {}", self.name.bold(), url.green());

		Ok(())
	}
}
