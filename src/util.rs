use std::ffi::OsStr;

/// Converts an OsStr to a String.
///
/// # Arguments
///
/// * `osstr` - The OsStr to convert.
pub fn osstr_to_string(osstr: &OsStr) -> String {
    osstr.to_str().unwrap().to_owned()
}
