use std::io;

/// A catch-all Shovel error.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A bucket does not exist.
    #[error("Bucket not found")]
    BucketNotFound,

    /// A bucket exists.
    #[error("Bucket already exists")]
    BucketExists,

    /// An app (version) does not exist.
    #[error("App {name:?} with version {version:?} not found")]
    AppNotFound { name: String, version: String },

    /// An app manifest does not exist.
    #[error("Manifest not found at {path}")]
    ManifestNotFound { path: String },

    /// An app's metadata does not exist.
    #[error("Metadata not found at {path}")]
    MetadataNotFound { path: String },

    /// An underlying error with serde_json.
    #[error(transparent)]
    JSON(#[from] serde_json::Error),

    /// An underlying error with std::io.
    #[error(transparent)]
    IO(#[from] io::Error),

    /// An underlying error with Git.
    #[error(transparent)]
    Git(#[from] git2::Error),
}

/// A Shovel result.
pub type Result<T> = std::result::Result<T, Error>;
