use std::io;

use crate::bucket::Error as BucketError;

/// A catch-all Shovel error.
#[derive(thiserror::Error, Debug)]
pub enum Error {
    /// A bucket-specific error.
    #[error(transparent)]
    Bucket(#[from] BucketError),

    /// An IO error.
    #[error(transparent)]
    IO(#[from] io::Error),
}

/// A Shovel result.
pub type Result<T> = std::result::Result<T, Error>;
