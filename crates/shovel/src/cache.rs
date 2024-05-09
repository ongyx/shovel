use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use futures_util::future;
use regex;
use thiserror;

use crate::download::Download;
use crate::download::Error as DownloadError;
use crate::download::Progress;
use crate::util;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// An error from the downloader.
	#[error("Failed to add URL to cache: {0}")]
	Download(#[from] DownloadError),

	/// An IO error.
	#[error("IO error: {0}")]
	Io(#[from] io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

fn re_invalid() -> &'static regex::Regex {
	static RE_INVALID: OnceLock<regex::Regex> = OnceLock::new();

	RE_INVALID.get_or_init(|| regex::Regex::new(r"[^\w\.\-]+").unwrap())
}

/// A cache key, representing a downloaded file in the cache.
pub struct Key {
	/// The app's name.
	pub name: String,

	/// The app's version.
	pub version: String,

	/// A URL belonging to the app.
	pub url: String,
}

impl fmt::Display for Key {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		// Replace all invalid characters with an underscore.
		let url = re_invalid().replace_all(&self.url, "_");

		// The cache path consists of '(name)#(version)#(url)'.
		write!(f, "{}#{}#{}", self.name, self.version, url)
	}
}

/// The error returned when a cache key failed to parse.
pub struct InvalidKey;

impl fmt::Display for InvalidKey {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		write!(f, "Cache key is invalid")
	}
}

impl TryFrom<String> for Key {
	type Error = InvalidKey;

	fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
		let parts: Vec<_> = value.split('#').collect();
		if parts.len() != 3 {
			return Err(InvalidKey);
		}

		Ok(Self {
			name: parts[0].to_owned(),
			version: parts[1].to_owned(),
			url: parts[2].to_owned(),
		})
	}
}

/// An iterator over keys in a cache.
pub struct Iter {
	inner: fs::ReadDir,
}

impl Iter {
	fn new<P>(dir: P) -> Result<Self>
	where
		P: AsRef<Path>,
	{
		let inner = fs::read_dir(dir)?;

		Ok(Self { inner })
	}
}

impl Iterator for Iter {
	type Item = Key;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.find_map(|res| {
			let path = res.ok()?.path();
			let name = util::osstr_to_string(path.file_name().unwrap());
			let key = Key::try_from(name).ok()?;

			Some(key)
		})
	}
}

/// Cache stats.
pub struct Stat {
	/// The number of keys.
	pub count: usize,

	/// The size of all files.
	pub length: u64,
}

/// A cache for URL downloads, keyed by app and version.
pub struct Cache {
	dir: PathBuf,
}

impl Cache {
	/// Creates a new cache.
	pub fn new<P>(dir: P) -> Cache
	where
		P: AsRef<Path>,
	{
		Cache {
			dir: dir.as_ref().to_owned(),
		}
	}

	/// Returns the cache path for an app.
	///
	/// # Arguments
	///
	/// * `name` - The app's name.
	/// * `version` - The app's version.
	/// * `url` - The app's URL.
	#[must_use]
	pub fn path(&self, key: &Key) -> PathBuf {
		self.dir.join(key.to_string())
	}

	/// Check if a key exists in the cache.
	///
	/// # Arguments
	///
	/// * `key`: The key to check.
	///
	/// # Errors
	///
	/// If the cached file cannot be read, [`Error::Io`] is returned.
	pub fn exists(&self, key: &Key) -> Result<bool> {
		let exists = self.path(key).try_exists()?;

		Ok(exists)
	}

	/// Yields the keys inside the cache.
	///
	/// # Errors
	///
	/// If the cache directory cannot be read, [`Error::Io`] is returned.
	pub fn iter(&self) -> Result<Iter> {
		Iter::new(&self.dir)
	}

	/// Returns statistics on the cache.
	///
	/// # Errors
	///
	/// If the cache directory or any cached file cannot be read, [`Error::Io`] is returned.
	pub fn stat(&self) -> Result<Stat> {
		let lengths: io::Result<Vec<_>> = self
			.iter()?
			.map(|key| {
				let metadata = self.path(&key).metadata()?;

				Ok(metadata.len())
			})
			.collect();

		let lengths = lengths?;

		Ok(Stat {
			count: lengths.len(),
			length: lengths.iter().sum(),
		})
	}

	/// Adds a key to the cache by downloading its URL, returning a 2-tuple (cached, path).
	/// cached is `true` if the URL has already been cached, otherwise `false.`
	///
	/// # Arguments
	///
	/// * `key`: The key to add.
	/// * `download`: The downloader to use.
	///
	/// # Errors
	///
	/// [`Error::Io`] is returned if the cached file cannot be read, created, or written to.
	///
	/// [`Error::Download`] is returned if the URL cannot be downloaded.
	pub async fn add<P: Progress>(
		&self,
		key: Key,
		download: &Download<P>,
	) -> Result<(bool, PathBuf)> {
		let path = self.path(&key);

		if self.exists(&key)? {
			// The file is cached.
			return Ok((true, path));
		}

		download.download_to_path(&key.url, &path).await?;

		Ok((false, path))
	}

	/// Add multiple keys to the cache and returns a Vec of 2-tuples (cached, path).
	///
	/// # Arguments
	///
	/// * `keys` - An iterator over keys to add.
	/// * `download`: The downloader to use.
	///
	/// # Errors
	///
	/// See [`add`].
	///
	/// [`add`]: Cache::add
	pub async fn add_multiple<I, P>(
		&self,
		keys: I,
		download: &Download<P>,
	) -> Result<Vec<(bool, PathBuf)>>
	where
		I: IntoIterator<Item = Key>,
		P: Progress,
	{
		let futures = keys.into_iter().map(|k| self.add(k, download));

		let downloaded = future::try_join_all(futures).await?;

		Ok(downloaded)
	}

	/// Removes all cached files for a specific app.
	///
	/// # Arguments
	///
	/// * `app` - The app's name.
	///
	/// # Errors
	///
	/// If any cached file cannot be read, [`Error::Io`] is returned.
	pub fn remove(&self, app: &str) -> Result<()> {
		for entry in fs::read_dir(&self.dir)? {
			let path = entry?.path();
			let name = path.file_name().unwrap_or_default().to_string_lossy();

			if name.split('#').next() == Some(app) {
				fs::remove_file(&path)?;
			}
		}

		Ok(())
	}

	/// Removes all cached files.
	///
	/// # Errors
	///
	/// If any cached file cannot be read, [`Error::Io`] is returned.
	pub fn remove_all(&self) -> io::Result<()> {
		// Remove the directory and create it again.
		fs::remove_dir_all(&self.dir).and_then(|()| fs::create_dir(&self.dir))?;

		Ok(())
	}
}
