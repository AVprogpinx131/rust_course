mod builtins;
mod expand;
mod external;
mod helper;
mod parser;

use std::fs::{File, OpenOptions};
use std::io::{self, Write};

use builtins::Flow;
use parser::Redirect;
use rustyline::error::ReadlineError;
use rustyline::history::FileHistory;
use rustyline::Editor;

const PROMPT: &str = "$ ";

fn history_path() -> Option<std::path::PathBuf> {
    expand::home_dir().map(|h| std::path::Path::new(&h).join(".rust_course_history"))
}

/// Expands the redirect target and opens it (truncating or appending).
fn open_redirect(redirect: &Option<Redirect>) -> Result<Option<File>, String> {
    let Some(r) = redirect else { return Ok(None) };
    let mut targets = expand::expand(&r.path);
    if targets.len() != 1 {
        return Err(format!("{}: ambiguous redirect", r.path));
    }
    let path = targets.remove(0);
    OpenOptions::new()
        .write(true)
        .create(true)
        .append(r.append)
        .truncate(!r.append)
        .open(&path)
        .map(Some)
        .map_err(|e| format!("{}: {}", path, e))
}

/// Runs one parsed command. Returns `false` when the shell should exit.
fn execute(cmd: parser::Command) -> bool {
    let (out_file, err_file) = match (open_redirect(&cmd.stdout), open_redirect(&cmd.stderr)) {
        (Ok(o), Ok(e)) => (o, e),
        (Err(msg), _) | (_, Err(msg)) => {
            eprintln!("{}", msg);
            return true;
        }
    };
    let args: Vec<String> = cmd.args.iter().flat_map(|a| expand::expand(a)).collect();

    if builtins::is_builtin(&cmd.name) {
        let mut out: Box<dyn Write> = match out_file {
            Some(f) => Box::new(f),
            None => Box::new(io::stdout()),
        };
        let mut err: Box<dyn Write> = match err_file {
            Some(f) => Box::new(f),
            None => Box::new(io::stderr()),
        };
        let flow = builtins::run(&cmd.name, &args, &mut out, &mut err);
        let _ = out.flush();
        return matches!(flow, Flow::Continue);
    }

    external::run(&cmd.name, &args, out_file, err_file);
    true
}

fn main() -> rustyline::Result<()> {
    let mut editor: Editor<helper::ShellHelper, FileHistory> = Editor::new()?;
    editor.set_helper(Some(helper::ShellHelper::new()));

    let history = history_path();
    if let Some(path) = &history {
        let _ = editor.load_history(path); // absent on first run
    }

    loop {
        let line = match editor.readline(PROMPT) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => continue,
            Err(ReadlineError::Eof) => break,
            Err(e) => return Err(e),
        };
        if !line.trim().is_empty() {
            let _ = editor.add_history_entry(line.as_str());
        }

        match parser::parse(&line) {
            Ok(Some(cmd)) => {
                if !execute(cmd) {
                    break;
                }
            }
            Ok(None) => {}
            Err(msg) => eprintln!("{}", msg),
        }
    }

    if let Some(path) = &history {
        let _ = editor.save_history(path);
    }
    Ok(())
}
