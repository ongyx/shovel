use std::io;

use git2;
use serde_json;
use serde_path_to_error;

/// A catch-all Shovel error.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A bucket does not exist.
    #[error("Bucket not found")]
    BucketNotFound,

    /// A bucket exists.
    #[error("Bucket already exists")]
    BucketExists,

    /// An app version does not exist.
    #[error("App version not found")]
    AppVersionNotFound,

    /// An app manifest does not exist.
    #[error("Manifest not found")]
    ManifestNotFound,

    /// An app's metadata does not exist.
    #[error("Metadata not found")]
    MetadataNotFound,

    /// An underlying error with serde_json.
    #[error(transparent)]
    JSON(#[from] serde_path_to_error::Error<serde_json::Error>),

    /// An underlying error with std::io.
    #[error(transparent)]
    IO(#[from] io::Error),

    /// An underlying error with Git.
    #[error(transparent)]
    Git(#[from] git2::Error),
}

/// A Shovel result.
pub type Result<T> = std::result::Result<T, Error>;
