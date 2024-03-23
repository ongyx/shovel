use std::thread;

use clap;
use eyre;
use eyre::WrapErr;
use owo_colors::OwoColorize;
use shovel;

use crate::commands::bucket::known;
use crate::run::Run;
use crate::tracker;

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

        let (sender, receiver) = tracker::channel();

        let result = thread::scope(|scope| {
            // Since updates on progress are sent over a channel, we run the bucket operation in a background thread.
            let handle = scope.spawn(move || -> shovel::Result<()> {
                let mut builder = sender.repo_builder();

                // Add the bucket.
                shovel.buckets.add(&self.name, url, Some(&mut builder))?;

                // Signal to the tracker that the operation is done.
                sender.close();

                Ok(())
            });

            // Show a progress bar on the main thread until the sender is closed.
            receiver.show_progress(Some(&self.name));

            // Wait for the background thread to join and return the error, if any.
            handle.join().unwrap()
        });

        // Wrap any errors with a more informative message.
        result.wrap_err_with(|| format!("Failed to add bucket {}", self.name))?;

        println!("Added bucket {} from {}", self.name.bold(), url.green());

        Ok(())
    }
}
