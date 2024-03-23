use std::thread;

use clap;
use eyre::WrapErr;
use shovel;
use tabled;

use crate::run::Run;
use crate::tracker;
use crate::util;

fn update_bucket(bucket: &mut shovel::Bucket, name: &str) -> shovel::Result<()> {
    let (sender, receiver) = tracker::channel();

    thread::scope(|scope| {
        let handle = scope.spawn(move || {
            let mut fo = sender.fetch_options();
            let mut cb = sender.checkout_builder();
            // According to the git2 pull example, not including this option causes the working directory to not update.
            cb.force();

            bucket.pull(Some(&mut fo), Some(&mut cb))?;

            sender.close();

            Ok(())
        });

        receiver.show_progress(Some(name));

        handle.join().unwrap()
    })
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
        for bucket in shovel.buckets.iter()? {
            let mut bucket = bucket?;
            let name = bucket.name();

            // Save the original HEAD commit before pulling.
            let old_head = bucket.commit()?.id();

            // Pull changes into the bucket.
            update_bucket(&mut bucket, &name)
                .wrap_err_with(|| format!("Failed to update bucket {}", name))?;

            let mut commits = bucket
                .commits(old_head)?
                .map(|commit| UpdateInfo::new(&commit))
                .peekable();

            if commits.peek().is_some() {
                println!("{}:\n{}\n", name, util::tableify(commits, false));
            }
        }

        Ok(())
    }
}
