use std::fs;

use crate::bucket::Buckets;
use crate::config::Config;
use crate::error::Result;

/// A high-level interface to Shovel.
pub struct Shovel {
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
        let bucket_dir = config.bucket_dir();

        // Ensure the installation directory, and all sub-directories, exist.
        for dir in [&install_dir, &bucket_dir] {
            fs::create_dir_all(dir)?;
        }

        Ok(Shovel {
            buckets: Buckets::new(bucket_dir),
        })
    }
}
