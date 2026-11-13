use std::io::ErrorKind;
use std::process::Command;

/// Runs an external program with the given args, inheriting stdio and waiting for it.
pub fn run(command: &str, args: &[String]) {
    match Command::new(command).args(args).status() {
        Ok(_) => {}
        Err(e) if e.kind() == ErrorKind::NotFound => eprintln!("{}: command not found", command),
        Err(e) => eprintln!("{}: {}", command, e),
    }
}
