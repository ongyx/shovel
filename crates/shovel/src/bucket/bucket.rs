use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

use git2::build;

use crate::bucket::Criteria;
use crate::bucket::Error;
use crate::bucket::Result;
use crate::json;
use crate::manifest::Manifest;
use crate::util;

fn manifest_from_file<P>(path: P) -> Result<Manifest>
where
	P: AsRef<Path>,
{
	let file = fs::File::open(path).map_err(|err| match err.kind() {
		io::ErrorKind::NotFound => Error::ManifestNotFound,
		_ => err.into(),
	})?;

	let manifest = json::from_reader(file)?;

	Ok(manifest)
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
	///
	/// # Errors
	///
	/// If dir is not a valid Git repository, [`Error::Git`] is returned.
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
	///
	/// # Errors
	///
	/// If the remote bucket failed to clone, [`Error::Git`] is returned.
	pub fn clone<P>(url: &str, dir: P, builder: Option<&mut build::RepoBuilder>) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		let dir = dir.as_ref().to_owned();

		// A temporary builder is needed if the one passed in is None.
		let mut temp_builder = build::RepoBuilder::new();

		let builder = builder.unwrap_or(&mut temp_builder);
		let repo = builder.clone(url, &dir)?;

		Ok(Bucket { dir, repo })
	}

	/// Returns the bucket directory.
	#[must_use]
	pub fn dir(&self) -> &Path {
		&self.dir
	}

	/// Returns the bucket name.
	#[must_use]
	pub fn name(&self) -> String {
		// The file name is None if the directory is '..', which results in an empty string here.
		let name = self.dir().file_name().unwrap_or_default();

		name.to_string_lossy().into_owned()
	}

	/// Returns the bucket URL, i.e., where it was cloned from.
	///
	/// # Errors
	///
	/// If the bucket does not have a origin, [`Error::Git`] is returned.
	pub fn url(&self) -> Result<String> {
		Ok(self.origin()?.url().unwrap_or("").to_owned())
	}

	fn origin(&self) -> Result<git2::Remote> {
		let origin = self.repo.find_remote("origin")?;

		Ok(origin)
	}

	/// Returns the HEAD commit of the bucket.
	///
	/// # Errors
	///
	/// If the bucket does not have a HEAD or HEAD commit, [`Error::Git`] is returned.
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
	///
	/// # Errors
	///
	/// If the Git revwalk failed, [`Error::Git`] is returned.
	pub fn commits(&self, since: git2::Oid) -> Result<Commits<'_>> {
		Commits::new(&self.repo, since)
	}

	/// Parses and yields each manifest in the bucket as (name, manifest) where the criteria is satisfied.
	///
	/// # Arguments
	///
	/// `criteria` - The criteria that determines if a manifest should be yielded.
	///
	/// # Errors
	///
	/// If the bucket directory cannot be read, [`Error::Io`] is returned.
	pub fn search<C: Criteria>(&self, criteria: C) -> Result<Search<C>> {
		let dir = self.dir().join("bucket");

		Search::new(dir, criteria)
	}

	/// Parses and yields each manifest in the bucket as (name, manifest).
	/// This is a convenience function over `Self::search`.
	///
	/// # Errors
	///
	/// If the bucket directory cannot be read, [`Error::Io`] is returned.
	pub fn manifests(&self) -> Result<Manifests> {
		self.search(())
	}

	/// Returns the path to an manifest.
	#[must_use]
	pub fn manifest_path(&self, name: &str) -> PathBuf {
		self.dir().join(format!(r"bucket\{name}.json"))
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

		manifest_from_file(path)
	}

	/// Returns the last commit made to a manifest.
	/// If the manifest has not been commited, None is returned.
	///
	/// # Errors
	///
	/// If the status of the manifest file cannot be read, or the revwalk failed, [`Error::Git`] is returned.
	#[allow(clippy::missing_panics_doc)]
	pub fn manifest_commit(&self, name: &str) -> Result<Option<git2::Commit>> {
		let path = self.manifest_path(name);

		// Ensure the manifest exists.
		if !path.exists() {
			return Err(Error::ManifestNotFound);
		}

		// SAFETY: path is always a child of dir.
		let relpath = path
			.strip_prefix(&self.dir)
			.expect("manifest path is in bucket");

		// Ensure the manifest is commited.
		if self.is_commited(relpath)? {
			let commit = self.find_commit(relpath)?;

			Ok(Some(commit.expect("Manifest is commited")))
		} else {
			Ok(None)
		}
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
	///
	/// # Errors
	///
	/// [`Error::Git`] is returned when any of these occur:
	/// * The HEAD branch cannot be retrieved.
	/// * The bucket does not have an origin.
	/// * Fetching changes from the origin failed.
	/// * Fast-forwarding failed.
	///
	/// # Panics
	///
	/// This function panics if the HEAD branch has a non-UTF-8 name.
	pub fn pull(
		&mut self,
		fetch_options: Option<&mut git2::FetchOptions>,
		checkout_builder: Option<&mut build::CheckoutBuilder>,
	) -> Result<()> {
		// Get the name of the HEAD branch.
		let head = self.repo.head()?;
		let branch = head.name().unwrap();

		// Get the origin remote.
		// TODO: Don't hardcode?
		let mut origin = self.origin()?;

		// Fetch updates for the HEAD branch from the origin.
		let new_head = self.fetch(&[branch], &mut origin, fetch_options)?;

		// Attempt to fast-forward changes.
		self.fast_forward(branch, &new_head, checkout_builder)?;

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

/// An iterator over commits since a specific commit. Created by `Buckets::commits`.
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

		if oid == self.since {
			// The `since` commit has been reached.
			None
		} else {
			// Return the commit.
			// NOTE: Since the OID was retreived through revwalk, this should not be None.
			self.repo.find_commit(oid).ok()
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

/// An iterator over manifests in a bucket, filtered by criteria of type `C`. Created by `Buckets::search`.
pub struct Search<C: Criteria> {
	entries: fs::ReadDir,
	criteria: C,
}

impl<C: Criteria> Search<C> {
	fn new<P>(dir: P, criteria: C) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		let entries = fs::read_dir(dir)?;

		Ok(Self { entries, criteria })
	}
}

impl<C: Criteria> Iterator for Search<C> {
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
			if !self.criteria.filter_manifest(&name) {
				return None;
			}

			Some(SearchItem {
				name,
				manifest: manifest_from_file(path),
			})
		})
	}
}

/// An iterator over all manifests in a bucket.
pub type Manifests = Search<()>;

#[cfg(test)]
mod tests {
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
}
