use std::fs;
use std::io;
use std::iter::{self, Flatten, Repeat, Zip};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::vec;

use git2;
use git2::build;

use crate::error::{Error, Result};
use crate::json;
use crate::manifest::Manifest;
use crate::util;

fn manifest_from_file<P>(path: P) -> Result<Manifest>
where
    P: AsRef<Path>,
{
    json::from_file(path).map_err(|err| match err {
        // Map the NotFound IO error kind to ManifestNotFound.
        Error::Io(ioerr) if ioerr.kind() == io::ErrorKind::NotFound => Error::ManifestNotFound,
        _ => err,
    })
}

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
    /// * `dir` - The path to clone to. It must not exist yet.
    /// * `builder` - The builder to clone with. If None, a new builder is created.
    pub fn clone<P>(url: &str, dir: P, builder: Option<&mut build::RepoBuilder>) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let dir = dir.as_ref().to_owned();

        // This is needed to get a &mut reference.
        let mut new_builder = None;

        let builder = builder
            .or_else(|| {
                new_builder = Some(build::RepoBuilder::new());
                new_builder.as_mut()
            })
            // SAFETY: The builder arg is not None, or a new one was initialised.
            .unwrap();

        let repo = builder.clone(url, &dir)?;

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

    /// Returns the bucket URL, i.e., where it was cloned from.
    pub fn url(&self) -> Result<String> {
        Ok(self.origin()?.url().unwrap_or("").to_owned())
    }

    fn origin(&self) -> Result<git2::Remote> {
        let origin = self.repo.find_remote("origin")?;

        Ok(origin)
    }

    /// Returns the last commit in the bucket.
    pub fn commit(&self) -> Result<git2::Commit> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;

        Ok(commit)
    }

    /// Yields the commits made to the bucket from HEAD until the commit pointed to by `since`.
    ///
    /// # Arguments
    ///
    /// * `since` - The commit ID to yield until.
    pub fn commits<'c>(&'c self, since: git2::Oid) -> Result<Commits<'c>> {
        Commits::new(&self.repo, since)
    }

    /// Parses and yields each manifest in the bucket as (name, manifest) where predicate(name) returns true.
    ///
    /// # Arguments
    ///
    /// `predicate` - A predicate function that determines if a manifest should be yielded.
    pub fn search<P>(&self, predicate: P) -> Result<Search<P>>
    where
        P: Fn(&str) -> bool,
    {
        let dir = self.dir().join("bucket");

        Search::new(dir, predicate)
    }

    /// Parses and yields each manifest in the bucket as (name, manifest).
    /// This is a convenience function over `Self::search`.
    pub fn manifests(&self) -> Result<Search<fn(&str) -> bool>> {
        self.search(|_| true)
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

        manifest_from_file(&path)
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

    /// Updates the bucket by pulling new changes. Only fast-forwarding is supported.
    ///
    /// # Arguments
    ///
    /// `fetch_options` - The options to use for fetching changes.
    /// `checkout_builder` - The builder to use for checking out changes.
    pub fn pull(
        &mut self,
        fetch_options: Option<&mut git2::FetchOptions>,
        checkout_builder: Option<&mut build::CheckoutBuilder>,
    ) -> Result<()> {
        // Get the HEAD branch.
        let branch = self.repo.head()?.name().unwrap().to_owned();

        // Get the origin remote.
        // TODO: Don't hardcode?
        let mut origin = self.origin()?;

        // Fetch updates for the HEAD branch from the origin.
        let new_head = self.fetch(&[&branch], &mut origin, fetch_options)?;

        // Attempt to fast-forward changes.
        self.fast_forward(&branch, &new_head, checkout_builder)?;

        Ok(())
    }

    fn fast_forward(
        &self,
        branch: &str,
        commit: &git2::AnnotatedCommit,
        checkout_builder: Option<&mut build::CheckoutBuilder>,
    ) -> Result<()> {
        let mut new_checkout_builder = None;

        let checkout_builder = checkout_builder
            .or_else(|| {
                new_checkout_builder = Some(build::CheckoutBuilder::new());
                new_checkout_builder.as_mut()
            })
            .unwrap();

        // NOTE: According to the git2 pull example, not including this option causes the working directory to not update.
        checkout_builder.force();

        // Obtain a reference to the branch HEAD.
        let mut head = self.repo.find_reference(branch)?;

        // Set the branch HEAD to the new commit ID.
        head.set_target(commit.id(), "pull: Fast-forward")?;

        // Set the repository HEAD to the branch HEAD.
        self.repo.set_head(branch)?;

        // Checkout the new changes.
        self.repo.checkout_head(Some(checkout_builder))?;

        Ok(())
    }

    fn fetch(
        &self,
        branches: &[&str],
        remote: &mut git2::Remote,
        fetch_options: Option<&mut git2::FetchOptions>,
    ) -> Result<git2::AnnotatedCommit> {
        remote.fetch(branches, fetch_options, None)?;

        // Get the HEAD of the fetched remote.
        let head = self.repo.find_reference("FETCH_HEAD")?;

        Ok(self.repo.reference_to_annotated_commit(&head)?)
    }
}

