use std::ffi;
use std::fs;
use std::io;
use std::path;
use std::time;

use serde::de;
use serde_json;
use serde_path_to_error;

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

/// Deserialize a type `T` from a JSON file.
///
/// # Arguments
///
/// * `path` - The path to the JSON file.
pub fn json_from_file<P, T>(path: P) -> Result<T>
where
    P: AsRef<path::Path>,
    T: de::DeserializeOwned,
{
    let file = fs::File::open(path)?;

    let reader = io::BufReader::new(file);
    let de = &mut serde_json::Deserializer::from_reader(reader);

    let value = serde_path_to_error::deserialize(de)?;

    Ok(value)
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
