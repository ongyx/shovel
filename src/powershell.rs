use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::OnceLock;

use which;

/// Returns the path to the newest PowerShell executable available,
/// either PowerShell Core (pwsh.exe, v6+) if it's installed,
/// or PowerShell (powershell.exe, v5.1-) which is in-built on Windows.
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
pub struct Powershell {
	executable: PathBuf,
	prelude: String,
}

impl Powershell {
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

	/// Sets the prelude script.
	///
	/// The prelude script is executed by PowerShell before any other script.
	pub fn prelude<S>(&mut self, script: S) -> &mut Self
	where
		S: Into<String>,
	{
		self.prelude = script.into();
		self
	}

	/// Executes the contents of a reader as a PowerShell script and returns its output.
	///
	/// # Arguments
	///
	/// * `reader` - The reader to read the script from.
	pub fn execute<R>(&self, reader: &mut R) -> io::Result<Output>
	where
		R: Read,
	{
		let mut child = self.to_command().spawn()?;

		// SAFETY: stdin for a new child process should never be None.
		let stdin = child.stdin.as_mut().unwrap();

		writeln!(stdin, "{}", self.prelude)?;

		io::copy(reader, stdin)?;

		// As wait_with_output closes stdin, the actual execution of the script occurs here.
		Ok(child.wait_with_output()?.into())
	}

	/// Runs a PowerShell script from a string and returns its output.
	///
	/// # Arguments
	///
	/// * `script` - The PowerShell script to run.
	pub fn run(&self, script: &str) -> io::Result<Output> {
		let mut cursor = io::Cursor::new(script);

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

impl Default for Powershell {
	fn default() -> Self {
		Self::new(executable())
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	fn powershell() -> Powershell {
		Powershell::default()
	}

	#[test]
	fn hello_world() {
		let output = powershell().run("Write-Host 'Hello World!'").unwrap();
		let lines: Vec<_> = output.stdout.lines().collect();

		dbg!(output.stderr);

		assert!(output.status.success());
		assert_eq!(lines, vec!["Hello World!"]);
	}

	#[test]
	fn multiple_stmts() {
		let output = powershell().run("Write-Host 'a'\nWrite-Host 'b'").unwrap();
		let lines: Vec<_> = output.stdout.lines().collect();

		dbg!(output.stderr);

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

		dbg!(output.stderr);

		assert!(output.status.success());
		assert_eq!(lines, vec!["Initializing...", "Done."]);
	}
}
