use eyre::WrapErr;
use shovel::app;

use crate::run::Run;
use crate::util;

#[derive(tabled::Tabled, Default)]
#[tabled(rename_all = "pascal")]
struct ListInfo {
	name: String,
	version: String,
	bucket: String,
	updated: String,
	info: String,
}

impl ListInfo {
	fn new(name: &str, app: app::Result<shovel::App>) -> Self {
		// Obtain the app's info if it didn't error out.
		let info = app.and_then(|app| {
			let manifest = app.manifest()?;
			let metadata = app.metadata()?;

			let version = manifest.version;
			let bucket = metadata.bucket;
			let updated = app.timestamp()?.to_string();

			Ok(Self {
				name: name.to_owned(),
				version,
				bucket,
				updated,
				..Default::default()
			})
		});

		match info {
			Ok(info) => info,
			// Wrap the error infomation.
			Err(err) => Self {
				name: name.to_owned(),
				info: err.to_string(),
				..Default::default()
			},
		}
	}
}

#[derive(clap::Args)]
pub struct ListCommand {
	/// The apps to list as a regex. To specify a bucket, use the syntax `bucket/pattern`.
	query: Option<String>,
}

impl Run for ListCommand {
	fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
		let query = match &self.query {
			Some(q) => q,
			None => "",
		};

		let (bucket, app) = util::parse_app(query);

		let regex = regex::Regex::new(app).wrap_err("Invalid pattern")?;

		let apps: Vec<_> = shovel
			.apps
			.each()?
			.map(|(name, app)| ListInfo::new(&name, app))
			.filter_map(|info| {
				// check the bucket and name.
				if (bucket.is_empty() || info.bucket == bucket)
					&& (app.is_empty() || regex.is_match(&info.name))
				{
					Some(info)
				} else {
					None
				}
			})
			.collect();

		match apps.len() {
			0 => eyre::bail!("No app(s) found."),
			_ => {
				println!("\n{}\n", util::tableify(apps, false));

				Ok(())
			}
		}
	}
}
