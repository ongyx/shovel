use std::fmt;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use futures_util::StreamExt;
use regex;
use reqwest;
use thiserror;

use crate::util;

#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// An error from reqwest.
	#[error("Failed to add URL to cache: {0}")]
	Reqwest(#[from] reqwest::Error),

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
	pub fn path(&self, key: &Key) -> PathBuf {
		self.dir.join(key.to_string())
	}

	/// Check if a key exists in the cache.
	///
	/// # Arguments
	///
	/// * `key`: The key to check.
	pub fn exists(&self, key: &Key) -> Result<bool> {
		let exists = self.path(key).try_exists()?;

		Ok(exists)
	}

	/// Yields the keys inside the cache.
	pub fn iter(&self) -> Result<Iter> {
		Iter::new(&self.dir)
	}

	/// Returns statistics on the cache.
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
	/// * `client`: The HTTP client to use.
	/// * `key`: The key to add.
	pub async fn add<P>(
		&self,
		client: reqwest::Client,
		key: &Key,
		progress: Option<P>,
	) -> Result<(bool, PathBuf)>
	where
		P: Fn(&Key, u64, u64),
	{
		let path = self.path(key);

		if self.exists(key)? {
			// The file is cached.
			return Ok((true, path));
		}

		let resp = client.get(&key.url).send().await?;

		let mut current = 0u64;
		let total = resp.content_length().unwrap();

		let mut stream = resp.bytes_stream();

		let mut file = fs::File::open(self.path(key))?;

		while let Some(chunk) = stream.next().await {
			let chunk = chunk?;

			file.write_all(&chunk)?;

			current += chunk.len() as u64;

			if let Some(ref progress) = progress {
				progress(key, current, total);
			}
		}

		Ok((false, path))
	}

	/// Removes all cached files for a specific app.
	///
	/// # Arguments
	///
	/// * `app` - The app's name.
	pub fn remove(&self, app: &str) -> Result<()> {
		for entry in fs::read_dir(&self.dir)? {
			let path = entry?.path();
			// SAFETY: Entries should never end in '..'.
			let name = path.file_name().unwrap().to_str().unwrap();

			if name.split('#').next() == Some(app) {
				fs::remove_file(&path)?;
			}
		}

		Ok(())
	}

	/// Removes all cached files.
	pub fn remove_all(&self) -> io::Result<()> {
		// Remove the directory and create it again.
		fs::remove_dir_all(&self.dir).and_then(|_| fs::create_dir(&self.dir))?;

		Ok(())
	}
}
