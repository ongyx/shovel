use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::BuildHasher;
use std::sync::OnceLock;

fn re_variable() -> &'static regex::Regex {
	static RE_VARIABLE: OnceLock<regex::Regex> = OnceLock::new();

	RE_VARIABLE.get_or_init(|| regex::Regex::new(r"`?\$(?:([\w_]+)|\{([\w_]+)\})").unwrap())
}

/// A trait for looking up variable values. Intended for use with [`format`].
///
/// [`format`]: crate::powershell::format
pub trait Lookup {
	/// The type of lookup value. Must be string-like.
	type Value: AsRef<str>;

	/// Returns the value of a variable if it exists, otherwise None.
	fn lookup(&self, var: &str) -> Option<Self::Value>;
}

impl<F, T> Lookup for F
where
	F: Fn(&str) -> Option<T>,
	T: AsRef<str>,
{
	type Value = T;

	fn lookup(&self, var: &str) -> Option<Self::Value> {
		self(var)
	}
}

macro_rules! impl_lookup_hashmap {
	($($type:ty),+ $(,)?) => {
		$(
			impl<'a, T, S: BuildHasher> Lookup for &'a HashMap<$type, T, S>
			where
				&'a T: AsRef<str>,
			{
				type Value = &'a T;

				fn lookup(&self, var: &str) -> Option<Self::Value> {
					self.get(var)
				}
			}
		)+
	};
}

impl_lookup_hashmap!(&str, String);

struct LookupWrapper<L>(L);

impl<L> regex::Replacer for LookupWrapper<L>
where
	L: Lookup,
	<L as Lookup>::Value: AsRef<str>,
{
	fn replace_append(&mut self, caps: &regex::Captures<'_>, dst: &mut String) {
		let (raw, [var]) = caps.extract();

		// If the variable is escaped (`$...):
		if raw.starts_with('`') {
			// Push the escaped variable back.
			dst.push_str(raw);
		} else {
			// Push the variable's value to the buffer if it exists, otherwise leave it blank.
			// This is consistent with PowerShell's behaviour.
			if let Some(value) = self.0.lookup(var) {
				dst.push_str(value.as_ref());
			}
		}
	}
}

/// Interpolates PowerShell variables in a template using the given lookup.
///
/// The following syntax is supported:
/// * `$foo` - Formats the variable `foo`.
/// * `${foo}_bar` - Formats the variable `foo`. The braces act as delimiters so `foo_bar` is not formatted instead.
///
/// # Arguments
///
/// * `template` - The template to format.
/// * `lookup` - The lookup implementation.
pub fn format<L>(template: &str, lookup: L) -> Cow<'_, str>
where
	L: Lookup,
{
	re_variable().replace_all(template, LookupWrapper(lookup))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn format_var() {
		let formatted = format("$foo, $bar, $foobar", |var: &str| match var {
			"foo" => Some("1"),
			"bar" => Some("2"),
			_ => None,
		});

		assert_eq!(formatted, "1, 2, ");

		let not_formatted = format("`$foo", |_: &str| Some("foo"));

		assert_eq!(not_formatted, "`$foo");

		let delimited = format("${foo}_bar", |_: &str| Some("1"));

		assert_eq!(delimited, "1_bar");
	}

	#[test]
	fn format_map() {
		let vars = HashMap::from([("foo", "1"), ("bar", "2")]);
		let formatted = format("$foo $bar", &vars);

		assert_eq!(formatted, "1 2");
	}
}
