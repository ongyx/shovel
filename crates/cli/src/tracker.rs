use std::cell::OnceCell;
use std::sync::OnceLock;

use git2::build::CheckoutBuilder;
use git2::build::RepoBuilder;
use git2::FetchOptions;
use git2::RemoteCallbacks;

use indicatif::MultiProgress;
use indicatif::ProgressBar;
use indicatif::ProgressStyle;

fn progress_style() -> &'static ProgressStyle {
	static PROGRESS_STYLE: OnceLock<ProgressStyle> = OnceLock::new();

	PROGRESS_STYLE
		.get_or_init(|| ProgressStyle::with_template("{msg} {pos}/{len} {wide_bar}").unwrap())
}

#[derive(Clone)]
pub struct Tracker {
	multi_progress: MultiProgress,
}

impl Tracker {
	pub fn new(multi_progress: MultiProgress) -> Self {
		Self { multi_progress }
	}

	/// Returns a set of remote callbacks that send updates when invoked.
	pub fn remote_callbacks(&self, repo: String) -> RemoteCallbacks<'_> {
		let mut callbacks = RemoteCallbacks::new();

		let recv_bar = OnceCell::new();
		let index_bar = OnceCell::new();
		let delta_bar = OnceCell::new();

		// Register a callback for receiving remote objects.
		callbacks.transfer_progress(move |stats| {
			let total = stats.total_objects() as u64;

			let received = stats.received_objects() as u64;
			if received > 0 {
				let recv_bar = recv_bar.get_or_init(|| {
					let bar = self.add_progress_bar(total);

					bar.set_message(format!("{repo}: Receiving objects"));
					bar
				});

				recv_bar.set_position(received);
			}

			let indexed = stats.indexed_objects() as u64;
			if indexed > 0 {
				let index_bar = index_bar.get_or_init(|| {
					let bar = self.add_progress_bar(total);

					bar.set_message(format!("{repo}: Indexing objects"));
					bar
				});

				index_bar.set_position(indexed);
			}

			let deltas = stats.indexed_deltas() as u64;
			let total_deltas = stats.total_deltas() as u64;
			if deltas > 0 {
				let delta_bar = delta_bar.get_or_init(|| {
					let bar = self.add_progress_bar(total_deltas);

					bar.set_message(format!("{repo}: Resolving deltas"));
					bar
				});

				delta_bar.set_position(deltas);
			}

			true
		});

		callbacks
	}

	/// Returns a set of fetch options that wraps `Self::remote_callbacks`.
	pub fn fetch_options(&self, repo: String) -> FetchOptions<'_> {
		let mut fetch = FetchOptions::new();

		fetch.remote_callbacks(self.remote_callbacks(repo));

		fetch
	}

	/// Returns a checkout builder with a callback that sends updates when invoked.
	pub fn checkout_builder(&self, repo: String) -> CheckoutBuilder<'_> {
		let mut checkout = CheckoutBuilder::new();

		let bar = OnceCell::new();

		// Register a callback for checking out objects.
		checkout.progress(move |_, current, total| {
			if current > 0 {
				let bar = bar.get_or_init(|| {
					let bar = self.add_progress_bar(total as u64);

					bar.set_message(format!("{repo}: Checking out files"));
					bar
				});

				bar.set_position(current as u64);
			}
		});

		checkout
	}

	/// Returns a repository builder with both remote and checkout callbacks.
	pub fn repo_builder(&self, repo: String) -> RepoBuilder<'_> {
		let mut builder = RepoBuilder::new();
		builder
			.fetch_options(self.fetch_options(repo.clone()))
			.with_checkout(self.checkout_builder(repo));

		builder
	}

	fn add_progress_bar(&self, len: u64) -> ProgressBar {
		self.multi_progress
			.add(ProgressBar::new(len).with_style(progress_style().clone()))
	}
}
