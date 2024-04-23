use std::io;
use std::io::prelude::*;
use std::path::Path;
use std::path::PathBuf;
use std::process;
use std::sync::OnceLock;

use crate::Expression;

/// Returns the path to the newest PowerShell executable available,
/// either PowerShell Core (pwsh.exe, v6+) if it's installed,
/// or PowerShell (powershell.exe, v5.1-) which is in-built on Windows.
#[allow(clippy::missing_panics_doc)]
pub fn executable() -> &'static PathBuf {
	static EXECUTABLE: OnceLock<PathBuf> = OnceLock::new();

	EXECUTABLE.get_or_init(|| {
		which::which_global("pwsh.exe")
			.or_else(|_| which::which_global("powershell.exe"))
			.expect("PowerShell v5.1 is installed by default on Windows")
	})
}

/// The output of a PowerShell script.
///
/// Depending on the version of PowerShell, newlines are not guaranteed to be CRLF.
/// Use `str::lines` to iterate over lines if needed.
#[derive(Debug)]
pub struct Output {
	/// The exit status.
	pub status: process::ExitStatus,

	/// The standard output.
	pub stdout: String,

	/// The standard error.
	pub stderr: String,
}

impl From<process::Output> for Output {
	fn from(output: process::Output) -> Self {
		Self {
			status: output.status,
			stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
			stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
		}
	}
}

/// A PowerShell script runner.
///
/// Scripts are executed in a sub-process for idempotency.
pub struct Runner {
	executable: PathBuf,
	prelude: String,
}

impl Runner {
	/// Creates a new runner.
	///
	/// # Arguments
	///
	/// * `executable` - The PowerShell executable. To use the newest version available, use `Powershell::default()`.
	pub fn new<P>(executable: P) -> Self
	where
		P: AsRef<Path>,
	{
		let executable = executable.as_ref().to_owned();

		Self {
			executable,
			prelude: String::new(),
		}
	}

	/// Adds a prelude script to the runner.
	/// Prelude scripts are always executed before scripts from `PowerShell::execute` or `PowerShell::run`.
	///
	/// # Arguments
	///
	/// * `script` - The prelude script.
	pub fn prelude<S>(&mut self, script: S) -> &mut Self
	where
		S: AsRef<str>,
	{
		self.prelude.push_str(script.as_ref());
		self.prelude.push_str("\r\n");
		self
	}

	/// Adds a prelude variable to the runner.
	///
	/// Variables are guaranteed to have the same order as when they were added.
	///
	/// # Arguments
	///
	/// * `name`: The variable name.
	/// * `expr`: The variable expression.
	pub fn var<S>(&mut self, name: S, expr: &Expression) -> &mut Self
	where
		S: Into<String>,
	{
		self.prelude(format!("${} = {}", name.into(), expr))
	}

	/// Adds multiple prelude variables to the runner.
	///
	/// # Arguments
	///
	/// * `vars`: The variables to add.
	pub fn vars<I, S>(&mut self, vars: I) -> &mut Self
	where
		I: IntoIterator<Item = (S, Expression)>,
		S: Into<String>,
	{
		for (name, expr) in vars {
			self.var(name, &expr);
		}

		self
	}

	/// Executes the contents of a reader as a PowerShell script and returns its output.
	///
	/// # Arguments
	///
	/// * `reader` - The reader to read the script from.
	///
	/// # Errors
	///
	/// If the PowerShell process cannot be spawned or waiting for its output failed, the IO error is returned.
	#[allow(clippy::missing_panics_doc)]
	pub fn execute<R>(&self, reader: &mut R) -> io::Result<Output>
	where
		R: Read,
	{
		let mut child = self.to_command().spawn()?;

		// SAFETY: stdin for a new child process should never be None.
		let stdin = child.stdin.as_mut().unwrap();

		// Bail on all uncaught errors.
		// It's possible for commands to fail if they don't exist.
		writeln!(stdin, "$ErrorActionPreference = 'Stop'; trap {{ exit 1 }}")?;

		// Write the prelude script.
		writeln!(stdin, "{}", self.prelude)?;

		// Write the script from the reader.
		io::copy(reader, stdin)?;

		// As wait_with_output closes stdin, the actual execution of the script occurs here.
		Ok(child.wait_with_output()?.into())
	}

	/// Runs a PowerShell script from a string and returns its output.
	///
	/// # Arguments
	///
	/// * `script` - The PowerShell script to run.
	///
	/// # Errors
	///
	/// See [`execute`].
	///
	/// [`execute`]: Powershell::execute
	pub fn run<S>(&self, script: S) -> io::Result<Output>
	where
		S: AsRef<str>,
	{
		let mut cursor = io::Cursor::new(script.as_ref());

		self.execute(&mut cursor)
	}

	fn to_command(&self) -> process::Command {
		let mut cmd = process::Command::new(&self.executable);

		// Pipe:
		// * stdio to pass the script to PowerShell.
		// * stdout/stderr to capture output.
		cmd.stdin(process::Stdio::piped());
		cmd.stdout(process::Stdio::piped());
		cmd.stderr(process::Stdio::piped());

		// Specify arguments for a non-interactive session.
		// This tells PowerShell to read scripts over stdin as well.
		cmd.args(["-NoLogo", "-NoProfile", "-NonInteractive", "-Command", "-"]);

		cmd
	}
}

impl Default for Runner {
	fn default() -> Self {
		Self::new(executable())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn powershell() -> Runner {
		Runner::default()
	}

	#[test]
	fn hello_world() {
		let output = powershell().run("Write-Host 'Hello World!'").unwrap();
		let lines: Vec<_> = output.stdout.lines().collect();

		assert!(output.status.success());
		assert_eq!(lines, vec!["Hello World!"]);
	}

	#[test]
	fn multiple_stmts() {
		let output = powershell().run("Write-Host 'a'\nWrite-Host 'b'").unwrap();
		let lines: Vec<_> = output.stdout.lines().collect();

		assert!(output.status.success());
		assert_eq!(lines, vec!["a", "b"]);
	}

	#[test]
	fn prelude() {
		let output = powershell()
			.prelude("Write-Host 'Initializing...'")
			.run("Write-Host 'Done.'")
			.unwrap();
		let lines: Vec<_> = output.stdout.lines().collect();

		assert!(output.status.success());
		assert_eq!(lines, vec!["Initializing...", "Done."]);
	}

	#[test]
	fn vars() {
		use Expression::*;

		let output = powershell()
			.vars([
				("foo", Verbatim("a".to_owned())),
				("bar", Expandable("${foo}b".to_owned())),
				("baz", Raw("$foo, $bar".to_owned())),
			])
			.run("Write-Host $baz")
			.unwrap();
		let lines: Vec<_> = output.stdout.lines().collect();

		assert!(output.status.success());
		assert_eq!(lines, vec!["a ab"]);
	}

	#[test]
	fn error() {
		let output = powershell().run("this-is-an-nonexistent-script").unwrap();

		assert!(!output.status.success());
	}
}
