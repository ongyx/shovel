use std::path;
use std::sync;

static TESTDIR: sync::OnceLock<path::PathBuf> = sync::OnceLock::new();

/// Returns the path to the test data directory.
pub fn testdir() -> &'static path::Path {
    TESTDIR.get_or_init(|| path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata")))
}
