mod bucket;
mod cat;
mod info;
mod list;
mod search;
mod update;

use clap;
use eyre;
use shovel;

use crate::run::Run;

#[derive(clap::Subcommand)]
pub enum Commands {
    /// Manage buckets
    #[command(subcommand)]
    Bucket(bucket::BucketCommands),

    /// Show an app's manifest
    Cat(cat::CatCommand),

    /// Show an app's info
    Info(info::InfoCommand),

    /// List installed apps
    List(list::ListCommand),

    /// Search for an app
    Search(search::SearchCommand),

    /// Update all buckets
    Update(update::UpdateCommand),
}

impl Run for Commands {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        match self {
            Self::Bucket(cmds) => cmds.run(shovel),
            Self::Cat(cmd) => cmd.run(shovel),
            Self::Info(cmd) => cmd.run(shovel),
            Self::List(cmd) => cmd.run(shovel),
            Self::Search(cmd) => cmd.run(shovel),
            Self::Update(cmd) => cmd.run(shovel),
        }
    }
}