/// An iterator over buckets. Created by the `iter` method on `Buckets`.
pub struct Iter {
    dirs: util::Dirs,
}

impl Iter {
    fn new<P>(dir: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            dirs: util::dirs(dir)?,
        })
    }
}

impl Iterator for Iter {
    type Item = Result<Bucket>;

    fn next(&mut self) -> Option<Self::Item> {
        let dir = self.dirs.next()?;

        Some(Bucket::open(dir))
    }
}

/// An iterator over commits since a specific commit. Created by the `commits` method on `Buckets`.
pub struct Commits<'c> {
    repo: &'c git2::Repository,
    revwalk: git2::Revwalk<'c>,
    since: git2::Oid,
}

impl<'c> Commits<'c> {
    fn new(repo: &'c git2::Repository, since: git2::Oid) -> Result<Self> {
        // Create a revwalk to iterate commits from HEAD chronologically.
        let mut revwalk = repo.revwalk()?;
        revwalk.set_sorting(git2::Sort::TIME)?;
        revwalk.push_head()?;

        Ok(Self {
            repo,
            revwalk,
            since,
        })
    }
}

impl<'c> Iterator for Commits<'c> {
    type Item = git2::Commit<'c>;

    fn next(&mut self) -> Option<Self::Item> {
        // Get the next OID.
        let oid = self.revwalk.next()?.ok()?;

        if oid != self.since {
            // Return the commit.
            // NOTE: Since the OID was retreived through revwalk, this should not be None.
            self.repo.find_commit(oid).ok()
        } else {
            // The `since` commit has been reached.
            None
        }
    }
}

/// A manifest in a bucket.
pub struct SearchItem {
    /// The manifest's name.
    pub name: String,

    /// The parsed manifest.
    pub manifest: Result<Manifest>,
}

pub struct Search<P>
where
    P: Fn(&str) -> bool,
{
    entries: fs::ReadDir,
    predicate: P,
}

impl<P> Search<P>
where
    P: Fn(&str) -> bool,
{
    fn new<PATH>(dir: PATH, predicate: P) -> Result<Self>
    where
        PATH: AsRef<Path>,
    {
        let entries = fs::read_dir(dir)?;

        Ok(Self { entries, predicate })
    }
}

impl<P> Iterator for Search<P>
where
    P: Fn(&str) -> bool,
{
    type Item = SearchItem;

    fn next(&mut self) -> Option<Self::Item> {
        self.entries.find_map(|res| {
            let path = res.ok()?.path();

            let ext = path.extension().unwrap_or_default();

            // If the path does not end in '.json', it is not a manifest.
            if ext != "json" {
                return None;
            }

            // Take only the file stem (i.e., 'example' for 'example.json')
            let name = util::osstr_to_string(path.file_stem().unwrap());

            // If the predicate does not match, the manifest is skipped.
            if !(self.predicate)(&name) {
                return None;
            }

            Some(SearchItem {
                name,
                manifest: manifest_from_file(path),
            })
        })
    }
}

pub struct SearchAll<P>
where
    P: Fn(&str) -> bool + Copy,
{
    inner: Flatten<vec::IntoIter<Zip<Repeat<Arc<Bucket>>, Search<P>>>>,
}

