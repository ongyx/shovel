use std::io;
use std::path::{Path, PathBuf};

use crate::error::{Error, Result};
use crate::json;
use crate::manifest::Arch;
use crate::manifest::Manifest;
use crate::timestamp::Timestamp;
use crate::util;

json::json_struct! {
    /// Metadata on an installed app.
    pub struct Metadata {
        /// The app's architecture.
        pub architecture: Arch,

        /// The bucket the app originated from.
        pub bucket: String,
    }
}

/// An installed app in a directory.
///
/// The directory must contain these two files:
/// * `manifest.json` - The app's manifest at the time of installation.
/// * `install.json` - The app's install metadata, describing its architecture type and the bucket it came from.
pub struct App {
    dir: PathBuf,
}

impl App {
    /// Opens an existing app.
    ///
    /// # Arguments
    ///
    /// * `dir` - The path to the app. It must point to a directory.
    pub fn open<P>(dir: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            dir: dir.as_ref().to_owned(),
        }
    }

    /// Returns the app directory.
    pub fn dir(&self) -> &Path {
        &self.dir
    }

    /// Returns the last modified time of the app as a UNIX timestamp.
    pub fn timestamp(&self) -> Result<Timestamp> {
        util::mod_time(self.dir())
    }

    /// Returns the path to the app's manifest, or None if it does not exist.
    pub fn manifest_path(&self) -> Option<PathBuf> {
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

        json::from_file(path)
    }

    /// Returns the path to the app's metadata.
    pub fn metadata_path(&self) -> PathBuf {
        self.dir().join("install.json")
    }

    /// Parses and returns the app's metadata.
    ///
    /// # Errors
    ///
    /// If the metadata file does not exist, `Error::MetadataNotFound` is returned.
    pub fn metadata(&self) -> Result<Metadata> {
        let path = self.metadata_path();

        json::from_file(&path).map_err(|err| match err {
            Error::Io(ioerr) if ioerr.kind() == io::ErrorKind::NotFound => Error::MetadataNotFound,
            _ => err,
        })
    }
}

/// An app manager.
///
/// Installed apps are stored as sub-directories with several versions,
/// as well as a `current` directory junction or symbolic link to the version in use,
/// typically the latest version:
/// * `dir`
///   * `app1`
///     * `1.0.0`
///     * `current` -> `1.0.0`
///   * `app2`
///     * `0.1.0`
///     * `0.2.0`
///     * `current` -> `0.2.0`
///   * `...`
pub struct Apps {
    dir: PathBuf,
}

impl Apps {
    /// Returns a new app manager.
    ///
    /// # Arguments
    ///
    /// * `dir` - The directory where apps are stored.
    pub fn new<P>(dir: P) -> Self
    where
        P: AsRef<Path>,
    {
        Self {
            dir: dir.as_ref().to_owned(),
        }
    }

    /// Yields the current version of each app.
    pub fn iter(&self) -> Result<impl Iterator<Item = (String, Result<App>)>> {
        let apps = util::subdirs(self.dir.clone())?.map(|mut dir| {
            let name = util::osstr_to_string(dir.file_name().unwrap());

            dir.push("current");

            // The 'current' directory may not exist if the app is corrupted.
            let app = dir
                .try_exists()
                .map_err(|err| Error::from(err))
                .and_then(|exists| {
                    if exists {
                        Ok(App::open(dir))
                    } else {
                        Err(Error::AppNotFound)
                    }
                });

            (name, app)
        });

        Ok(apps)
    }

    /// Yields the installed versions for an app. This does not include 'current'.
    pub fn versions(&self, name: &str) -> Result<impl Iterator<Item = App>> {
        let path = self.dir.join(name);

        let versions = util::subdirs(path)?.filter_map(|dir| {
            if dir.file_name().unwrap() != "current" {
                Some(App::open(dir))
            } else {
                None
            }
        });

        Ok(versions)
    }

    /// Returns the path to an app.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the app.
    /// * `version` - The version of the app.
    pub fn path(&self, name: &str, version: &str) -> PathBuf {
        let mut dir = self.dir.to_owned();
        dir.extend([name, version]);

        dir
    }

    /// Opens and returns an app.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the app.
    /// * `version` - The version of the app.
    ///
    /// # Errors
    ///
    /// If the app does not exist, `Error::AppNotFound` is returned.
    pub fn open(&self, name: &str, version: &str) -> Result<App> {
        let dir = self.path(name, version);

        if dir.try_exists()? {
            Ok(App::open(dir))
        } else {
            Err(Error::AppNotFound)
        }
    }

    /// Opens and returns an app's current version.
    /// This is a convenience function for `get(name, "current")`.
    pub fn open_current(&self, name: &str) -> Result<App> {
        self.open(name, "current")
    }
}
