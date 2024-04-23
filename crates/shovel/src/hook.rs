use std::fmt::Display;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use crate::config;
use crate::manifest::Arch;
use crate::util;
use crate::Config;
use crate::Manifest;

use powershell;

fn home_dir() -> &'static Path {
	static HOME_DIR: OnceLock<PathBuf> = OnceLock::new();

	HOME_DIR.get_or_init(|| home::home_dir().expect("home dir should exist"))
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
	#[error("Failed to run hook {script:?} - got {output:?}")]
	Failure {
		script: Script,
		output: powershell::Output,
	},

	#[error("IO error encountered when running hook: {0}")]
	Io(#[from] io::Error),

	/// A JSON error.
	#[error(transparent)]
	Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

/// The kind of hook being executed.
#[derive(Clone, Copy, Debug)]
pub enum Script {
	Install,
	Uninstall,
	PreInstall,
	PostInstall,
	PreUninstall,
	PostUninstall,
}

/// The command invoking the hook.
#[derive(Clone, Copy, Debug)]
pub enum Command {
	/// An app is being installed.
	Install,

	/// An app is being uninstalled.
	Uninstall,

	/// An app is being updated.
	Update,
}

impl Display for Command {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use Command::*;

		write!(
			f,
			"{}",
			match self {
				Install => "install",
				Uninstall => "uninstall",
				Update => "update",
			}
		)
	}
}

/// A hook context.
pub struct Context<'c> {
	/// The app's name.
	pub app: &'c str,

	/// The app's manifest.
	pub manifest: &'c Manifest,

	/// The current configuration.
	pub config: &'c Config,

	/// The architecture to install for the app.
	pub arch: Arch,

	/// The command in use.
	pub command: Command,
}

impl Context<'_> {
	fn runner(&self) -> Result<powershell::Runner> {
		let mut runner = powershell::Runner::default();

		self.non_path_vars(&mut runner)?;
		self.path_vars(&mut runner);
		Self::functions(&mut runner);

		Ok(runner)
	}

	fn non_path_vars(&self, runner: &mut powershell::Runner) -> Result<()> {
		use powershell::Expression;

		let app = self.app.to_owned();
		// TODO: Pick architecture?
		let arch = Arch::native().to_string();
		let command = self.command.to_string();
		// TODO: Read the actual scoop config?
		let config = Expression::Raw("[PSCustomObject]{}".to_owned());
		let global = self.config.is_global();
		let manifest = Expression::object(&self.manifest)?;
		let version = self.manifest.version.clone();

		runner.vars([
			// Non-paths.
			("app", app.into()),
			("architecture", arch.into()),
			("cmd", command.into()),
			("cfg", config),
			("global", global.into()),
			("manifest", manifest),
			("version", version.into()),
		]);

		Ok(())
	}

	fn path_vars(&self, runner: &mut powershell::Runner) {
		let dir = util::path_to_string(
			self.config
				.app_dir()
				.join(format!(r"{}\{}", self.app, self.manifest.version)),
		);
		let persist_dir = util::path_to_string(self.config.persist_dir().join(self.app));

		let buckets_dir = util::path_to_string(self.config.bucket_dir());
		let cache_dir = util::path_to_string(self.config.cache_dir());
		let cfg_path = util::path_to_string(home_dir().join(".scoop"));
		let global_dir = util::path_to_string(config::global_install_dir());
		let module_dir = util::path_to_string(self.config.module_dir());
		let original_dir = dir.clone();
		let old_scoop_dir = util::path_to_string(home_dir().join(r"AppData\Local\Scoop"));
		let scoop_dir = util::path_to_string(self.config.install_dir());

		runner.vars([
			// Important paths.
			("dir", dir.into()),
			("persist_dir", persist_dir.into()),
			// Other paths.
			("bucketsdir", buckets_dir.into()),
			("cachedir", cache_dir.into()),
			("cfgpath", cfg_path.into()),
			("globaldir", global_dir.into()),
			("modulesdir", module_dir.into()),
			("original_dir", original_dir.into()),
			("oldscoopdir", old_scoop_dir.into()),
			("scoopdir", scoop_dir.into()),
		]);
	}

	fn functions(runner: &mut powershell::Runner) {
		runner.prelude(
			r#"	
			function basedir($global) { if($global) { return $globaldir } $scoopdir }
			function appsdir($global) { "$(basedir $global)\apps" }
			function shimdir($global) { "$(basedir $global)\shims" }
			function appdir($app, $global) { "$(appsdir $global)\$app" }
			function versiondir($app, $version, $global) { "$(appdir $app $global)\$version" }
		"#,
		);
	}
}

