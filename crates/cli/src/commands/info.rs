use std::iter;

use shovel::app::Error as AppError;
use shovel::bucket::Name;

use crate::run::Run;
use crate::util;

#[derive(tabled::Tabled, Debug)]
#[tabled(rename_all = "pascal")]
struct Info {
	name: String,
	description: String,
	version: String,
	bucket: String,
	website: String,
	license: String,
	#[tabled(rename = "Updated at")]
	updated_at: String,
	#[tabled(rename = "Updated by")]
	updated_by: String,
	installed: String,
	binaries: String,
	shortcuts: String,
}

impl Info {
	fn new(shovel: &mut shovel::Shovel, name: &Name) -> shovel::Result<Self> {
		let (bucket, item) = shovel.buckets.manifest(name)?;
		let manifest = item.manifest?;
		let arch = manifest.compatible();

		let license = manifest.license.to_string();

		let commit = bucket.manifest_commit(name.manifest())?;

		let (updated_at, updated_by) = match commit {
			Some(commit) => {
				let updated_at = shovel::Timestamp::from(commit.time()).to_string();
				let updated_by = commit.author().name().unwrap().to_owned();

				(updated_at, updated_by)
			}
			None => (
				"(commit not found)".to_owned(),
				"(author not found)".to_owned(),
			),
		};

		let app = shovel.apps.open_current(name.manifest());

		let installed = match app {
			Ok(app) => Ok(app.manifest()?.version),
			// If the app is not found, do not propagate the error.
			Err(AppError::NotFound { .. }) => Ok("(not installed)".to_owned()),
			Err(err) => Err(err),
		}?;

		let binaries = manifest
			.bin(arch)
			.map(ToString::to_string)
			.unwrap_or_default();

		let shortcuts = manifest
			.shortcuts(arch)
			.map(|shortcuts| {
				let shortcuts: Vec<_> = shortcuts.iter().map(ToString::to_string).collect();

				shortcuts.join(" | ")
			})
			.unwrap_or_default();

		Ok(Self {
			name: name.full().to_owned(),
			description: manifest.description.unwrap_or_default(),
			version: manifest.version,
			bucket: bucket.name(),
			website: manifest.homepage,
			license,
			updated_at,
			updated_by,
			installed,
			binaries,
			shortcuts,
		})
	}
}

#[derive(clap::Args)]
pub struct InfoCommand {
	/// The app to show info for
	pub app: String,
}

impl Run for InfoCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let name = Name::new(self.app.clone());
		let info = Info::new(shovel, &name)?;

		let table = util::tableify(iter::once(info), true);

		println!("\n{table}\n");

		Ok(())
	}
}
