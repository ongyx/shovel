use std::ffi;
use std::fs;
use std::io;
use std::iter;
use std::path::Path;
use std::path::PathBuf;
use std::time;

use crate::timestamp::Timestamp;

/// Converts an OsStr to a String.
///
/// # Arguments
///
/// * `osstr` - The OsStr to convert.
pub fn osstr_to_string(osstr: &ffi::OsStr) -> String {
	osstr.to_str().unwrap().to_owned()
}

/// An iterator over directories in a path. Created by the `subdirs` function.
pub struct Dirs {
	inner: iter::FilterMap<fs::ReadDir, fn(io::Result<fs::DirEntry>) -> Option<PathBuf>>,
}

impl Iterator for Dirs {
	type Item = PathBuf;

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
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
	fn entry_to_path(result: io::Result<fs::DirEntry>) -> Option<PathBuf> {
		let path = result.ok()?.path();

		if path.is_dir() {
			Some(path)
		} else {
			None
		}
	}

	Ok(Dirs {
		inner: fs::read_dir(path)?.filter_map(entry_to_path),
	})
}

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
