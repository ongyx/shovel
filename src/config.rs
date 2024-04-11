use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use home;

use crate::json;
use crate::util;

/// Returns the current user's installation directory at `%USERPROFILE%\scoop`.
pub fn user_install_dir() -> &'static Path {
	static DEFAULT_INSTALL_DIR: OnceLock<PathBuf> = OnceLock::new();

	DEFAULT_INSTALL_DIR.get_or_init(|| {
		home::home_dir()
			.expect("home dir should exist")
			.join("scoop")
	})
}

/// Returns the default installation directory.
///
/// # Deprecated
///
/// This function was renamed to user_install_dir to better reflect its intended use.
#[deprecated(since = "0.5.0", note = "use `user_install_dir` instead")]
pub fn default_install_dir() -> &'static Path {
	user_install_dir()
}

/// Returns the global installation directory at `%ProgramData%\scoop`.
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
	pub fn install_dir(&self) -> &Path {
		Path::new(&self.install_dir)
	}

	/// Returns the directory where apps are stored.
	pub fn app_dir(&self) -> PathBuf {
		self.install_dir().join("apps")
	}

	/// Returns the directory where buckets are stored.
	pub fn bucket_dir(&self) -> PathBuf {
		self.install_dir().join("buckets")
	}

	/// Returns the directory where app downloads are cached.
	pub fn cache_dir(&self) -> PathBuf {
		self.install_dir().join("cache")
	}

	/// Returns the directory where user data is persisted.
	pub fn persist_dir(&self) -> PathBuf {
		self.install_dir().join("persist")
	}
}

impl Default for Config {
	fn default() -> Self {
		Config {
			install_dir: util::osstr_to_string(user_install_dir().as_os_str()),
		}
	}
}
