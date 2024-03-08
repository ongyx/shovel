use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::iter;
use std::path::PathBuf;

use thiserror;

use crate::bucket::{Bucket, BucketError};
use crate::config::Config;
use crate::util::osstr_to_string;

/// A shovel error.
/// This acts as a catch-all for other errors.
#[derive(thiserror::Error, Debug)]
pub enum ShovelError {
    /// A bucket does not exist.
    #[error("Bucket not found")]
    BucketNotFound,

    /// A bucket exists.
    #[error("Bucket already exists")]
    BucketExists,

    /// A bucket-specific error.
    #[error(transparent)]
    Bucket(#[from] BucketError),

    /// An underlying error with std::io.
    #[error(transparent)]
    IO(#[from] io::Error),
}

pub type ShovelResult<T> = Result<T, ShovelError>;

/// A high-level interface to Shovel.
pub struct Shovel {
    config: Config,
    buckets: HashMap<String, Bucket>,
}

impl Shovel {
    /// Creates a new shovel.
    ///
    /// # Arguments
    ///
    /// * `config` - The config to use.
    pub fn new(config: Config) -> ShovelResult<Self> {
        // Ensure the installation directory, and all sub-directories, exist.
        for dir in [config.install_dir(), config.bucket_dir()] {
            fs::create_dir_all(dir)?;
        }

        Ok(Shovel {
            config,
            buckets: HashMap::new(),
        })
    }

    /// Returns an iterator over all buckets by name.
    pub fn buckets(&self) -> ShovelResult<impl Iterator<Item = String>> {
        // Collect the first error.
        let entries: Result<Vec<_>, _> = fs::read_dir(self.config.bucket_dir())?.collect();

        Ok(entries?
            .into_iter()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .map(|p| osstr_to_string(p.file_name().unwrap())))
    }

    /// Returns the path to a bucket, or None if it does not exist.
    pub fn bucket_path(&self, name: &str) -> Option<PathBuf> {
        let mut path = self.config.bucket_dir();
        path.push(name);

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Opens and returns a bucket.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the bucket.
    ///
    /// # Errors
    ///
    /// If the bucket does not exist, `ShovelError::BucketNotFound` is returned.
    pub fn bucket(&mut self, name: &str) -> ShovelResult<&mut Bucket> {
        // Make sure the bucket still exists.
        let dir = self.bucket_path(name).ok_or(ShovelError::BucketNotFound)?;

        match self.buckets.entry(name.to_owned()) {
            // Return the existing bucket.
            Entry::Occupied(o) => Ok(o.into_mut()),
            // Open a new bucket.
            Entry::Vacant(v) => Ok(v.insert(Bucket::open(dir)?)),
        }
    }

    /// Adds a bucket.
    ///
    /// # Arguments
    ///
    /// * `name` - The name to add the remote bucket as.
    /// * `url` - The Git URL of the remote bucket.
    ///
    /// # Errors
    ///
    /// If the bucket name already exists, `ShovelError::BucketExists` is returned.
    pub fn add_bucket(&mut self, name: &str, url: &str) -> ShovelResult<&mut Bucket> {
        // Bail if the bucket already exists.
        if self.bucket_path(name).is_some() {
            return Err(ShovelError::BucketExists);
        }

        let mut dir = self.config.bucket_dir();
        dir.push(name);

        let bucket = Bucket::clone(url, dir)?;

        // It is possible for a bucket to be removed from the filesystem while the handle persists.
        // Just in case, remove the previous entry (if any) to guarantee .or_insert will insert the bucket.
        self.buckets.remove(name);

        Ok(self.buckets.entry(name.to_owned()).or_insert(bucket))
    }

    /// Removes a bucket.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the bucket.
    ///
    /// # Errors
    ///
    /// If the bucket does not exist, `ShovelError::BucketNotFound` is returned.
    pub fn remove_bucket(&mut self, name: &str) -> ShovelResult<()> {
        match self.bucket_path(name) {
            Some(dir) => {
                // Remove the handle - it will be invalid after the bucket directory is deleted.
                self.buckets.remove(name);

                fs::remove_dir_all(dir)?;

                Ok(())
            }
            None => Err(ShovelError::BucketNotFound),
        }
    }

    /// Returns an iterator over manifests in all buckets and yields (bucket_name, manifest_name).
    pub fn manifests(&mut self) -> ShovelResult<impl Iterator<Item = (String, String)>> {
        // Get manifests from each bucket.
        // Any error in opening a bucket is returned.
        let bucket_manifests: ShovelResult<Vec<_>> = self
            .buckets()?
            .map(|b| {
                // NOTE: The bucket cannot be returned directly, since self cannot escape this closure
                let bucket = self.bucket(&b)?;
                let manifests = bucket.manifests()?;

                Ok((b, manifests))
            })
            .collect();

        let manifests = bucket_manifests?
            .into_iter()
            // Zip each bucket's manifest as a two-tuple (bucket_name, manifest_name).
            .map(|(b, m)| iter::repeat(b).zip(m))
            // Flatten the manifest iterators.
            .flatten();

        Ok(manifests)
    }

    /// Searches all buckets for app manifests and yields (bucket_name, manifest_name).
    /// This is a convenience function for filtering `Self::manifests()`.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A predicate function with input (bucket_name, manifest_name) that determines if the pair should be yielded.
    pub fn search<P>(
        &mut self,
        predicate: P,
    ) -> ShovelResult<impl Iterator<Item = (String, String)>>
    where
        P: Fn(&str, &str) -> bool,
    {
        // Move the predicate inside and filter the manifests.
        Ok(self.manifests()?.filter(move |(b, m)| predicate(&b, &m)))
    }
}
