use std::fs;
use std::iter;
use std::iter::Flatten;
use std::iter::Repeat;
use std::iter::Zip;
use std::path::Path;
use std::path::PathBuf;
use std::rc::Rc;
use std::vec::IntoIter;

use git2::build;

use crate::bucket::Bucket;
use crate::bucket::Error;
use crate::bucket::Result;
use crate::bucket::Search;
use crate::bucket::SearchItem;
use crate::util;

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
	///
	/// # Errors
	///
	/// If the directory containing the buckets cannot be read, [`Error::Io`] is returned.
	pub fn iter(&self) -> Result<Iter> {
		Iter::new(self.dir.clone())
	}

	/// Returns the path to a bucket.
	#[must_use]
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
	/// If the bucket does not exist, `Error::NotFound` is returned.
	pub fn open(&self, name: &str) -> Result<Bucket> {
		let dir = self.path(name);

		// Make sure the bucket exists.
		if dir.try_exists()? {
			Bucket::open(dir)
		} else {
			Err(Error::NotFound)
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

		if dir.try_exists()? {
			Err(Error::Exists)
		} else {
			let dir = self.dir.join(name);

			Bucket::clone(url, dir, builder)
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
	/// If the bucket does not exist, `Error::NotFound` is returned.
	pub fn remove(&self, name: &str) -> Result<()> {
		let dir = self.path(name);

		if dir.try_exists()? {
			fs::remove_dir_all(dir)?;

			Ok(())
		} else {
			Err(Error::NotFound)
		}
	}

	/// Parses and yields each manifest in all buckets where `filter(bucket)` and `predicate(manifest_name)` returns true.
	///
	/// # Arguments
	///
	/// * `filter` - A filter function that determins if the bucket should be searched.
	/// * `predicate` - A predicate function that determines if the manifest should be yielded.
	///
	/// # Errors
	///
	/// If the directory containing the buckets or any bucket directory cannot be read, [`Error::Io`] is returned.
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
	/// This is a convenience function over [`search_all`].
	///
	/// [`search_all`]: Buckets::search_all
	///
	/// # Errors
	///
	/// If the directory containing the buckets or any bucket directory cannot be read, [`Error::Io`] is returned.
	pub fn manifests(&self) -> Result<AllManifests> {
		self.search_all(|_| true, |_| true)
	}

	/// Parses and returns a single manifest in any bucket.
	///
	/// # Arguments
	///
	/// * `name` - The name of the manifest.
	///
	/// # Errors
	///
	/// If the directory containing the buckets or any bucket directory cannot be read, [`Error::Io`] is returned.
	pub fn manifest(&self, name: &str) -> Result<(Rc<Bucket>, SearchItem)> {
		let mut search = self.search_all(|_| true, |manifest_name| manifest_name == name)?;

		search.next().ok_or(Error::ManifestNotFound)
	}
}

/// An iterator over buckets. Created by `Buckets::iter`.
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

type SearchAllInner<P> = Zip<Repeat<Rc<Bucket>>, Search<P>>;

/// An iterator over manifests in all buckets, filtered by a predicate of type `P`. Created by `Buckets::search_all`.
pub struct SearchAll<P>
where
	P: Fn(&str) -> bool + Copy,
{
	inner: Flatten<IntoIter<SearchAllInner<P>>>,
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
			// Since each item has a handle to the bucket, the handle is wrapped in an Rc to avoid duplication.
			.map(move |bucket| {
				// Since each item has a handle to the bucket, the handle is wrapped in an Rc to avoid duplication.
				let bucket = Rc::new(bucket);
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
	type Item = (Rc<Bucket>, SearchItem);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}

/// An iterator over all manifests in all buckets.
pub type AllManifests = SearchAll<fn(&str) -> bool>;

#[cfg(test)]
mod tests {
	use std::fs;

	use super::*;
	use crate::test;

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
