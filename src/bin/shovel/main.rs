mod commands;
mod multi_progress;
mod run;
mod tracker;
mod util;

use std::fs;

use clap;
use clap::Parser;
use color_eyre;
use eyre;
use eyre::WrapErr;
use shovel;
use shovel::json;

use run::Run;

#[derive(clap::Parser)]
#[command(version, about)]
#[command(propagate_version = true)]
struct Args {
	#[command(subcommand)]
	commands: commands::Commands,

	/// Specify a configuration file
	#[arg(short, long, global = true)]
	config: Option<String>,
}

fn main() -> eyre::Result<()> {
	color_eyre::install()?;

	let args = Args::parse();

	let config: shovel::Config = match args.config {
		Some(config_path) => {
			let config_file = fs::File::open(&config_path)?;

			// Read the config file.
			json::from_reader(config_file)
				.wrap_err_with(|| format!("Failed to parse config file {}", config_path))?
		}
		None => Default::default(),
	};

	let mut shovel = shovel::Shovel::new(config)?;

	// Delegate to sub-commands.
	args.commands.run(&mut shovel)
}
