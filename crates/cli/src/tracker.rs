use git2::build::CheckoutBuilder;
use git2::build::RepoBuilder;
use git2::FetchOptions;
use git2::RemoteCallbacks;

use crate::multi_progress;

#[derive(Clone)]
pub struct Tracker {
	sender: multi_progress::Sender,
	repo: String,
}

impl Tracker {
	pub fn new(sender: multi_progress::Sender, repo: &str) -> Self {
		Self {
			sender,
			repo: repo.to_owned(),
		}
	}

	/// Returns a set of remote callbacks that send updates when invoked.
	pub fn remote_callbacks(&self) -> RemoteCallbacks<'_> {
		let mut callbacks = RemoteCallbacks::new();

		let mut receive_id = None;
		let mut index_id = None;
		let mut delta_id = None;

		// Register a callback for receiving remote objects.
		callbacks.transfer_progress(move |stats| {
			let total = stats.total_objects();

			let received = stats.received_objects();
			if received > 0 {
				if receive_id.is_none() {
					receive_id = self
						.sender
						.bar(total, format!("{}: Receiving objects", self.repo));
				}

				self.sender.set(receive_id.unwrap(), received);
			}

			let indexed = stats.indexed_objects();
			if indexed > 0 {
				if index_id.is_none() {
					index_id = self
						.sender
						.bar(total, format!("{}: Indexing objects", self.repo));
				}

				self.sender.set(index_id.unwrap(), indexed).unwrap();
			}

			let deltas = stats.indexed_deltas();
			let total_deltas = stats.total_deltas();
			if deltas > 0 {
				if delta_id.is_none() {
					delta_id = self
						.sender
						.bar(total_deltas, format!("{}: Resolving deltas", self.repo));
				}

				self.sender.set(delta_id.unwrap(), deltas).unwrap();
			}

			true
		});

		callbacks
	}

	/// Returns a set of fetch options that wraps `Self::remote_callbacks`.
	pub fn fetch_options(&self) -> FetchOptions<'_> {
		let mut fetch = FetchOptions::new();

		fetch.remote_callbacks(self.remote_callbacks());

		fetch
	}

	/// Returns a checkout builder with a callback that sends updates when invoked.
	pub fn checkout_builder(&self) -> CheckoutBuilder<'_> {
		let mut checkout = CheckoutBuilder::new();

		let mut id = None;

		// Register a callback for checking out objects.
		checkout.progress(move |_, current, total| {
			if current > 0 {
				if id.is_none() {
					id = self
						.sender
						.bar(total, format!("{}: Checking out files", self.repo));
				}

				self.sender.set(id.unwrap(), current);
			}
		});

		checkout
	}

	/// Returns a repository builder with both remote and checkout callbacks.
	pub fn repo_builder(&self) -> RepoBuilder<'_> {
		let mut builder = RepoBuilder::new();
		builder
			.fetch_options(self.fetch_options())
			.with_checkout(self.checkout_builder());

		builder
	}
}
