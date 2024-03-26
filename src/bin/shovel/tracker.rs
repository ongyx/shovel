use std::collections::HashMap;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

use git2::build::{CheckoutBuilder, RepoBuilder};
use git2::{FetchOptions, RemoteCallbacks};
use linya::{Bar, Progress};

/// A kind of Git operation in progress.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug)]
enum Operation {
    /// An object has been received.
    Receive,

    /// An object has been indexed.
    Index,

    /// A delta has been resolved.
    Delta,
}

impl Display for Operation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let desc = match self {
            Self::Receive => "Receiving objects",
            Self::Index => "Indexing objects",
            Self::Delta => "Resolving deltas",
        };

        write!(f, "{}", desc)
    }
}

/// A thread-safe progress coordinator.
#[derive(Clone)]
pub struct SharedProgress(pub Arc<Mutex<Progress>>);

impl SharedProgress {
    /// Returns a new progress coordinator.
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(Progress::new())))
    }

    /// Convenience function for `Progress::bar`.
    pub fn bar<S: Into<String>>(&self, total: usize, label: S) -> Bar {
        self.0.lock().unwrap().bar(total, label)
    }

    /// Convenience function for `Progress::set_and_draw`.
    pub fn set_and_draw(&self, bar: &Bar, value: usize) {
        self.0.lock().unwrap().set_and_draw(bar, value);
    }
}

/// A map of Git operations to their progress bars.
struct Bars {
    repo: String,
    progress: SharedProgress,
    bars: HashMap<Operation, Bar>,
}

impl Bars {
    /// Returns a new bar map.
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository name for display purposes.
    /// * `progress` - The progress coordinator.
    pub fn new(repo: &str, progress: SharedProgress) -> Self {
        Self {
            repo: repo.to_owned(),
            progress,
            bars: HashMap::new(),
        }
    }

    /// Sets and draws a progress bar for a Git operation.
    pub fn set_and_draw(&mut self, op: Operation, current: usize, total: usize) {
        let bar = self
            .bars
            .entry(op)
            .or_insert_with(|| self.progress.bar(total, format!("{}: {}", &self.repo, op)));

        self.progress.set_and_draw(&bar, current);
    }
}

/// A progress tracker for Git operations.
pub struct Tracker {
    repo: String,
    progress: SharedProgress,
}

impl Tracker {
    /// Returns a new tracker.
    ///
    /// # Arguments
    ///
    /// * `repo` - The repository name for display purposes.
    /// * `progress` - The progress coordinator.
    pub fn new(repo: &str, progress: SharedProgress) -> Self {
        Self {
            repo: repo.to_owned(),
            progress,
        }
    }

    /// Returns a set of remote callbacks that send updates when invoked.
    pub fn remote_callbacks<'rc>(&'rc self) -> RemoteCallbacks<'rc> {
        use Operation::*;

        let mut callbacks = RemoteCallbacks::new();
        let mut bars = Bars::new(&self.repo, self.progress.clone());

        // Register a callback for receiving remote objects.
        callbacks.transfer_progress(move |stats| {
            let total = stats.total_objects();

            let received = stats.received_objects();
            if received > 0 {
                bars.set_and_draw(Receive, received, total);
            }

            let indexed = stats.indexed_objects();
            if indexed > 0 {
                bars.set_and_draw(Index, indexed, total);
            }

            let deltas = stats.indexed_deltas();
            let total_deltas = stats.total_deltas();
            if deltas > 0 {
                bars.set_and_draw(Delta, deltas, total_deltas);
            }

            true
        });

        callbacks
    }

    /// Returns a set of fetch options that wraps `Self::remote_callbacks`.
    pub fn fetch_options<'fo>(&'fo self) -> FetchOptions<'fo> {
        let mut fetch = FetchOptions::new();

        fetch.remote_callbacks(self.remote_callbacks());

        fetch
    }

    /// Returns a checkout builder with a callback that sends updates when invoked.
    pub fn checkout_builder<'cb>(&'cb self) -> CheckoutBuilder<'cb> {
        let mut checkout = CheckoutBuilder::new();
        let mut bar = None;
        let progress = self.progress.clone();

        // Register a callback for checking out objects.
        checkout.progress(move |_, current, total| {
            if total > 0 {
                if bar.is_none() {
                    bar = Some(progress.bar(total, format!("{}: Checking out files", &self.repo)))
                }

                // SAFETY: The bar is initialised from `if bar.is_none() { ... }`.
                progress.set_and_draw(bar.as_ref().unwrap(), current);
            }
        });

        checkout
    }

    /// Returns a repository builder with both remote and checkout callbacks.
    pub fn repo_builder<'rb>(&'rb self) -> RepoBuilder<'rb> {
        let mut builder = RepoBuilder::new();
        builder
            .fetch_options(self.fetch_options())
            .with_checkout(self.checkout_builder());

        builder
    }
}
