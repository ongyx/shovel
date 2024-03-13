use std::path;

use crate::app::App;
use crate::error::{Error, Result};
use crate::util::list_dir;

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
    dir: path::PathBuf,
}

impl Apps {
    /// Returns a new app manager.
    ///
    /// # Arguments
    ///
    /// * `dir` - The directory where apps are stored.
    pub fn new<P>(dir: P) -> Self
    where
        P: AsRef<path::Path>,
    {
        Self {
            dir: dir.as_ref().to_owned(),
        }
    }

    /// Returns an iterator over all apps by name.
    pub fn iter(&self) -> Result<impl Iterator<Item = String>> {
        list_dir(self.dir.to_owned())
    }

    /// Returns an iterator over an app's versions. This does not include 'current'.
    pub fn versions(&self, name: &str) -> Result<impl Iterator<Item = String>> {
        let path = self.dir.join(name);

        Ok(list_dir(path)?.filter(|v| v != "current"))
    }

    /// Returns the path to an app, or None if it does not exist.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the app.
    /// * `version` - The version of the app.
    pub fn path(&self, name: &str, version: &str) -> Option<path::PathBuf> {
        let mut dir = self.dir.to_owned();
        dir.extend([name, version]);

        if dir.exists() {
            Some(dir)
        } else {
            None
        }
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
    /// If the app does not exist, `Error::AppVersionNotFound` is returned.
    pub fn get(&self, name: &str, version: &str) -> Result<App> {
        let dir = self.path(name, version).ok_or(Error::AppVersionNotFound)?;

        Ok(App::open(dir))
    }

    /// Opens and returns an app's current version.
    /// This is a convenience function for `get(name, "current")`.
    pub fn get_current(&self, name: &str) -> Result<App> {
        self.get(name, "current")
    }
}
