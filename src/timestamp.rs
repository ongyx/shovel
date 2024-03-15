use std::fmt;

use chrono;
use git2;

/// A UNIX timestamp in seconds.
#[derive(Clone, Copy, Debug)]
pub struct Timestamp(pub i64);

impl fmt::Display for Timestamp {
    /// Display the UNIX timestamp in human-readable local time.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let timestamp = chrono::DateTime::from_timestamp(self.0, 0)
            .unwrap()
            .with_timezone(&chrono::Local)
            .format("%d/%m/%Y %H:%M:%S %P");

        write!(f, "{}", timestamp)
    }
}

impl From<git2::Time> for Timestamp {
    fn from(time: git2::Time) -> Self {
        Self(time.seconds())
    }
}
