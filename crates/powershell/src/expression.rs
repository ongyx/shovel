use std::fmt::Display;

/// A PowerShell expression.
pub enum Expression {
	/// A literal string. Single quotes are escaped.
	Verbatim(String),

	/// A string with interpolation. Double quotes are escaped.
	Expandable(String),

	/// A raw expression. No escaping is done.
	Raw(String),

	/// A boolean expression.
	Bool(bool),
}

impl Expression {
	/// Convert a serializable type `T` to a PowerShell object.
	///
	/// # Arguments
	///
	/// `value` - The value to convert.
	///
	/// # Errors
	///
	/// If the value cannot be converted to JSON, [`Error::Json`] is returned.
	pub fn object<T>(value: &T) -> Result<Self, serde_json::Error>
	where
		T: serde::Serialize,
	{
		let value = serde_json::to_string(value)?;

		// TODO: Is there a more efficient way of passing the object to PowerShell?
		Ok(Self::Raw(format!("{value} | ConvertTo-Json")))
	}
}

impl Display for Expression {
	fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
		use Expression::*;

		match self {
			Verbatim(str) => {
				write!(f, "'{}'", str.replace('\'', "''"))
			}
			Expandable(str) => {
				write!(f, "\"{}\"", str.replace('"', "`\""))
			}
			Raw(str) => {
				write!(f, "{str}")
			}
			Bool(bool) => {
				write!(f, "{}", if *bool { "$true" } else { "$false" })
			}
		}
	}
}

impl From<String> for Expression {
	fn from(value: String) -> Self {
		Self::Verbatim(value)
	}
}

impl From<bool> for Expression {
	fn from(value: bool) -> Self {
		Self::Bool(value)
	}
}
