use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use home;
use serde::{Deserialize, Serialize};

use crate::util;

static DEFAULT_INSTALL_DIR: OnceLock<PathBuf> = OnceLock::new();

/// Returns the default installation directory.
pub fn get_default_install_dir() -> &'static Path {
    DEFAULT_INSTALL_DIR.get_or_init(|| {
        let mut install_dir = home::home_dir().unwrap();
        install_dir.push("scoop");
        install_dir
    })
}

/// A set of configuration options for Shovel.
/// Use `Default::default` for the defaults.
#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    /// The installation directory where apps, buckets, etc. are stored.
    pub install_dir: String,
}

impl Config {
    /// Returns the installation directory as a path.
    pub fn install_dir(&self) -> PathBuf {
        PathBuf::from(&self.install_dir)
    }

    /// Returns the directory where buckets are stored.
    pub fn bucket_dir(&self) -> PathBuf {
        let mut dir = self.install_dir();
        dir.push("buckets");
        dir
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            install_dir: util::osstr_to_string(get_default_install_dir().as_os_str()),
        }
    }
}
