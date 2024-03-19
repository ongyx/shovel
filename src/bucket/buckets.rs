use std::fs;
use std::iter;
use std::path;

use crate::bucket::Bucket;
use crate::error::{Error, Result};
use crate::util::subdirs;

/// A bucket manager.
///
/// Buckets are stored as sub-directories. For example:
/// * `dir`
///   * `bucket1`
///   * `bucket2`
///   * `...`
pub struct Buckets {
    dir: path::PathBuf,
}

impl Buckets {
    /// Returns a new bucket manager.
    ///
    /// # Arguments
    ///
    /// * `dir` - The directory where buckets are stored.
    pub fn new<P>(dir: P) -> Self
    where
        P: AsRef<path::Path>,
    {
        Self {
            dir: dir.as_ref().to_owned(),
        }
    }

    /// Yields buckets by name.
    pub fn iter(&self) -> Result<impl Iterator<Item = String> + '_> {
        subdirs(&self.dir)
    }

    /// Returns the path to a bucket, or None if it does not exist.
    pub fn path(&self, name: &str) -> Option<path::PathBuf> {
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
    pub fn get(&self, name: &str) -> Result<Bucket> {
        // Make sure the bucket still exists.
        let dir = self.path(name).ok_or(Error::BucketNotFound)?;

        Bucket::open(dir)
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
    pub fn add(&self, name: &str, url: &str) -> Result<Bucket> {
        // Bail if the bucket already exists.
        if self.path(name).is_some() {
            return Err(Error::BucketExists);
        }

        let dir = self.dir.join(name);

        Bucket::clone(url, dir)
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
    pub fn remove(&self, name: &str) -> Result<()> {
        match self.path(name) {
            Some(dir) => {
                fs::remove_dir_all(dir)?;

                Ok(())
            }
            None => Err(Error::BucketNotFound),
        }
    }

    /// Yields manifests in all buckets as a 2-tuple (bucket, manifest) by name.
    pub fn manifests(&self) -> Result<impl Iterator<Item = (String, String)>> {
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
    pub fn search<P>(&self, predicate: P) -> Result<impl Iterator<Item = (String, String)>>
    where
        P: Fn(&str, &str) -> bool,
    {
        // Move the predicate inside and filter the manifests.
        Ok(self.manifests()?.filter(move |(b, m)| predicate(&b, &m)))
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path;

    use serde_json;

    use super::*;
    use crate::test;

    fn buckets_dir() -> path::PathBuf {
        test::testdir().join("buckets")
    }

    #[test]
    fn buckets_iter() {
        let dir = buckets_dir();
        let buckets = Buckets::new(&dir);

        let mut names_from_buckets: Vec<_> = buckets.iter().unwrap().collect();
        names_from_buckets.sort();

        let mut names_from_fs: Vec<_> = fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().path())
            .filter(|p| p.is_dir())
            .map(|p| p.file_name().unwrap().to_str().unwrap().to_owned())
            .collect();
        names_from_fs.sort();

        assert_eq!(names_from_buckets, names_from_fs);
    }

    #[test]
    fn buckets_manifests() {
        let schema = test::schema();
        let buckets = Buckets::new(buckets_dir());

        for (bucket_name, manifest_name) in buckets.manifests().unwrap() {
            // Try to get the bucket...
            let bucket = buckets.get(&bucket_name).unwrap();
            // ...and parse the manifest.
            let manifest = bucket.manifest(&manifest_name).unwrap();

            // Get a value from the manifest.
            let manifest_value = serde_json::to_value(manifest).unwrap();

            if let Err(errors) = schema.validate(&manifest_value) {
                println!(
                    "Errors found while validating manifest {}/{} at '{}'",
                    bucket_name,
                    manifest_name,
                    bucket.manifest_path(&manifest_name).display()
                );

                for error in errors {
                    dbg!(error);
                }
            };
        }
    }
}
