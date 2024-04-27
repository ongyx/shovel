use crate::run::Run;
use crate::tracker::Tracker;
use crate::util;

#[derive(tabled::Tabled, Debug)]
#[tabled(rename_all = "pascal")]
struct UpdateInfo {
	hash: String,
	summary: String,
	time: String,
}

impl UpdateInfo {
	fn new(commit: &git2::Commit) -> Self {
		// Take the first 9 characters from the commit ID.
		let hash: String = commit.id().to_string().chars().take(9).collect();

		let summary = commit.summary().unwrap().to_owned();

		let time = shovel::Timestamp::from(commit.time()).to_string();

		Self {
			hash,
			summary,
			time,
		}
	}
}

#[derive(clap::Args)]
pub struct UpdateCommand {}

impl Run for UpdateCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		// Prepare a new Git progress tracker.
		let multi_progress = indicatif::MultiProgress::new();
		let tracker = Tracker::new(multi_progress);

		let mut opts = shovel::UpdateOptions::new();

		// Map the fetch options and checkout builder to the tracker.
		opts.fetch_options(|bucket| tracker.fetch_options(bucket.name()));
		opts.checkout_builder(|bucket| tracker.checkout_builder(bucket.name()));

		// Update all buckets.
		let updates = shovel.update(&opts)?;

		for (bucket, head) in updates {
			println!();

			// Get the commits between the previous and current HEAD.
			let mut infos = bucket
				.commits(head)?
				.map(|commit| UpdateInfo::new(&commit))
				.peekable();

			// If there are any commits, show them.
			if infos.peek().is_some() {
				println!(
					"{} has been updated:\n{}",
					bucket.name(),
					util::tableify(infos, false)
				);
			} else {
				println!("{} is already up-to-date.", bucket.name());
			}
		}

		println!();

		Ok(())
	}
}
