use std::io;

use serde;
use serde::de;
use serde_json;
use serde_path_to_error;

/// A JSON (de)serialization error.
pub type Error = serde_path_to_error::Error<serde_json::Error>;

/// Deserialize a type `T` from a reader as JSON.
/// The reader is wrapped in a buffered reader.
///
/// # Arguments
///
/// * `reader` - The reader to deserialize from.
///
/// # Errors
///
/// Errors from `serde_path_to_error` are returned verbatim.
pub fn from_reader<R, T>(reader: R) -> Result<T, Error>
where
	R: io::Read,
	T: de::DeserializeOwned,
{
	let reader = io::BufReader::new(reader);
	let de = &mut serde_json::Deserializer::from_reader(reader);

	let value = serde_path_to_error::deserialize(de)?;

	Ok(value)
}

/// Serialize a type `T` to a writer as JSON.
/// The writer is wrapped in a buffered writer.
///
/// # Arguments
///
/// * `writer` - The writer to serialize to.
/// * `value` - The value to serialize.
///
/// # Errors
///
/// Errors from `serde_path_to_error` are returned verbatim.
pub fn to_writer<W, T>(writer: W, value: &T) -> Result<(), Error>
where
	W: io::Write,
	T: serde::Serialize,
{
	let writer = io::BufWriter::new(writer);
	let ser = &mut serde_json::Serializer::new(writer);

	serde_path_to_error::serialize(value, ser)
}

/// Serialize a type `T` to a string as JSON.
///
/// # Arguments
///
/// * `value` - The value to serialize.
///
/// # Errors
///
/// Errors from `serde_path_to_error` are returned verbatim.
///
/// # Panics
///
/// This function panics if the JSON produced is not valid UTF-8.
pub fn to_string<T>(value: &T) -> Result<String, Error>
where
	T: serde::Serialize,
{
	let mut buf = Vec::new();

	to_writer(&mut buf, value)?;

	Ok(String::from_utf8(buf).expect("JSON is valid UTF-8"))
}

/// Macro for generating a JSON enum.
macro_rules! json_enum {
	($item:item) => {
		#[serde_with::serde_as]
		#[serde_with::skip_serializing_none]
		#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
		#[serde(untagged)]
		$item
	};
}

/// Macro for generating a JSON enum as a map key.
macro_rules! json_enum_key {
	($item:item) => {
		#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq, Hash, Copy, Clone)]
		$item
	};
}

/// Macro for generating a JSON struct without a default impl.
macro_rules! json_struct_nodefault {
	($item:item) => {
		#[serde_with::serde_as]
		#[serde_with::skip_serializing_none]
		#[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Eq)]
		$item
	};
}

/// Macro for generating a JSON struct.
macro_rules! json_struct {
	($item:item) => {
		$crate::json::json_struct_nodefault! {
			#[derive(Default)]
			$item
		}
	};
}

pub(crate) use json_enum;
pub(crate) use json_enum_key;
pub(crate) use json_struct;
pub(crate) use json_struct_nodefault;
