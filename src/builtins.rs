use crate::expand::home_dir;
use std::io::Write;

pub enum Flow {
    Continue,
    Exit,
}

pub const NAMES: [&str; 3] = ["exit", "cd", "ls"];

pub fn is_builtin(command: &str) -> bool {
    NAMES.contains(&command)
}

/// Runs a built-in command, writing to the given (possibly redirected) streams.
pub fn run(command: &str, args: &[String], out: &mut dyn Write, err: &mut dyn Write) -> Flow {
    match command {
        "exit" => return Flow::Exit,
        "cd" => cd(args, err),
        "ls" => ls(args, out, err),
        _ => {}
    }
    Flow::Continue
}

fn cd(args: &[String], err: &mut dyn Write) {
    let target = match args.first() {
        Some(t) => t.clone(),
        None => home_dir().unwrap_or_else(|| ".".to_string()),
    };
    if let Err(e) = std::env::set_current_dir(&target) {
        let _ = writeln!(err, "cd: {}: {}", target, e);
    }
}

fn ls(args: &[String], out: &mut dyn Write, err: &mut dyn Write) {
    let dirs: Vec<&str> = if args.is_empty() {
        vec!["."]
    } else {
        args.iter().map(String::as_str).collect()
    };
    let multiple = dirs.len() > 1;

    for dir in dirs {
        if multiple {
            let _ = writeln!(out, "{}:", dir);
        }
        match std::fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let _ = writeln!(out, "{}", entry.file_name().to_string_lossy());
                }
            }
            // A glob like `*.rs` expands to files, so list a plain file by name.
            Err(_) if std::path::Path::new(dir).is_file() => {
                let _ = writeln!(out, "{}", dir);
            }
            Err(e) => {
                let _ = writeln!(err, "ls: {}: {}", dir, e);
            }
        }
    }
}
