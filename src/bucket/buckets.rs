use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs;
use std::io::Result as IOResult;
use std::iter;
use std::path::{Path, PathBuf};

use crate::bucket::{Bucket, Error, Result};
use crate::util::osstr_to_string;

/// A bucket manager.
pub struct Buckets {
    dir: PathBuf,
    map: HashMap<String, Bucket>,
}

impl Buckets {
    /// Creates and returns a new bucket manager.
    ///
    /// # Arguments
    ///
    /// * `dir` - The directory where buckets are stored.
    pub fn new<P>(dir: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            dir: dir.as_ref().to_owned(),
            map: HashMap::new(),
        }
    }

    /// Returns an iterator over all buckets by name.
    pub fn iter(&self) -> Result<impl Iterator<Item = String>> {
        // Collect the first error.
        let entries: IOResult<Vec<_>> = fs::read_dir(&self.dir)?.collect();

        Ok(entries?
            .into_iter()
            .map(|e| e.path())
            .filter(|p| p.is_dir())
            .map(|p| osstr_to_string(p.file_name().unwrap())))
    }

    /// Returns the path to a bucket, or None if it does not exist.
    pub fn path(&self, name: &str) -> Option<PathBuf> {
        let path = self.dir.join(name);

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
    /// If the bucket does not exist, `Error::BucketNotFound` is returned.
    pub fn get(&mut self, name: &str) -> Result<&mut Bucket> {
        // Make sure the bucket still exists.
        let dir = self.path(name).ok_or(Error::BucketNotFound)?;

        match self.map.entry(name.to_owned()) {
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
    /// If the bucket name already exists, `Error::BucketExists` is returned.
    pub fn add(&mut self, name: &str, url: &str) -> Result<&mut Bucket> {
        // Bail if the bucket already exists.
        if self.path(name).is_some() {
            return Err(Error::BucketExists);
        }

        let dir = self.dir.join(name);

        let bucket = Bucket::clone(url, dir)?;

        // It is possible for a bucket to be removed from the filesystem while the handle persists.
        // Just in case, remove the previous entry (if any) to guarantee .or_insert will insert the bucket.
        self.map.remove(name);

        Ok(self.map.entry(name.to_owned()).or_insert(bucket))
    }

    /// Removes a bucket.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the bucket.
    ///
    /// # Errors
    ///
    /// If the bucket does not exist, `Error::BucketNotFound` is returned.
    pub fn remove(&mut self, name: &str) -> Result<()> {
        match self.path(name) {
            Some(dir) => {
                // Remove the handle - it will be invalid after the bucket directory is deleted.
                self.map.remove(name);

                fs::remove_dir_all(dir)?;

                Ok(())
            }
            None => Err(Error::BucketNotFound),
        }
    }

    /// Returns an iterator over manifests in all buckets and yields (bucket_name, manifest_name).
    pub fn manifests(&mut self) -> Result<impl Iterator<Item = (String, String)>> {
        // Get manifests from each bucket.
        // Any error in opening a bucket is returned.
        let bucket_manifests: Result<Vec<_>> = self
            .iter()?
            .map(|b| {
                // NOTE: The bucket cannot be returned directly, since self cannot escape this closure
                let bucket = self.get(&b)?;
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
    pub fn search<P>(&mut self, predicate: P) -> Result<impl Iterator<Item = (String, String)>>
    where
        P: Fn(&str, &str) -> bool,
    {
        // Move the predicate inside and filter the manifests.
        Ok(self.manifests()?.filter(move |(b, m)| predicate(&b, &m)))
    }
}
