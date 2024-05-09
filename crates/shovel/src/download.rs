use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::io::BufWriter;
use std::path::Path;

use futures_util::StreamExt;

/// A download error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
	/// An error from reqwest.
	#[error("Failed to download URL: {0}")]
	Reqwest(#[from] reqwest::Error),

	/// An IO error.
	#[error("IO error: {0}")]
	Io(#[from] io::Error),
}

/// A download result.
pub type Result<T> = std::result::Result<T, Error>;

/// A streaming progress callback.
///
/// When a chunk is streamed, the [`update`] method is called.
///
/// [`update`]: crate::download::Download::Progress::update
pub trait Progress: Clone {
	/// Updates the download progress of the given URL.
	///
	/// # Arguments
	///
	/// * `url` - The URL being downloaded.
	/// * `current` - The current number of bytes downloaded.
	/// * `total` - The total number of bytes to download, or None if it cannot be determined.
	fn update(&self, url: &str, current: u64, total: Option<u64>);
}

impl<F> Progress for F
where
	F: Fn(&str, u64, Option<u64>) + Clone,
{
	fn update(&self, url: &str, current: u64, total: Option<u64>) {
		self(url, current, total);
	}
}

impl Progress for () {
	fn update(&self, _url: &str, _current: u64, _total: Option<u64>) {}
}

/// An asynchronous streaming downloader.
#[derive(Clone)]
pub struct Download<P: Progress> {
	client: reqwest::Client,
	progress: Option<P>,
}

impl<P: Progress> Download<P> {
	/// Creates a new downloader using the given HTTP client.
	#[must_use]
	pub fn new(client: reqwest::Client) -> Self {
		Self {
			client,
			progress: None,
		}
	}

	/// Sets the progress callback to use. Refer to [`Progress`] for details.
	///
	/// [`Progress`]: crate::download::Progress
	pub fn progress(&mut self, progress: P) -> &mut Self {
		self.progress = Some(progress);
		self
	}

	/// Downloads a URL by streaming it to a writer and returns the number of bytes written.
	///
	/// # Errors
	///
	/// If the HTTP request could not be sent or a chunk failed to be received, [`Error::Reqwest`] is returned.
	///
	/// If writing to the writer failed, [`Error::Io`] is returned.
	///
	/// [`Error::Reqwest`]: crate::download::Error::Reqwest
	/// [`Error::Io`]: crate::download::Error::Io
	pub async fn download<W: Write>(&self, url: &str, writer: W) -> Result<u64> {
		let mut buf = BufWriter::new(writer);

		let resp = self.client.get(url).send().await?;

		let mut current = 0u64;
		let total = resp.content_length();

		let mut stream = resp.bytes_stream();

		while let Some(chunk) = stream.next().await {
			let chunk = chunk?;

			buf.write_all(&chunk)?;

			current += chunk.len() as u64;

			if let Some(progress) = &self.progress {
				progress.update(url, current, total);
			}
		}

		buf.flush()?;

		Ok(current)
	}

	/// Downloads a URL to a file path and returns the number of bytes written.
	///
	/// # Errors
	///
	/// See [`download`].
	///
	/// [`download`]: crate::download::Download::download
	pub async fn download_to_path<PR: AsRef<Path>>(&self, url: &str, path: PR) -> Result<u64> {
		let file = File::create(path)?;

		self.download(url, file).await
	}
}

impl Default for Download<()> {
	fn default() -> Self {
		Self::new(reqwest::Client::new())
	}
}
