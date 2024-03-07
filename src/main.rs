use std::{fs, io};

use anyhow::{anyhow, Context};
use clap;
use clap::Parser;
use regex;

use shovel::{Config, Shovel};

#[derive(clap::Parser)]
#[command(version, about)]
#[command(propagate_version = true)]
struct Args {
    #[command(subcommand)]
    command: Commands,

    /// Specify a configuration file
    #[arg(short, long, global = true)]
    config: Option<String>,
}

#[derive(clap::Subcommand)]
enum Commands {
    /// Search for an app
    Search(SearchCommand),
    /// Verify apps in a bucket
    Verify(VerifyCommand),
}

#[derive(clap::Args)]
struct SearchCommand {
    /// The search pattern as a regex.
    pattern: String,

    /// The bucket to search.
    #[arg(short, long)]
    bucket: Option<String>,
}

impl SearchCommand {
    pub fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        let regex = regex::Regex::new(&self.pattern).context("Invalid pattern")?;

        let mut manifests = shovel
            .search(|b, m| {
                // If bucket is not None, check the bucket name.
                if let Some(bk) = &self.bucket {
                    if b != bk {
                        return false;
                    }
                }

                regex.is_match(m)
            })
            .context("Search failed")?
            .peekable();

        if manifests.peek().is_none() {
            return Err(anyhow!("No app(s) found."));
        }

        for (bucket, manifest) in manifests {
            println!("{}/{}", bucket, manifest);
        }

        Ok(())
    }
}

#[derive(clap::Args)]
struct VerifyCommand {
    /// The bucket to verify apps for.
    bucket: String,
}

impl VerifyCommand {
    pub fn run(&self, shovel: &mut Shovel) -> anyhow::Result<()> {
        let bucket = shovel.get_bucket(&self.bucket)?;

        let mut count = 0;

        for manifest_name in bucket.manifests() {
            bucket.get_manifest(&manifest_name).with_context(|| {
                format!(
                    "Failed parsing manifest {}",
                    bucket
                        .get_manifest_path(&manifest_name)
                        .unwrap()
                        .to_string_lossy()
                )
            })?;

            count += 1;
        }

        println!("Ok: {} manifests parsed", count);

        Ok(())
    }
}

fn main() -> anyhow::Result<()> {
    use Commands::*;

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

    match &args.command {
        Search(cmd) => cmd.run(&mut shovel)?,
        Verify(cmd) => cmd.run(&mut shovel)?,
    }

    Ok(())
}
