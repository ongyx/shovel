use std::fmt;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::sync::OnceLock;

use regex;

use crate::util;

fn re_invalid() -> &'static regex::Regex {
	static RE_INVALID: OnceLock<regex::Regex> = OnceLock::new();

	RE_INVALID.get_or_init(|| regex::Regex::new(r"[^\w\.\-]+").unwrap())
}

/// A cache key.
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

impl TryFrom<String> for Key {
	type Error = InvalidKey;

	fn try_from(value: String) -> Result<Self, Self::Error> {
		let parts: Vec<_> = value.split("#").collect();
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
	fn new<P>(dir: P) -> io::Result<Self>
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

	/// Yields the keys inside the cache.
	pub fn iter(&self) -> io::Result<Iter> {
		Iter::new(&self.dir)
	}

	/// Removes all cached files for a specific app.
	///
	/// # Arguments
	///
	/// * `app` - The app's name.
	pub fn remove(&self, app: &str) -> io::Result<()> {
		for entry in fs::read_dir(&self.dir)? {
			let path = entry?.path();
			// SAFETY: Entries should never end in '..'.
			let name = path.file_name().unwrap().to_str().unwrap();

			if name.split("#").next() == Some(app) {
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
