use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{BufReader, Result as IOResult};
use std::path::Path;

use serde::de::DeserializeOwned;
use serde_json;

use crate::error::Result;

/// Converts an OsStr to a String.
///
/// # Arguments
///
/// * `osstr` - The OsStr to convert.
pub fn osstr_to_string(osstr: &OsStr) -> String {
    osstr.to_str().unwrap().to_owned()
}

/// Deserialize a type `T` from a JSON file.
///
/// # Arguments
///
/// * `path` - The path to the JSON file.
pub fn json_from_file<P, T>(path: P) -> Result<T>
where
    P: AsRef<Path>,
    T: DeserializeOwned,
{
    let file = File::open(path)?;

    let reader = BufReader::new(file);
    let value_t = serde_json::from_reader(reader)?;

    Ok(value_t)
}

/// List all directories in a path by their final component.
///
/// # Arguments
///
/// * `path` - The path.
pub fn list_dir<P>(path: P) -> Result<impl Iterator<Item = String>>
where
    P: AsRef<Path>,
{
    // Collect the first error.
    let entries: IOResult<Vec<_>> = fs::read_dir(path)?.collect();

    Ok(entries?
        .into_iter()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .map(|p| osstr_to_string(p.file_name().unwrap())))
}
