use std::fs;
use std::io;
use std::io::prelude::*;
use std::vec;

use git2::build::CheckoutBuilder;
use git2::FetchOptions;
use rayon::prelude::*;

use crate::app::Apps;
use crate::bucket::Bucket;
use crate::bucket::Buckets;
use crate::cache::Cache;
use crate::config::Config;
use crate::error::Result;
use crate::persist::Persist;

/// A high-level interface to Shovel.
#[allow(dead_code)]
pub struct Shovel {
	/// The app manager.
	pub apps: Apps,

	/// The bucket manager.
	pub buckets: Buckets,

	/// The cache for storing app downloads.
	pub cache: Cache,

	/// The data persistence manager.
	pub persist: Persist,

	config: Config,
}

impl Shovel {
	/// Creates a new shovel.
	///
	/// # Arguments
	///
	/// * `config` - The config to use.
	///
	/// # Errors
	///
	/// If any config-related directory cannot be created, [`Error::Io`] is returned.
	///
	/// [`Error::Io`]: crate::error::Error::Io
	pub fn new(config: Config) -> Result<Self> {
		let install_dir = config.install_dir();
		let app_dir = config.app_dir();
		let bucket_dir = config.bucket_dir();
		let cache_dir = config.cache_dir();
		let persist_dir = config.persist_dir();

		// Ensure the installation directory, and all sub-directories, exist.
		for dir in [
			&install_dir,
			&app_dir,
			&bucket_dir,
			&cache_dir,
			&persist_dir,
		] {
			fs::create_dir_all(dir)?;
		}

		Ok(Shovel {
			apps: Apps::new(app_dir),
			buckets: Buckets::new(bucket_dir),
			cache: Cache::new(cache_dir),
			persist: Persist::new(persist_dir),
			config,
		})
	}

	/// Copies the contents of a manifest to a writer specified by `options`.
	/// If the manifest exists and was copied, `Ok(true)` is returned, otherwise `Ok(false)`.
	///
	/// # Arguments
	///
	/// * `options` - The cat options.
	///
	/// # Errors
	///
	/// [`Error::Bucket`] is returned if the manifest search failed.
	///
	/// [`Error::Io`] is returned if the manifest file cannot be opened or copied to the writer.
	///
	/// [`Error::Bucket`]: crate::error::Error::Bucket
	/// [`Error::Io`]: crate::error::Error::Io
	pub fn cat<W: Write>(&self, options: &mut CatOptions<W>) -> Result<bool> {
		let mut search = self.buckets.search_all(
			|bucket| {
				let name = bucket.name();

				options.bucket.is_none() || Some(name.as_str()) == options.bucket
			},
			|manifest| manifest == options.manifest,
		)?;

		if let Some((bucket, item)) = search.next() {
			let path = bucket.manifest_path(&item.name);
			let mut file = fs::File::open(path)?;

			io::copy(&mut file, &mut options.writer)?;

			Ok(true)
		} else {
			Ok(false)
		}
	}

	/// Updates all buckets by pulling new changes.
	///
	/// # Arguments
	///
	/// * `options` - The update options. To specify the defaults, use [`&UpdateOptions::Default`].
	///
	/// [`&UpdateOptions::Default`]: crate::shovel::UpdateOptions::default
	///
	/// # Errors
	///
	/// [`Error::Bucket`] is returned if:
	/// * A bucket cannot be read.
	/// * A bucket does not have a HEAD commit.
	/// * A bucket failed to pull.
	///
	/// [`Error::Bucket`]: crate::error::Error::Bucket
	pub fn update(&self, options: &UpdateOptions) -> Result<Updates> {
		let buckets = self.buckets.iter()?;

		let updated: Result<Vec<_>> = buckets
			.par_bridge()
			.map(|res| {
				let mut bucket = res?;

				// Save the original HEAD commit before pulling.
				let head = bucket.commit()?.id();

				let mut fetch_options = match &options.fetch_options {
					Some(factory) => factory(&bucket),
					None => FetchOptions::new(),
				};

				let mut checkout_builder = match &options.checkout_builder {
					Some(factory) => factory(&bucket),
					None => CheckoutBuilder::new(),
				};

				// According to the git2 pull example, not including this option causes the working directory to not update.
				checkout_builder.force();

				bucket.pull(Some(&mut fetch_options), Some(&mut checkout_builder))?;

				Ok((bucket, head))
			})
			.collect();

		Ok(Updates {
			inner: updated?.into_iter(),
		})
	}
}

type FetchOptionsFactory<'a> = dyn Fn(&Bucket) -> FetchOptions<'a> + Send + Sync + 'a;
type CheckoutBuilderFactory<'a> = dyn Fn(&Bucket) -> CheckoutBuilder<'a> + Send + Sync + 'a;

/// Options for updating buckets. See [`update`].
///
/// [`update`]: crate::shovel::Shovel::update
#[derive(Default)]
pub struct UpdateOptions<'a> {
	fetch_options: Option<Box<FetchOptionsFactory<'a>>>,
	checkout_builder: Option<Box<CheckoutBuilderFactory<'a>>>,
}

impl<'a> UpdateOptions<'a> {
	/// Creates a new set of update options.
	#[must_use]
	pub fn new() -> Self {
		Self::default()
	}

	/// Sets the Git fetch options to use for a bucket.
	///
	/// `factory` takes a bucket and returns the fetch options for the bucket.
	pub fn fetch_options<F>(&mut self, factory: F) -> &mut Self
	where
		F: Fn(&Bucket) -> FetchOptions<'a> + Send + Sync + 'a,
	{
		self.fetch_options = Some(Box::new(factory));
		self
	}

	/// Sets the Git checkout builder to use for a bucket.
	///
	/// `factory` takes a bucket and returns the checkout builder for the bucket.
	pub fn checkout_builder<F>(&mut self, factory: F) -> &mut Self
	where
		F: Fn(&Bucket) -> CheckoutBuilder<'a> + Send + Sync + 'a,
	{
		self.checkout_builder = Some(Box::new(factory));
		self
	}
}

/// An iterator over updated buckets.
/// Yields a two-tuple (bucket, commit) where commit is the HEAD commit before the update.
pub struct Updates {
	inner: vec::IntoIter<(Bucket, git2::Oid)>,
}

impl Iterator for Updates {
	type Item = (Bucket, git2::Oid);

	fn next(&mut self) -> Option<Self::Item> {
		self.inner.next()
	}
}

/// Options for copying a manifest's content. See [`cat`].
///
/// [`cat`]: crate::shovel::Shovel::cat
pub struct CatOptions<'a, W: Write> {
	manifest: &'a str,
	bucket: Option<&'a str>,
	writer: W,
}

impl<'a, W: Write> CatOptions<'a, W> {
	/// Creates a new set of cat options.
	///
	/// # Arguments
	///
	/// * `manifest` - The manifest to read.
	/// * `writer` - The writer to write the manifest into.
	pub fn new(manifest: &'a str, writer: W) -> Self {
		Self {
			manifest,
			bucket: None,
			writer,
		}
	}

	/// Sets the specific bucket to search in for the app.
	pub fn bucket(&mut self, bucket: &'a str) -> &mut Self {
		self.bucket = Some(bucket);
		self
	}
}
