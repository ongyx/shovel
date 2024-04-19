use std::ffi::OsStr;
use std::fs;
use std::io;
use std::path::Path;
use std::path::PathBuf;
use std::time;

use crate::timestamp::Timestamp;

/// A URL error.
#[derive(Debug, thiserror::Error)]
pub enum UrlError {
	/// A URL failed to parse.
	#[error(transparent)]
	Parse(#[from] url::ParseError),

	/// A URL does not have a filename.
	#[error("Filename not found")]
	FilenameNotFound,
}

/// A URL result.
pub type UrlResult<T> = std::result::Result<T, UrlError>;

/// Converts an OsStr to a string.
/// This is a lossy operation if the OsStr has characters not encoded in UTF-8.
///
/// # Arguments
///
/// * `osstr` - The OsStr to convert.
#[inline]
pub fn osstr_to_string(osstr: &OsStr) -> String {
	osstr.to_string_lossy().into_owned()
}

/// Converts a path to a string.
/// This is a lossy operation if the path has characters not encoded in UTF-8.
///
/// # Arguments
///
/// * `path` - The path to convert.
#[inline]
pub fn path_to_string<P>(path: P) -> String
where
	P: AsRef<Path>,
{
	osstr_to_string(path.as_ref().as_os_str())
}

/// An iterator over directories in a path. Created by the `subdirs` function.
pub struct Dirs {
	read_dir: fs::ReadDir,
}

impl Iterator for Dirs {
	type Item = PathBuf;

	fn next(&mut self) -> Option<Self::Item> {
		self.read_dir.find_map(|res| {
			let path = res.ok()?.path();

			if path.is_dir() {
				Some(path)
			} else {
				None
			}
		})
	}
}

/// Yields directories in a path.
///
/// # Arguments
///
/// * `path` - The path.
pub fn dirs<P>(path: P) -> io::Result<Dirs>
where
	P: AsRef<Path>,
{
	Ok(Dirs {
		read_dir: fs::read_dir(path)?,
	})
}

/// Returns the modification time of a path as a timestamp.
///
/// # Arguments
///
/// `path` - The path to get the timestamp for.
pub fn mod_time<P>(path: P) -> io::Result<Timestamp>
where
	P: AsRef<Path>,
{
	let timestamp = path
		.as_ref()
		.metadata()?
		.modified()?
		// https://doc.rust-lang.org/std/time/struct.SystemTime.html#associatedconstant.UNIX_EPOCH
		.duration_since(time::UNIX_EPOCH)
		.unwrap()
		.as_secs();

	Ok(Timestamp(timestamp as i64))
}

/// Returns the filename of a URL. The URL must be absolute.
///
/// If the URL has a fragment starting with '/', it is used as the filename. Otherwise, the last path segment is used.
///
/// # Arguments
///
/// * `url` - The URL to get the filename for.
pub fn url_to_filename(url: &str) -> UrlResult<String> {
	let url = url::Url::parse(url)?;

	let filename = url
		.fragment()
		// If there is a URL fragment starting with '/', return it.
		// i.e. https://example.test/original.txt#/renamed.txt returns renamed.txt instead of original.txt.
		.and_then(|f| f.strip_prefix('/'))
		// Otherwise, return the last path segment, if any.
		.or_else(|| url.path_segments()?.last());

	let filename = filename.ok_or(UrlError::FilenameNotFound)?;

	Ok(filename.to_owned())
}
