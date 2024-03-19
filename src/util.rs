use std::ffi;
use std::fs;
use std::path;
use std::time;

use crate::error::Result;
use crate::timestamp::Timestamp;

/// Converts an OsStr to a String.
///
/// # Arguments
///
/// * `osstr` - The OsStr to convert.
pub fn osstr_to_string(osstr: &ffi::OsStr) -> String {
    osstr.to_str().unwrap().to_owned()
}

/// Yields directories in a path.
///
/// # Arguments
///
/// * `path` - The path.
pub fn subdirs<P>(path: P) -> Result<impl Iterator<Item = String>>
where
    P: AsRef<path::Path>,
{
    let dirs = fs::read_dir(path)?
        // Discard errors.
        .filter_map(|res| res.ok())
        .filter_map(|entry| {
            let path = entry.path();

            if path.is_dir() {
                let name = path.file_name().unwrap();

                Some(osstr_to_string(name))
            } else {
                None
            }
        });

    Ok(dirs)
}

/// Returns the modification time of a path as a UNIX timestamp.
pub fn mod_time<P>(path: P) -> Result<Timestamp>
where
    P: AsRef<path::Path>,
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
