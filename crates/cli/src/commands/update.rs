use eyre::WrapErr;
use rayon::prelude::*;

use crate::run::Run;
use crate::tracker;
use crate::util;

fn update_bucket(
	bucket: &mut shovel::Bucket,
	tracker: &tracker::Tracker,
) -> eyre::Result<git2::Oid> {
	let mut fo = tracker.fetch_options();
	let mut cb = tracker.checkout_builder();
	// According to the git2 pull example, not including this option causes the working directory to not update.
	cb.force();

	// Save the original HEAD commit before pulling.
	let head = bucket.commit()?.id();

	bucket
		.pull(Some(&mut fo), Some(&mut cb))
		.wrap_err_with(|| format!("Failed to update bucket {}", bucket.name()))?;

	Ok(head)
}

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
		let buckets = shovel.buckets.iter()?;

		let mut results: Option<Vec<_>> = None;

		rayon::scope(|scope| {
			let multi_progress = indicatif::MultiProgress::new();

			scope.spawn(|_| {
				// Take ownership of `multi_progress` to let it be used in the closure below.
				// https://docs.rs/rayon/latest/rayon/fn.scope.html
				let multi_progress = multi_progress;

				let updated = buckets.par_bridge().map(
					|bucket| -> eyre::Result<(git2::Oid, shovel::Bucket)> {
						let mut bucket = bucket?;

						let tracker =
							tracker::Tracker::new(multi_progress.clone(), bucket.name().as_str());

						// Update the bucket and get the original HEAD.
						let head = update_bucket(&mut bucket, &tracker)?;

						Ok((head, bucket))
					},
				);

				results = Some(updated.collect());
			});
		});

		// SAFETY: results will not be None after the scope closes.
		for result in results.unwrap() {
			println!();

			let (head, bucket) = result?;

			// Get the commits between the previous and current HEAD.
			let mut updates = bucket
				.commits(head)?
				.map(|commit| UpdateInfo::new(&commit))
				.peekable();

			// If there are any commits, show them.
			if updates.peek().is_some() {
				println!(
					"{} has been updated:\n{}",
					bucket.name(),
					util::tableify(updates, false)
				);
			} else {
				println!("{} is already up-to-date.", bucket.name());
			}
		}

		println!();

		Ok(())
	}
}
