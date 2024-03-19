use std::path;
use std::sync;

use jsonschema;

use crate::json;

static TESTDIR: sync::OnceLock<path::PathBuf> = sync::OnceLock::new();

static SCHEMA: sync::OnceLock<jsonschema::JSONSchema> = sync::OnceLock::new();

/// Returns the path to the test data directory.
pub fn testdir() -> &'static path::Path {
    TESTDIR.get_or_init(|| path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata")))
}

/// Returns the JSON schema for verifying manfiests.
pub fn schema() -> &'static jsonschema::JSONSchema {
    SCHEMA.get_or_init(|| {
        let value = json::from_file(testdir().join("schema.json")).unwrap();

        jsonschema::JSONSchema::compile(&value).unwrap()
    })
}
