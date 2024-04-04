use std::thread;

use clap;
use eyre;
use eyre::WrapErr;
use owo_colors::OwoColorize;
use shovel;

use crate::commands::bucket::known;
use crate::multi_progress;
use crate::run::Run;
use crate::tracker::Tracker;

fn add_bucket(shovel: &mut shovel::Shovel, name: &str, url: &str) -> shovel::Result<()> {
	thread::scope(|scope| {
		let (tx, rx) = multi_progress::channel();

		// Since updates on progress are sent over a channel, we run the bucket operation in a background thread.
		let handle = scope.spawn(|| {
			let tracker = Tracker::new(tx, name);
			let mut builder = tracker.repo_builder();

			// Add the bucket.
			shovel.buckets.add(name, url, Some(&mut builder))?;

			Ok(())
		});

		// Show the progress.
		rx.show();

		// Wait for the background thread to join and return the error, if any.
		handle.join().unwrap()
	})
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