impl<P> SearchAll<P>
where
    P: Fn(&str) -> bool + Copy,
{
    fn new<I>(buckets: I, predicate: P) -> Result<Self>
    where
        I: Iterator<Item = Bucket>,
    {
        let manifests: Result<Vec<_>> = buckets
            // Since each item has a handle to the bucket, the handle is wrapped in an Arc to avoid duplication.
            .map(move |bucket| {
                // Since each item has a handle to the bucket, the handle is wrapped in an Arc to avoid duplication.
                let bucket = Arc::new(bucket);
                let manifests = bucket.search(predicate)?;

                // Zip the bucket and its manifests together.
                Ok(iter::repeat(bucket).zip(manifests))
            })
            .collect();

        let manifests = manifests?.into_iter().flatten();

        Ok(Self { inner: manifests })
    }
}

impl<P> Iterator for SearchAll<P>
where
    P: Fn(&str) -> bool + Copy,
{
    type Item = (Arc<Bucket>, SearchItem);

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
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
}

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
        }
    }

    /// Yields all buckets.
    pub fn iter(&self) -> Result<Iter> {
        Iter::new(self.dir.clone())
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
    pub fn open(&self, name: &str) -> Result<Bucket> {
        let dir = self.path(name);

        // Make sure the bucket exists.
        if dir.try_exists()? {
            Bucket::open(dir)
        } else {
            Err(Error::BucketNotFound)
        }
    }

    /// Adds a bucket.
    ///
    /// # Arguments
    ///
    /// * `name` - The name to add the remote bucket as.
    /// * `url` - The Git URL of the remote bucket.
    /// * `builder` - The builder to clone with.
    ///
    /// # Errors
    ///
    /// If the bucket name already exists, `Error::BucketExists` is returned.
    pub fn add(
        &self,
        name: &str,
        url: &str,
        builder: Option<&mut build::RepoBuilder>,
    ) -> Result<Bucket> {
        let dir = self.path(name);

        if !dir.try_exists()? {
            let dir = self.dir.join(name);

            Bucket::clone(url, dir, builder)
        } else {
            Err(Error::BucketExists)
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
    pub fn remove(&self, name: &str) -> Result<()> {
        let dir = self.path(name);

        if dir.try_exists()? {
            fs::remove_dir_all(dir)?;

            Ok(())
        } else {
            Err(Error::BucketNotFound)
        }
    }

    /// Parses and yields each manifest in all buckets where filter(bucket) and predicate(manifest_name) returns true.
    ///
    /// # Arguments
    ///
    /// * `filter` - A filter function that determins if the bucket should be searched.
    /// * `predicate` - A predicate function that determines if the manifest should be yielded.
    pub fn search_all<F, P>(&self, filter: F, predicate: P) -> Result<SearchAll<P>>
    where
        F: Fn(&Bucket) -> bool,
        P: Fn(&str) -> bool + Copy,
    {
        let buckets: Result<Vec<_>> = self.iter()?.collect();

        let buckets = buckets?.into_iter().filter(filter);

        SearchAll::new(buckets, predicate)
    }

    /// Parses and yields each manifest in all buckets.
    /// This is a convenience function over `Self::search`.
    pub fn manifests(&self) -> Result<SearchAll<fn(&str) -> bool>> {
        self.search_all(|_| true, |_| true)
    }

    /// Parses and returns a single manifest in any bucket.
    ///
    /// # Arguments
    ///
    /// * `name`- The name of the manifest.
    pub fn manifest(&self, name: &str) -> Result<(Arc<Bucket>, SearchItem)> {
        let mut search = self.search_all(|_| true, |manifest_name| manifest_name == name)?;

        search.next().ok_or(Error::ManifestNotFound)
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
            bucket.url().unwrap(),
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

        let mut names_from_buckets: Vec<_> = buckets
            .iter()
            .unwrap()
            .map(|bucket| bucket.unwrap().name())
            .collect();
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
        let buckets = Buckets::new(buckets_dir());

        for (_, item) in buckets.manifests().unwrap() {
            item.manifest.unwrap();
        }
    }
}