/// A hook runner.
///
/// Hooks allow manifests to run arbitrary PowerShell scripts before, during, or after (un)installation.
/// See https://github.com/ScoopInstaller/Scoop/wiki/Pre-Post-(un)install-scripts for details.
pub struct Hook<'h> {
	context: Context<'h>,
	runner: powershell::Runner,
}

impl<'h> Hook<'h> {
	/// Creates a new hook runner.
	///
	/// # Arguments
	///
	/// * `context` - The hook context.
	///
	/// # Errors
	///
	/// If the manifest cannot be serialized as JSON to pass to PowerShell, [`Error::Json`] is returned.
	pub fn new(context: Context<'h>) -> Result<Self> {
		let powershell = context.runner()?;

		Ok(Self {
			context,
			runner: powershell,
		})
	}

	/// Runs a hook script and returns its output.
	/// If the hook script does not exist in the manifest or is empty, Ok(None) is returned, otherwise Ok(output).
	///
	/// # Arguments
	///
	/// * `script` - The hook script to run.
	///
	/// # Errors
	///
	/// If the hook failed to run, `Error::Failure` is returned.
	pub fn run(&self, script: Script) -> Result<Option<powershell::Output>> {
		use Script::*;

		let manifest = self.context.manifest;
		let arch = self.context.arch;

		let hook = match script {
			Install => manifest.installer_script(arch),
			Uninstall => manifest.uninstaller_script(arch),
			PreInstall => manifest.pre_install(arch),
			PostInstall => manifest.post_install(arch),
			PreUninstall => manifest.pre_uninstall(arch),
			PostUninstall => manifest.post_uninstall(arch),
		}
		.unwrap_or_default()
		.join("\r\n");

		if hook.is_empty() {
			Ok(None)
		} else {
			let output = self.runner.run(hook)?;

			if output.status.success() {
				Ok(Some(output))
			} else {
				Err(Error::Failure { script, output })
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use crate::list;
	use crate::manifest::Installer;
	use crate::manifest::ManifestArch;

	use super::*;

	#[test]
	fn hook() {
		let context = Context {
			app: "test",
			manifest: &Manifest {
				common: ManifestArch {
					pre_install: Some(list!["Write-Host 'Hello World!'".into()]),
					..Default::default()
				},
				..Default::default()
			},
			config: &Config::default(),
			arch: Arch::X86_64,
			command: Command::Install,
		};

		let hook = Hook::new(context).unwrap();
		let output = hook.run(Script::PreInstall).unwrap().unwrap();
		let lines: Vec<_> = output.stdout.lines().collect();

		assert_eq!(lines, ["Hello World!"]);
	}

	#[test]
	fn failure() {
		let context = Context {
			app: "test",
			manifest: &Manifest {
				common: ManifestArch {
					installer: Some(Installer {
						script: Some(list!["throw".into()]),
						..Default::default()
					}),
					..Default::default()
				},
				..Default::default()
			},
			config: &Config::default(),
			arch: Arch::X86_64,
			command: Command::Install,
		};

		let hook = Hook::new(context).unwrap();
		let result = hook.run(Script::Install);

		assert!(result.is_err());
	}
}
