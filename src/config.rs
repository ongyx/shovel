use std::path;
use std::sync;

use home;

use crate::json::json_struct_nodefault;
use crate::util::osstr_to_string;

static DEFAULT_INSTALL_DIR: sync::OnceLock<path::PathBuf> = sync::OnceLock::new();

/// Returns the default installation directory.
pub fn get_default_install_dir() -> &'static path::Path {
    // A new PathBuf is allocated on PathBuf.join,
    // but since this is only called once it does not matter.
    DEFAULT_INSTALL_DIR.get_or_init(|| home::home_dir().unwrap().join("scoop"))
}

json_struct_nodefault! {
    /// A set of configuration options for Shovel.
    /// Use `Default::default` for the defaults.
    pub struct Config {
        /// The installation directory where apps, buckets, etc. are stored.
        pub install_dir: String,
    }
}

impl Config {
    /// Returns the installation directory as a path.
    pub fn install_dir(&self) -> path::PathBuf {
        path::PathBuf::from(&self.install_dir)
    }

    /// Returns the directory where apps are stored.
    pub fn app_dir(&self) -> path::PathBuf {
        self.install_dir().join("apps")
    }

    /// Returns the directory where buckets are stored.
    pub fn bucket_dir(&self) -> path::PathBuf {
        self.install_dir().join("buckets")
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            install_dir: osstr_to_string(get_default_install_dir().as_os_str()),
        }
    }
}
