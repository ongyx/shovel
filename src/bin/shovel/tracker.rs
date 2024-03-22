use std::collections::HashMap;

use std::sync::mpsc;

use git2;
use git2::build;
use linya;

use UpdateKind::*;

/// The kind of progress update being sent.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum UpdateKind {
    Receive,
    Index,
    Delta,
    Checkout,
    Close,
}

impl UpdateKind {
    /// Returns a description of the progress kind.
    fn describe(&self) -> &'static str {
        match self {
            Self::Receive => "Receiving objects",
            Self::Index => "Indexing objects",
            Self::Delta => "Resolving deltas",
            Self::Checkout => "Checking out files",
            Self::Close => "",
        }
    }
}

/// A progress update for a Git operation.
pub struct Update {
    /// The kind of update.
    pub kind: UpdateKind,

    /// The current progress.
    pub current: usize,

    /// The total progress.
    pub total: usize,
}

/// An update sender for Git operations.
/// Various callbacks are provided for hooking into git2.
#[derive(Clone)]
pub struct Sender(mpsc::Sender<Update>);

impl Sender {
    /// Send a close message to the receiver.
    pub fn close(&self) {
        self.0
            .send(Update {
                kind: Close,
                current: 0,
                total: 0,
            })
            .unwrap();
    }

    /// Returns a set of remote callbacks that send updates when invoked.
    pub fn remote_callbacks<'rc>(&'rc self) -> git2::RemoteCallbacks<'rc> {
        let mut callbacks = git2::RemoteCallbacks::new();
        let sender = self.0.clone();

        // Register a callback for receiving remote objects.
        callbacks.transfer_progress(move |stats| {
            let total = stats.total_objects();

            let received = stats.received_objects();
            if received > 0 {
                sender
                    .send(Update {
                        kind: Receive,
                        current: received,
                        total,
                    })
                    .unwrap();
            }

            let indexed = stats.indexed_objects();
            if indexed > 0 {
                sender
                    .send(Update {
                        kind: Index,
                        current: indexed,
                        total,
                    })
                    .unwrap();
            }

            let deltas = stats.indexed_deltas();
            let total_deltas = stats.total_deltas();
            if deltas > 0 {
                sender
                    .send(Update {
                        kind: Delta,
                        current: deltas,
                        total: total_deltas,
                    })
                    .unwrap();
            }

            true
        });

        callbacks
    }

    /// Returns a checkout builder with a callback that sends updates when invoked.
    pub fn checkout_builder<'cb>(&'cb self) -> build::CheckoutBuilder<'cb> {
        let mut checkout = build::CheckoutBuilder::new();
        let sender = self.0.clone();

        // Register a callback for checking out objects.
        checkout.progress(move |_, current, total| {
            sender
                .send(Update {
                    kind: Checkout,
                    current,
                    total,
                })
                .unwrap();
        });

        checkout
    }

    /// Returns a repository builder with both remote and checkout callbacks.
    pub fn repo_builder<'rb>(&'rb self) -> build::RepoBuilder<'rb> {
        let mut options = git2::FetchOptions::new();
        options.remote_callbacks(self.remote_callbacks());

        let mut builder = build::RepoBuilder::new();
        builder
            .fetch_options(options)
            .with_checkout(self.checkout_builder());

        builder
    }
}

/// An update receiver for Git operations.
pub struct Receiver(mpsc::Receiver<Update>);

impl Receiver {
    /// Returns an iterator over updates.
    pub fn iter(&self) -> mpsc::Iter<Update> {
        self.0.iter()
    }
}

impl IntoIterator for Receiver {
    type Item = Update;
    type IntoIter = mpsc::IntoIter<Update>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'r> IntoIterator for &'r Receiver {
    type Item = Update;
    type IntoIter = mpsc::Iter<'r, Update>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

/// A tracker for Git operations.
pub struct Tracker {
    tx: Sender,
    rx: Receiver,
}

impl Tracker {
    /// Returns a new tracker.
    pub fn new() -> Self {
        let (tx, rx) = mpsc::channel();

        Self {
            tx: Sender(tx),
            rx: Receiver(rx),
        }
    }

    /// Returns the sender and receiver halves of the tracker.
    pub fn channel(&self) -> (Sender, &Receiver) {
        (self.tx.clone(), &self.rx)
    }

    /// Listens for updates and presents a progress bar for each kind.
    pub fn show_progress(&self) {
        let mut progress = linya::Progress::new();
        let mut bars = HashMap::new();

        for update in &self.rx {
            if let Close = update.kind {
                break;
            }

            // Check if a progress bar exists for this update, and create one otherwise.
            let bar = bars
                .entry(update.kind)
                .or_insert_with(|| progress.bar(update.total, update.kind.describe()));

            progress.set_and_draw(bar, update.current);
        }
    }
}
