use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use git2;
use thiserror;

use crate::manifest::Manifest;
use crate::util::osstr_to_string;

/// A bucket error.
#[derive(thiserror::Error, Debug)]
pub enum BucketError {
    /// An app's manifest does not exist.
    #[error("Manifest not found")]
    ManifestNotFound,

    /// An underlying error with serde_json.
    #[error(transparent)]
    JSON(#[from] serde_json::Error),

    /// An underlying error with std::io.
    #[error(transparent)]
    IO(#[from] io::Error),

    /// An underlying error with Git.
    #[error(transparent)]
    Git(#[from] git2::Error),
}

/// A bucket result.
pub type BucketResult<T> = Result<T, BucketError>;

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
    pub fn open<P>(dir: P) -> BucketResult<Self>
    where
        P: AsRef<Path>,
    {
        let dir = dir.as_ref().to_owned();
        let repo = git2::Repository::open(&dir)?;

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
    pub fn origin(&self) -> BucketResult<String> {
        let origin = self.repo.find_remote("origin")?;

        Ok(origin.url().unwrap_or("").to_owned())
    }

    /// Returns an iterator over all app manifests by name.
    pub fn manifests(&self) -> BucketResult<impl Iterator<Item = String>> {
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
    pub fn manifest(&self, name: &str) -> BucketResult<Manifest> {
        let path = self
            .manifest_path(name)
            .ok_or(BucketError::ManifestNotFound)?;

        let file = fs::File::open(path)?;

        let reader = io::BufReader::new(file);
        let manifest = serde_json::from_reader(reader)?;

        Ok(manifest)
    }
}
