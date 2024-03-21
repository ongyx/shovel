use clap;
use shovel;
use tabled;

use crate::run::Run;
use crate::util;

#[derive(tabled::Tabled)]
#[tabled(rename_all = "pascal")]
struct BucketInfo {
    name: String,
    source: String,
    updated: String,
    manifests: usize,
}

impl BucketInfo {
    fn new(bucket: &shovel::Bucket) -> shovel::Result<Self> {
        let name = bucket.name();
        let source = bucket.url()?;
        let commit = bucket.commit()?;
        let updated = shovel::Timestamp::from(commit.time()).to_string();
        let manifests = bucket.manifests()?.count();

        Ok(Self {
            name,
            source,
            updated,
            manifests,
        })
    }
}

#[derive(clap::Args)]
pub struct ListCommand {}

impl ListCommand {}

impl Run for ListCommand {
    fn run(&self, shovel: &mut shovel::Shovel) -> eyre::Result<()> {
        let info: shovel::Result<Vec<_>> = shovel
            .buckets
            .iter()?
            .map(|n| shovel.buckets.open(&n).and_then(|b| BucketInfo::new(&b)))
            .collect();

        println!("\n{}\n", util::tableify(info?, false));

        Ok(())
    }
}
