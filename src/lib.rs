//! A package manager for Windows, based off of [Scoop].
//!
//! This library exposes Scoop functions independently of the command-line interface.
//! Documentation and API stability are on a best-effort basis.
//!
//! [Scoop]: https://github.com/ScoopInstaller/Scoop

pub mod app;
pub mod bucket;
pub mod cache;
pub mod config;
pub mod error;
pub mod json;
pub mod manifest;
pub mod shovel;
pub mod timestamp;

#[cfg(test)]
mod test;
mod util;

pub use app::{App, Apps};
pub use bucket::{Bucket, Buckets};
pub use cache::Cache;
pub use config::Config;
pub use error::{Error, Result};
pub use manifest::Manifest;
pub use shovel::Shovel;
pub use timestamp::Timestamp;
