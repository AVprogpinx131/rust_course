use std::fs::File;
use std::io::ErrorKind;
use std::process::{Command, Stdio};

/// Runs an external program, waiting for it. Redirected streams are passed as files;
/// `None` means inherit the shell's own stream.
pub fn run(command: &str, args: &[String], stdout: Option<File>, stderr: Option<File>) {
    let mut cmd = Command::new(command);
    cmd.args(args);
    if let Some(f) = stdout {
        cmd.stdout(Stdio::from(f));
    }
    if let Some(f) = stderr {
        cmd.stderr(Stdio::from(f));
    }
    match cmd.status() {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => eprintln!("{}: command not found", command),
        Err(e) => eprintln!("{}: {}", command, e),
    }
}
