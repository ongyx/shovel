use std::collections::hash_map::Entry;
use std::collections::HashMap;
use std::fs;
use std::io;
use std::iter;
use std::path::{Path, PathBuf};

use git2;

use crate::error::{Error, Result};
use crate::json;
use crate::manifest::Manifest;
use crate::util;

/// A collection of manifests in a Git repository.
///
/// The repository must have a `bucket` directory, with manifest files in `.json` format.
/// Refer to [`crate::manifest::Manifest`] for the schema.
pub struct Bucket {
    dir: PathBuf,
    repo: git2::Repository,
}

impl Bucket {
    /// Opens an existing bucket.
    ///
    /// # Arguments
    ///
    /// * `dir` - The path to the bucket. It must point to a directory.
    pub fn open<P>(dir: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let dir = dir.as_ref().to_owned();
        let repo = git2::Repository::open(&dir)?;

        Ok(Bucket { dir, repo })
    }

    /// Clone a remote bucket.
    ///
    /// # Arguments
    ///
    /// * `url` - The Git URL of the remote bucket.
    /// * `dir` - The path to clone the remote bucket to. It must not exist yet.
    pub fn clone<P>(url: &str, dir: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let dir = dir.as_ref().to_owned();
        let repo = git2::Repository::clone(url, &dir)?;

        Ok(Bucket { dir, repo })
    }

    /// Returns the bucket directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Returns the bucket name.
    pub fn name(&self) -> String {
        let name = self.dir().file_name().map(|o| o.to_str().unwrap());

        // File name is None only if the directory is '..'.
        name.unwrap_or("").to_owned()
    }

    /// Returns the bucket origin, i.e., the URL it was cloned from.
    pub fn origin(&self) -> Result<String> {
        let origin = self.repo.find_remote("origin")?;

        Ok(origin.url().unwrap_or("").to_owned())
    }

    /// Returns the last commit in the bucket.
    pub fn commit(&self) -> Result<git2::Commit> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;

        Ok(commit)
    }

    /// Yields manifests by name.
    pub fn manifests(&self) -> Result<impl Iterator<Item = String>> {
        let dir = self.dir().join("bucket");
        let entries = fs::read_dir(dir)?;

        let manifests = entries.filter_map(|res| res.ok()).filter_map(|entry| {
            let path = entry.path();
            let ext = path.extension().unwrap_or_default();

            if ext == "json" {
                // Take only the file stem (i.e., 'example' for 'example.json')
                let name = path.file_stem().unwrap();

                Some(util::osstr_to_string(name))
            } else {
                None
            }
        });

        Ok(manifests)
    }

    /// Returns the path to an manifest.
    pub fn manifest_path(&self, name: &str) -> PathBuf {
        self.dir().join(format!(r"bucket\{}.json", name))
    }

    /// Parses and returns an manifest.
    ///
    /// # Arguments
    ///
    /// * `name` - The app's name.
    ///
    /// # Errors
    ///
    /// If the manifest file does not exist, `Error::ManifestNotFound` is returned.
    pub fn manifest(&self, name: &str) -> Result<Manifest> {
        let path = self.manifest_path(name);

        json::from_file(&path).map_err(|err| match err {
            // Map the NotFound IO error kind to ManifestNotFound.
            Error::Io(ioerr) if ioerr.kind() == io::ErrorKind::NotFound => Error::ManifestNotFound,
            _ => err,
        })
    }

    /// Returns the last commit made to a manifest.
    ///
    /// If the manifest has not been commited, None is returned.
    pub fn manifest_commit(&self, name: &str) -> Result<git2::Commit> {
        let path = self.manifest_path(name);

        // Ensure the manifest exists.
        if !path.exists() {
            return Err(Error::ManifestNotFound);
        }

        // SAFETY: path is always a child of dir.
        let relpath = path.strip_prefix(&self.dir).unwrap();

        // Ensure the manifest is commited.
        if !self.is_commited(relpath)? {
            return Err(Error::ManifestNotCommited);
        }

        let commit = self.find_commit(relpath)?;

        Ok(commit.expect("Manifest is commited"))
    }

    fn find_commit(&self, path: &Path) -> Result<Option<git2::Commit>> {
        let mut revwalk = self.repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        revwalk.push_head()?;

        let mut old_tree: Option<git2::Tree> = None;

        for oid in revwalk {
            let commit = self.repo.find_commit(oid?)?;
            let tree = Some(commit.tree()?);

            let diff = self
                .repo
                .diff_tree_to_tree(old_tree.as_ref(), tree.as_ref(), None)?;

            let has_file = diff.deltas().any(|delta| {
                // SAFETY: Okay as long as the path is UTF-8.
                let delta_path = delta.new_file().path().unwrap();

                delta_path == path
            });

            if has_file {
                return Ok(Some(commit));
            }

            old_tree = tree;
        }

        Ok(None)
    }

    fn is_commited(&self, path: &Path) -> Result<bool> {
        let status = self.repo.status_file(path)?;

        Ok(!(status.contains(git2::Status::WT_NEW) || status.contains(git2::Status::INDEX_NEW)))
    }
}

