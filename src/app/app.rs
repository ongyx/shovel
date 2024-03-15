use std::io;
use std::path;

use crate::app::Metadata;
use crate::error::{Error, Result};
use crate::manifest::Manifest;
use crate::timestamp::Timestamp;
use crate::util::{json_from_file, mod_time};

/// An installed app in a directory.
///
/// The directory must contain these two files:
/// * `manifest.json` - The app's manifest at the time of installation.
/// * `install.json` - The app's install metadata, describing its architecture type and the bucket it came from.
pub struct App {
    dir: path::PathBuf,
}

impl App {
    /// Opens an existing app.
    ///
    /// # Arguments
    ///
    /// * `dir` - The path to the app. It must point to a directory.
    pub fn open<P>(dir: P) -> Self
    where
        P: AsRef<path::Path>,
    {
        Self {
            dir: dir.as_ref().to_owned(),
        }
    }

    /// Returns the app directory.
    pub fn dir(&self) -> &path::Path {
        &self.dir
    }

    /// Returns the last modified time of the app as a UNIX timestamp.
    pub fn timestamp(&self) -> Result<Timestamp> {
        mod_time(self.dir())
    }

    /// Returns the path to the app's manifest, or None if it does not exist.
    pub fn manifest_path(&self) -> Option<path::PathBuf> {
        let path = self.dir().join("manifest.json");

        if path.exists() {
            Some(path)
        } else {
            None
        }
    }

    /// Parses and returns the app's manifest.
    ///
    /// # Errors
    ///
    /// If the manifest file does not exist, `Error::ManifestNotFound` is returned.
    pub fn manifest(&self) -> Result<Manifest> {
        let path = self.manifest_path().ok_or(Error::ManifestNotFound)?;

        json_from_file(path)
    }

    /// Returns the path to the app's metadata.
    pub fn metadata_path(&self) -> path::PathBuf {
        self.dir().join("install.json")
    }

    /// Parses and returns the app's metadata.
    ///
    /// # Errors
    ///
    /// If the metadata file does not exist, `Error::MetadataNotFound` is returned.
    pub fn metadata(&self) -> Result<Metadata> {
        let path = self.metadata_path();

        json_from_file(&path).map_err(|err| match err {
            Error::IO(ioerr) if ioerr.kind() == io::ErrorKind::NotFound => Error::MetadataNotFound,
            _ => err,
        })
    }
}
