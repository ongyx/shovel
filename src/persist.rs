use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;

/// A data persistence manager.
///
/// When app versions are upgraded, user data may need to be persisted.
/// The files/directories to persist are:
/// * Stored in the persist directory.
/// * Symlinked/junctioned into the app version.
/// * Kept across new versions.
pub struct Persist {
	dir: PathBuf,
}

impl Persist {
	/// Returns a new data persistence manager.
	pub fn new<P>(dir: P) -> Self
	where
		P: AsRef<Path>,
	{
		Self {
			dir: dir.as_ref().to_owned(),
		}
	}

	/// Creates an app's persistence directory.
	/// If it already exists, this is a no-op.
	///
	/// # Arguments
	///
	/// * `name` - The app's name.
	///
	/// # Errors
	///
	/// If the directory could not be created, the IO error is returned.
	pub fn add(&self, name: &str) -> io::Result<PathBuf> {
		let path = self.path(name);

		fs::create_dir_all(&path)?;

		Ok(path)
	}

	/// Removes an app's persistence directory.
	///
	/// # Arguments
	///
	/// * `name` - The app's name.
	///
	/// # Errors
	///
	/// If the directory could not be removed, the IO error is returned.
	pub fn remove(&self, name: &str) -> io::Result<()> {
		fs::remove_dir_all(self.path(name))
	}

	/// Returns the path to an app's persistence directory.
	///
	/// # Arguments
	///
	/// * `name` - The app's name.
	#[must_use]
	pub fn path(&self, name: &str) -> PathBuf {
		self.dir.join(name)
	}
}
