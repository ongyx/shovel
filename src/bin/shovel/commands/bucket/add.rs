use std::sync;

use clap;
use eyre;
use eyre::WrapErr;
use git2;
use git2::build;
use linya;
use owo_colors::OwoColorize;
use shovel;

use crate::commands::bucket::known;
use crate::run::Run;

/// A progress tracker for clone operations in Git.
struct CloneTracker {
    progress: sync::Mutex<linya::Progress>,
    // These bars are initialized on first use.
    recv_bar: Option<linya::Bar>,
    indx_bar: Option<linya::Bar>,
    cout_bar: Option<linya::Bar>,
}

impl CloneTracker {
    /// Returns a new clone tracker.
    pub fn new() -> Self {
        Self {
            progress: sync::Mutex::new(linya::Progress::new()),
            recv_bar: None,
            indx_bar: None,
            cout_bar: None,
        }
    }

    /// Returns a RepoBuilder that shows progress bars.
    pub fn builder<'p>(&'p mut self) -> build::RepoBuilder<'p> {
        let mut callbacks = git2::RemoteCallbacks::new();

        // Register a callback for receiving remote objects.
        callbacks.transfer_progress(|stats| {
            let recv_current = stats.received_objects();
            let indx_current = stats.indexed_objects();
            let total = stats.total_objects();

            if self.recv_bar.is_none() {
                self.recv_bar = Some(
                    self.progress
                        .lock()
                        .unwrap()
                        .bar(total, "Receiving objects"),
                );
            }

            self.progress
                .lock()
                .unwrap()
                .set_and_draw(self.recv_bar.as_ref().unwrap(), recv_current);

            if self.indx_bar.is_none() {
                self.indx_bar = Some(self.progress.lock().unwrap().bar(total, "Indexing objects"));
            }

            self.progress
                .lock()
                .unwrap()
                .set_and_draw(self.indx_bar.as_ref().unwrap(), indx_current);

            true
        });

        let mut options = git2::FetchOptions::new();
        options.remote_callbacks(callbacks);

        let mut checkout = build::CheckoutBuilder::new();

        // Register a callback for checking out objects.
        checkout.progress(|_, current, total| {
            if self.cout_bar.is_none() {
                self.cout_bar = Some(
                    self.progress
                        .lock()
                        .unwrap()
                        .bar(total, "Checking out objects"),
                );
            }

            self.progress
                .lock()
                .unwrap()
                .set_and_draw(self.cout_bar.as_ref().unwrap(), current);
        });

        let mut builder = build::RepoBuilder::new();
        builder.fetch_options(options).with_checkout(checkout);

        builder
    }
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
            .as_ref()
            .map(|u| u.as_str())
            .or_else(|| known::bucket(&self.name));

        let mut tracker = CloneTracker::new();

        match url {
            Some(url) => {
                let mut builder = tracker.builder();

                shovel
                    .buckets
                    .add(&self.name, url, Some(&mut builder))
                    .wrap_err_with(|| format!("Failed to add bucket {}", self.name))?;

                println!("Added bucket {} from {}", self.name.bold(), url.green());

                Ok(())
            }
            None => eyre::bail!("URL was not specified"),
        }
    }
}
