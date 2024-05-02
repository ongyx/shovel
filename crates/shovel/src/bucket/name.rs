use crate::bucket::Bucket;
use crate::bucket::Criteria;

/// A name for a manifest in a bucket.
///
/// The syntax is `(bucket/)manifest`, where bucket is optional.
#[derive(Clone)]
pub struct Name {
	full: String,
	sep: Option<usize>,
}

impl Name {
	/// Creates a new name from a string.
	#[must_use]
	pub fn new(full: String) -> Self {
		let sep = full.find('/');

		Self { full, sep }
	}

	/// Returns the full name.
	#[must_use]
	pub fn full(&self) -> &str {
		&self.full
	}

	/// Returns the manifest for this name.
	#[must_use]
	pub fn manifest(&self) -> &str {
		match self.sep {
			Some(sep) => {
				// sep is the start of the slash, so advance by 1.
				&self.full[sep + 1..]
			}
			None => &self.full,
		}
	}

	/// Returns the bucket for this name.
	///
	/// If the bucket is not present, None is returned.
	#[must_use]
	pub fn bucket(&self) -> Option<&str> {
		self.sep.map(|sep| &self.full[..sep])
	}
}

impl From<String> for Name {
	fn from(value: String) -> Self {
		Self::new(value)
	}
}

impl Criteria for Name {
	fn filter_bucket(&self, bucket: &Bucket) -> bool {
		if let Some(name) = self.bucket() {
			bucket.name() == name
		} else {
			true
		}
	}

	fn filter_manifest(&self, manifest: &str) -> bool {
		manifest == self.manifest()
	}
}
