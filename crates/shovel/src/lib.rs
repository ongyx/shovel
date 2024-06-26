//! The Shovel package manager as a library.
//! Documentation and API stability are on a best-effort basis.

pub mod app;
pub mod bucket;
pub mod cache;
pub mod config;
pub mod download;
pub mod error;
pub mod hook;
pub mod json;
pub mod manifest;
pub mod persist;
pub mod shovel;
pub mod timestamp;

#[cfg(test)]
mod test;
mod util;

pub use config::Config;
pub use error::Error;
pub use error::Result;
pub use shovel::CatOptions;
pub use shovel::Shovel;
pub use shovel::UpdateOptions;
pub use timestamp::Timestamp;
