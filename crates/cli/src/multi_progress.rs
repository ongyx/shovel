use std::collections::HashMap;
use std::sync::atomic::AtomicUsize;
use std::sync::atomic::Ordering;
use std::sync::mpsc;

use linya::Progress;

enum Update {
	New(usize, String),
	Set(usize),
}

/// An identifier for a progress bar.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
pub struct Id(usize);

impl Id {
	/// Returns a new identifier, which is guaranteed to be unique.
	pub fn new() -> Self {
		static ID: AtomicUsize = AtomicUsize::new(0);

		let id = ID.fetch_add(1, Ordering::Relaxed);

		Self(id)
	}
}

/// A progress sender.
#[derive(Clone)]
pub struct Sender {
	inner: mpsc::Sender<(Id, Update)>,
}

impl Sender {
	/// Creates a new progress bar and returns its ID.
	///
	/// # Arguments
	///
	/// * `total` - The total amount of progress to be made.
	/// * `label` - The label to show for the progress bar.
	pub fn bar<S: Into<String>>(&self, total: usize, label: S) -> Option<Id> {
		let id = Id::new();
		let update = Update::New(total, label.into());

		self.send(id, update)
	}

	/// Sets the value of a progress bar.
	///
	/// # Arguments
	///
	/// * `id` - The progress bar's ID.
	/// * `value` - The value to set.
	pub fn set(&self, id: Id, value: usize) -> Option<Id> {
		self.send(id, Update::Set(value))
	}

	fn send(&self, id: Id, update: Update) -> Option<Id> {
		self.inner.send((id, update)).ok().map(|()| id)
	}
}

/// A progress receiver.
pub struct Receiver {
	inner: mpsc::Receiver<(Id, Update)>,
}

impl Receiver {
	/// Displays all progress bars received from senders.
	/// This blocks the current thread until all senders are dropped.
	pub fn show(&self) {
		let mut bars = HashMap::new();
		let mut progress = Progress::new();

		for (id, update) in &self.inner {
			let bar = bars.get(&id);

			match update {
				Update::New(total, label) => {
					bars.insert(id, progress.bar(total, label));
				}
				Update::Set(value) => {
					progress.set_and_draw(bar.unwrap(), value);
				}
			}
		}
	}
}

/// Creates and returns a progress sender and receiver.
/// The sender may be cloned to allow multiple threads to send their progress.
pub fn channel() -> (Sender, Receiver) {
	let (tx, rx) = mpsc::channel();

	(Sender { inner: tx }, Receiver { inner: rx })
}
