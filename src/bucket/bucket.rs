use std::fs;
use std::io;
use std::path;

use git2;

use crate::error::{Error, Result};
use crate::manifest::Manifest;
use crate::util::{json_from_file, osstr_to_string};

/// A collection of app manifests in a Git repository.
///
/// The repository must have a `bucket` directory, with manifest files in `.json` format.
/// Refer to [`crate::manifest::Manifest`] for the schema.
pub struct Bucket {
    dir: path::PathBuf,
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
        P: AsRef<path::Path>,
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
        P: AsRef<path::Path>,
    {
        let dir = dir.as_ref().to_owned();
        let repo = git2::Repository::clone(url, &dir)?;

        Ok(Bucket { dir, repo })
    }

    /// Returns the bucket directory.
    pub fn dir(&self) -> &path::Path {
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

    /// Returns the path to an app manifest.
    pub fn manifest_path(&self, name: &str) -> path::PathBuf {
        self.dir().join(format!(r"bucket\{}.json", name))
    }

    /// Parses and returns an app manifest.
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

        json_from_file(&path).map_err(|err| match err {
            // Map the NotFound IO error kind to ManifestNotFound.
            Error::IO(ioerr) if ioerr.kind() == io::ErrorKind::NotFound => Error::ManifestNotFound,
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

        let commit = self.find(relpath)?;

        Ok(commit.expect("Manifest is commited"))
    }

    fn find(&self, path: &path::Path) -> Result<Option<git2::Commit>> {
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

    fn is_commited(&self, path: &path::Path) -> Result<bool> {
        let status = self.repo.status_file(path)?;

        Ok(!(status.contains(git2::Status::WT_NEW) || status.contains(git2::Status::INDEX_NEW)))
    }
}

#[cfg(test)]
mod tests {
    use git2;
    use tempfile;

    use super::*;

    #[test]
    fn bucket_name() {
        let name = "this is a bucket";
        let (temp_dir, _) = create_repo(name);

        let bucket = Bucket::open(&temp_dir).unwrap();

        assert_eq!(bucket.name(), name);
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
}
