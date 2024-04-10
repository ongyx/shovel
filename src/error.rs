use std::io;

use crate::app;
use crate::bucket;
use crate::cache;
use crate::json;
use crate::manifest;

/// A catch-all error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	// An app error.
	#[error(transparent)]
	App(#[from] app::Error),

	// A bucket error.
	#[error(transparent)]
	Bucket(#[from] bucket::Error),

	// A cache error.
	#[error(transparent)]
	Cache(#[from] cache::Error),

	// A manifest error.
	#[error(transparent)]
	Manifest(#[from] manifest::Error),

	// An IO error.
	#[error(transparent)]
	Io(#[from] io::Error),

	// A JSON error.
	#[error(transparent)]
	Json(#[from] json::Error),
}

/// A catch-all result.
pub type Result<T> = std::result::Result<T, Error>;
