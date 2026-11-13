use crate::expand::home_dir;

pub enum Flow {
    Continue,
    Exit,
}

/// Runs a built-in command. Returns `None` if `command` is not a built-in.
pub fn run(command: &str, args: &[String]) -> Option<Flow> {
    match command {
        "exit" => Some(Flow::Exit),
        "cd" => {
            cd(args);
            Some(Flow::Continue)
        }
        "ls" => {
            ls(args);
            Some(Flow::Continue)
        }
        _ => None,
    }
}

fn cd(args: &[String]) {
    let target = match args.first() {
        Some(t) => t.clone(),
        None => home_dir().unwrap_or_else(|| ".".to_string()),
    };
    if let Err(e) = std::env::set_current_dir(&target) {
        eprintln!("cd: {}: {}", target, e);
    }
}

fn ls(args: &[String]) {
    let dirs: Vec<&str> = if args.is_empty() {
        vec!["."]
    } else {
        args.iter().map(String::as_str).collect()
    };
    let multiple = dirs.len() > 1;

    for dir in dirs {
        if multiple {
            println!("{}:", dir);
        }
        match std::fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    println!("{}", entry.file_name().to_string_lossy());
                }
            }
            // A glob like `*.rs` expands to files, so list a plain file by name.
            Err(_) if std::path::Path::new(dir).is_file() => println!("{}", dir),
            Err(e) => eprintln!("ls: {}: {}", dir, e),
        }
    }
}
