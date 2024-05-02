use crate::bucket::Bucket;

/// A filter for buckets and manifests.
///
/// For a no-op filter, use `()`.
pub trait Criteria: Clone {
	/// Determines if the bucket should be yielded.
	fn filter_bucket(&self, bucket: &Bucket) -> bool;

	/// Determines if the manifest by name should be yielded.
	fn filter_manifest(&self, manifest: &str) -> bool;
}

impl Criteria for () {
	fn filter_bucket(&self, _bucket: &Bucket) -> bool {
		true
	}

	fn filter_manifest(&self, _manifest: &str) -> bool {
		true
	}
}

/// A predicate filter.
#[derive(Clone)]
pub struct Predicate<B, M>
where
	B: Fn(&Bucket) -> bool + Clone,
	M: Fn(&str) -> bool + Clone,
{
	bucket: B,
	manifest: M,
}

impl<B, M> Predicate<B, M>
where
	B: Fn(&Bucket) -> bool + Clone,
	M: Fn(&str) -> bool + Clone,
{
	/// Creates a new predicate filter.
	///
	/// # Arguments
	///
	/// * `bucket` - The bucket filter, corresponding to [`Criteria::filter_bucket`].
	/// * `manifest` - The manifest filter, corresponding to [`Criteria::filter_manifest`].
	pub fn new(bucket: B, manifest: M) -> Self {
		Self { bucket, manifest }
	}
}

impl<B, M> Criteria for Predicate<B, M>
where
	B: Fn(&Bucket) -> bool + Clone,
	M: Fn(&str) -> bool + Clone,
{
	fn filter_bucket(&self, bucket: &Bucket) -> bool {
		(self.bucket)(bucket)
	}

	fn filter_manifest(&self, manifest: &str) -> bool {
		(self.manifest)(manifest)
	}
}
