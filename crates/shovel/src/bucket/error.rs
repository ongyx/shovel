use std::io;

use crate::json;

/// A bucket error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// A bucket does not exist.
	#[error("Bucket not found")]
	NotFound,

	/// A bucket exists.
	#[error("Bucket already exists")]
	Exists,

	/// A manifest does not exist.
	#[error("Manifest not found")]
	ManifestNotFound,

	/// An IO error occurred.
	#[error(transparent)]
	Io(#[from] io::Error),

	/// A JSON (de)serialization error occurred.
	#[error(transparent)]
	Json(#[from] json::Error),

	/// An underlying error with Git.
	#[error(transparent)]
	Git(#[from] git2::Error),
}

/// A bucket result.
pub type Result<T> = std::result::Result<T, Error>;
