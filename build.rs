use std::env;
use std::fs;
use std::io;
use std::io::prelude::*;
use std::path::PathBuf;

fn main() {
	let mut json_path = env_as_path("CARGO_MANIFEST_DIR");
	json_path.push("buckets.json");

	// Read the known buckets from the JSON file.
	let json_file = io::BufReader::new(fs::File::open(json_path).unwrap());
	// Use Value to preserve ordering.
	let json: serde_json::Value = serde_json::from_reader(json_file).unwrap();

	let mut map = phf_codegen::OrderedMap::new();

	// Add entries to the phf map from the JSON.
	let json = json.as_object().unwrap();
	for (bucket, url) in json {
		let url = url.as_str().unwrap();

		map.entry(bucket, &format!(r#""{}""#, url));
	}

	let mut rs_path = env_as_path("OUT_DIR");
	rs_path.push("buckets.rs");

	let mut rs_file = io::BufWriter::new(fs::File::create(rs_path).unwrap());

	// Write the phf map to the Rust file.
	writeln!(&mut rs_file, "use phf;").unwrap();
	writeln!(&mut rs_file, "/// Map of known bucket names to their URLs.").unwrap();
	writeln!(
		&mut rs_file,
		"/// Derived from https://github.com/ScoopInstaller/Scoop/blob/master/buckets.json"
	)
	.unwrap();
	writeln!(
		&mut rs_file,
		"static KNOWN_BUCKETS: phf::OrderedMap<&'static str, &'static str> = {};",
		map.build(),
	)
	.unwrap();
}

fn env_as_path(name: &str) -> PathBuf {
	PathBuf::from(&env::var(name).unwrap())
}
