//! A package manager for Windows, based off of [Scoop].
//!
//! This library exposes Scoop functions independently of the command-line interface.
//! Documentation and API stability are on a best-effort basis.
//!
//! [Scoop]: https://github.com/ScoopInstaller/Scoop

#![forbid(unsafe_code)]
#![deny(clippy::all)]
#![warn(clippy::pedantic)]
#![allow(
	clippy::enum_glob_use,
	clippy::doc_markdown,
	clippy::module_name_repetitions,
	clippy::iter_not_returning_iterator
)]

pub mod app;
pub mod bucket;
pub mod cache;
pub mod config;
pub mod error;
pub mod hook;
pub mod json;
pub mod manifest;
pub mod persist;
pub mod powershell;
pub mod shovel;
pub mod timestamp;

#[cfg(test)]
mod test;
mod util;

pub use app::App;
pub use app::Apps;
pub use bucket::Bucket;
pub use bucket::Buckets;
pub use cache::Cache;
pub use config::Config;
pub use error::Error;
pub use error::Result;
pub use hook::Hook;
pub use manifest::Manifest;
pub use powershell::Powershell;
pub use shovel::Shovel;
pub use timestamp::Timestamp;
