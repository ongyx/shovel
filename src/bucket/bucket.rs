use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use git2;

use crate::app::Manifest;
use crate::util::osstr_to_string;

use crate::bucket::{Error, Result};

/// A collection of app manifests in a Git repository.
///
/// At minimum, the repository must have a `bucket` directory, with manifest files in `.json` format.
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

    /// Returns the UNIX timestamp in seconds for the last commit in the bucket.
    pub fn timestamp(&self) -> Result<i64> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;

        Ok(commit.time().seconds())
    }

    /// Returns an iterator over all app manifests by name.
    pub fn manifests(&self) -> Result<impl Iterator<Item = String>> {
        let dir = self.dir().join("bucket");
        let entries = fs::read_dir(dir)?;

        Ok(entries
            .filter_map(|r| r.ok())
            .map(|e| e.path())
            // Only yield files ending in .json.
            .filter(|p| p.extension().map_or(false, |e| e == "json"))
            .map(|p| osstr_to_string(p.file_stem().unwrap())))
    }

    /// Returns the path to an app manifest, or None if it does not exist.
    pub fn manifest_path(&self, name: &str) -> Option<PathBuf> {
        let mut path = self.dir().to_owned();
        path.push(format!(r"bucket\{}.json", name));

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Parses and returns an app manifest.
    ///
    /// # Arguments
    ///
    /// * `name` - The app's name.
    ///
    /// # Errors
    ///
    /// If the manifest file does not exist, `BucketError::ManifestNotFound` is returned.
    pub fn manifest(&self, name: &str) -> Result<Manifest> {
        let path = self.manifest_path(name).ok_or(Error::ManifestNotFound)?;

        let file = fs::File::open(path)?;

        let reader = io::BufReader::new(file);
        let manifest = serde_json::from_reader(reader)?;

        Ok(manifest)
    }
}
