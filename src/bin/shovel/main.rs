mod bucket;
mod global;
mod run;

use std::{fs, io};

use anyhow::Context;
use clap;
use clap::Parser;
use shovel::{Config, Shovel};

use global::GlobalCommands;
use run::Run;

#[derive(clap::Parser)]
#[command(version, about)]
#[command(propagate_version = true)]
struct Args {
    #[command(subcommand)]
    commands: GlobalCommands,

    /// Specify a configuration file
    #[arg(short, long, global = true)]
    config: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    let config: Config = match args.config {
        Some(config_path) => {
            // Read the config file.
            let file = fs::File::open(&config_path)
                .with_context(|| format!("Failed to open config file {}", config_path))?;

            let reader = io::BufReader::new(file);

            serde_json::from_reader(reader)
                .with_context(|| format!("Failed to parse config file {}", config_path))?
        }
        None => Default::default(),
    };

    let mut shovel = Shovel::new(config)?;

    // Delegate to sub-commands.
    args.commands.run(&mut shovel)
}
