use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use home;

use crate::json;
use crate::util;

/// Returns the current user's installation directory at `%USERPROFILE%\scoop`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn user_install_dir() -> &'static Path {
	static DEFAULT_INSTALL_DIR: OnceLock<PathBuf> = OnceLock::new();

	DEFAULT_INSTALL_DIR.get_or_init(|| {
		home::home_dir()
			.expect("home dir should exist")
			.join("scoop")
	})
}

/// Returns the global installation directory at `%ProgramData%\scoop`.
#[must_use]
#[allow(clippy::missing_panics_doc)]
pub fn global_install_dir() -> &'static Path {
	static GLOBAL_INSTALL_DIR: OnceLock<PathBuf> = OnceLock::new();

	GLOBAL_INSTALL_DIR.get_or_init(|| {
		env::var("ProgramData")
			.map(PathBuf::from)
			.expect("ProgramData env var should exist")
			.join("scoop")
	})
}

json::json_struct_nodefault! {
	/// A set of configuration options for Shovel.
	/// Use `Default::default` for the defaults.
	pub struct Config {
		/// The installation directory where apps, buckets, etc. are stored.
		pub install_dir: String,
	}
}

impl Config {
	/// Returns the installation directory as a path.
	#[must_use]
	pub fn install_dir(&self) -> PathBuf {
		PathBuf::from(&self.install_dir)
	}

	/// Checks if the installation directory is global.
	#[must_use]
	pub fn is_global(&self) -> bool {
		self.install_dir() == global_install_dir()
	}

	/// Returns the directory where apps are stored.
	#[must_use]
	pub fn app_dir(&self) -> PathBuf {
		self.install_dir().join("apps")
	}

	/// Returns the directory where buckets are stored.
	#[must_use]
	pub fn bucket_dir(&self) -> PathBuf {
		self.install_dir().join("buckets")
	}

	/// Returns the directory where app downloads are cached.
	#[must_use]
	pub fn cache_dir(&self) -> PathBuf {
		self.install_dir().join("cache")
	}

	/// Returns the directory where user data is persisted.
	#[must_use]
	pub fn persist_dir(&self) -> PathBuf {
		self.install_dir().join("persist")
	}

	/// Returns the directory where PowerShell modules are symlinked.
	#[must_use]
	pub fn module_dir(&self) -> PathBuf {
		self.install_dir().join("modules")
	}
}

impl Default for Config {
	fn default() -> Self {
		Config {
			install_dir: util::path_to_string(user_install_dir()),
		}
	}
}
