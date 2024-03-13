mod bucket;
mod global;
mod run;
mod util;

use anyhow::Context;
use clap;
use clap::Parser;
use shovel;

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

    let config: shovel::Config = match args.config {
        Some(config_path) => {
            // Read the config file.
            shovel::json_from_file(&config_path)
                .with_context(|| format!("Failed to parse config file {}", config_path))?
        }
        None => Default::default(),
    };

    let mut shovel = shovel::Shovel::new(config)?;

    // Delegate to sub-commands.
    args.commands.run(&mut shovel)
}
