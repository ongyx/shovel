use std::thread;

use clap;
use eyre::WrapErr;
use shovel;

use crate::run::Run;
use crate::tracker;

#[derive(clap::Args)]
pub struct UpdateCommand {}

impl Run for UpdateCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        for bucket in shovel.buckets.iter()? {
            let mut bucket = bucket?;
            let bucket_name = bucket.name();

            let (sender, receiver) = tracker::channel();

            let result = thread::scope(|scope| {
                let handle = scope.spawn(move || -> shovel::Result<()> {
                    let mut fo = sender.fetch_options();
                    let mut cb = sender.checkout_builder();

                    bucket.pull(Some(&mut fo), Some(&mut cb))?;

                    sender.close();

                    Ok(())
                });

                receiver.show_progress(Some(&bucket_name));

                handle.join().unwrap()
            });

            result.wrap_err_with(|| format!("Failed to update bucket {}", bucket_name))?;
        }

        Ok(())
    }
}
