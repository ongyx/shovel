use std::ffi;
use std::fs;
use std::io;
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