/// A bucket manager.
///
/// Buckets are stored as sub-directories. For example:
/// * `dir`
///   * `bucket1`
///   * `bucket2`
///   * `...`
pub struct Buckets {
    dir: PathBuf,
    handles: HashMap<String, Bucket>,
}

// Bucket-related functions.
impl Buckets {
    /// Returns a new bucket manager.
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
            handles: HashMap::new(),
        }
    }

    /// Yields buckets by name.
    pub fn iter(&self) -> Result<impl Iterator<Item = String>> {
        util::subdirs(self.dir.clone())
    }

    /// Returns the path to a bucket.
    pub fn path(&self, name: &str) -> PathBuf {
        self.dir.join(name)
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
    pub fn open(&mut self, name: &str) -> Result<&mut Bucket> {
        let dir = self.path(name);
        let entry = self.handles.entry(name.to_owned());

        match entry {
            Entry::Occupied(occupied) => Ok(occupied.into_mut()),
            Entry::Vacant(vacant) => {
                // Make sure the bucket exists.
                if dir.try_exists()? {
                    let bucket = Bucket::open(dir)?;

                    // Insert the bucket and return it.
                    Ok(vacant.insert(bucket))
                } else {
                    Err(Error::BucketNotFound)
                }
            }
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
        let dir = self.path(name);
        let entry = self.handles.entry(name.to_owned());

        match entry {
            Entry::Occupied(_) => Err(Error::BucketExists),
            Entry::Vacant(vacant) => {
                if !dir.try_exists()? {
                    let dir = self.dir.join(name);

                    Ok(vacant.insert(Bucket::clone(url, dir)?))
                } else {
                    Err(Error::BucketExists)
                }
            }
        }
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
        let entry = self.handles.entry(name.to_owned());

        // Remove the existing bucket handle, if any.
        if let Entry::Occupied(occupied) = entry {
            occupied.remove_entry();
        }

        let dir = self.path(name);

        if dir.try_exists()? {
            fs::remove_dir_all(dir)?;

            Ok(())
        } else {
            Err(Error::BucketNotFound)
        }
    }

    /// Yields manifests in all buckets as a 2-tuple (bucket, manifest) by name.
    pub fn manifests(&mut self) -> Result<impl Iterator<Item = (String, String)>> {
        // Get manifests from each bucket.
        // Any error in opening a bucket is returned.
        let manifests: Result<Vec<_>> = self
            .iter()?
            .map(|name| {
                // NOTE: The bucket cannot be returned directly, since self cannot escape this closure
                let bucket = self.open(&name)?;
                let manifests = bucket.manifests()?;

                Ok((name, manifests))
            })
            .collect();

        let manifests = manifests?
            .into_iter()
            // Zip each bucket's manifest as a two-tuple (bucket, manifest).
            .map(|(b, m)| iter::repeat(b).zip(m))
            // Flatten the manifest iterators.
            .flatten();

        Ok(manifests)
    }

    /// Searches all buckets for manifests and yields (bucket, manifest) by name.
    ///
    /// # Arguments
    ///
    /// * `predicate` - A predicate function with input (bucket, manifest) that determines if the pair should be yielded.
    pub fn search<P>(&mut self, predicate: P) -> Result<impl Iterator<Item = (String, String)>>
    where
        P: Fn(&str, &str) -> bool,
    {
        // Move the predicate inside and filter the manifests.
        Ok(self.manifests()?.filter(move |(b, m)| predicate(&b, &m)))
    }

    /// Searches all buckets for a single manifest and returns (bucket, manifest).
    ///
    /// # Arguments
    ///
    /// * `name`- The name of the manifest.
    pub fn manifest(&mut self, name: &str) -> Result<(&mut Bucket, Manifest)> {
        let mut search = self.search(|_, manifest_name| manifest_name == name)?;

        match search.next() {
            Some((bucket, manifest)) => {
                let bucket = self.open(&bucket)?;
                let manifest = bucket.manifest(&manifest)?;

                Ok((bucket, manifest))
            }
            None => Err(Error::ManifestNotFound),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use git2;
    use tempfile;

    use super::*;
    use crate::test;

    #[test]
    fn name() {
        let name = "this is a bucket";
        let (temp_dir, _) = create_repo(name);

        let bucket = Bucket::open(&temp_dir).unwrap();

        assert_eq!(bucket.name(), name);
    }

    #[test]
    fn origin() {
        let bucket = Bucket::open(test::testdir().join(r"buckets\main")).unwrap();

        assert_eq!(
            bucket.origin().unwrap(),
            "https://github.com/ScoopInstaller/Main"
        );
    }

    fn create_repo(name: &str) -> (tempfile::TempDir, git2::Repository) {
        let temp_dir = tempfile::Builder::new()
            // Disable randomizing the name.
            .rand_bytes(0)
            .prefix(name)
            .tempdir()
            .unwrap();
        let repo = git2::Repository::init(&temp_dir).unwrap();

        (temp_dir, repo)
    }

    fn buckets_dir() -> PathBuf {
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
        let mut buckets = Buckets::new(buckets_dir());

        for (name, manifest_name) in buckets.manifests().unwrap() {
            // Try to get the bucket...
            let bucket = buckets.open(&name).unwrap();
            // ...and parse the manifest.
            bucket.manifest(&manifest_name).unwrap();
        }
    }
}
