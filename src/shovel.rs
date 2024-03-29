use std::fs;

use crate::app::Apps;
use crate::bucket::Buckets;
use crate::config::Config;
use crate::error::Result;

/// A high-level interface to Shovel.
pub struct Shovel {
    /// The app manager.
    pub apps: Apps,

    /// The bucket manager.
    pub buckets: Buckets,
}

impl Shovel {
    /// Creates a new shovel.
    ///
    /// # Arguments
    ///
    /// * `config` - The config to use.
    pub fn new(config: Config) -> Result<Self> {
        let install_dir = config.install_dir();
        let app_dir = config.app_dir();
        let bucket_dir = config.bucket_dir();

        // Ensure the installation directory, and all sub-directories, exist.
        for dir in [&install_dir, &app_dir, &bucket_dir] {
            fs::create_dir_all(dir)?;
        }

        Ok(Shovel {
            apps: Apps::new(app_dir),
            buckets: Buckets::new(bucket_dir),
        })
    }
}
