//! rustyline integration: command/path completion and syntax highlighting.

use std::borrow::Cow;
use std::collections::BTreeSet;
use std::path::Path;

use rustyline::completion::{Completer, Pair};
use rustyline::highlight::{CmdKind, Highlighter};
use rustyline::hint::Hinter;
use rustyline::validate::Validator;
use rustyline::{Context, Helper};

use crate::builtins;
use crate::expand::home_dir;
use crate::parser::split_operator;

const RESET: &str = "\x1b[0m";
const GREEN: &str = "\x1b[32m";
const RED: &str = "\x1b[31m";
const CYAN: &str = "\x1b[36m";
const MAGENTA: &str = "\x1b[35m";
const YELLOW: &str = "\x1b[33m";

pub struct ShellHelper {
    /// Built-ins plus every executable name found on `PATH`.
    commands: BTreeSet<String>,
}

impl ShellHelper {
    pub fn new() -> Self {
        let mut commands: BTreeSet<String> = builtins::NAMES.iter().map(|s| s.to_string()).collect();
        if let Some(path) = std::env::var_os("PATH") {
            for dir in std::env::split_paths(&path) {
                let Ok(entries) = std::fs::read_dir(dir) else { continue };
                for entry in entries.flatten() {
                    if !entry.path().is_file() {
                        continue;
                    }
                    let name = entry.file_name().to_string_lossy().into_owned();
                    commands.insert(strip_exe_ext(&name).to_string());
                    commands.insert(name);
                }
            }
        }
        ShellHelper { commands }
    }

    fn is_command(&self, word: &str) -> bool {
        if word.contains('/') || word.contains('\\') {
            return Path::new(word).is_file();
        }
        self.commands.contains(word)
    }
}

/// Drops a Windows executable extension so `git.exe` also completes as `git`.
fn strip_exe_ext(name: &str) -> &str {
    let lower = name.to_ascii_lowercase();
    for ext in [".exe", ".cmd", ".bat", ".com"] {
        if lower.ends_with(ext) {
            return &name[..name.len() - ext.len()];
        }
    }
    name
}

impl Helper for ShellHelper {}
impl Hinter for ShellHelper {
    type Hint = String;
}
impl Validator for ShellHelper {}

impl Completer for ShellHelper {
    type Candidate = Pair;

    fn complete(
        &self,
        line: &str,
        pos: usize,
        _ctx: &Context<'_>,
    ) -> rustyline::Result<(usize, Vec<Pair>)> {
        let before = &line[..pos];
        let start = before.rfind(char::is_whitespace).map_or(0, |i| i + 1);
        let word = &before[start..];
        let preceding: Vec<&str> = before[..start].split_whitespace().collect();

        // The command position: nothing before it except (possibly) redirections.
        let is_command_pos = preceding.is_empty();
        if is_command_pos && !word.contains('/') && !word.contains('\\') && !word.starts_with('~') {
            let matches = self
                .commands
                .iter()
                .filter(|c| c.starts_with(word))
                .map(|c| Pair { display: c.clone(), replacement: format!("{} ", c) })
                .collect();
            return Ok((start, matches));
        }
        Ok((start, complete_path(word)))
    }
}

/// Completes a file path, keeping the user's typed prefix (`~`, relative dirs) intact.
fn complete_path(word: &str) -> Vec<Pair> {
    let (dir_part, prefix) = match word.rfind(['/', '\\']) {
        Some(i) => (&word[..=i], &word[i + 1..]),
        None => ("", word),
    };
    let real_dir = match (dir_part, home_dir()) {
        ("", _) => ".".to_string(),
        (d, Some(home)) if d.starts_with("~/") => format!("{}{}", home, &d[1..]),
        (d, _) => d.to_string(),
    };

    let Ok(entries) = std::fs::read_dir(&real_dir) else { return Vec::new() };
    let mut out: Vec<Pair> = entries
        .flatten()
        .filter_map(|e| {
            let name = e.file_name().to_string_lossy().into_owned();
            if !name.starts_with(prefix) || (name.starts_with('.') && !prefix.starts_with('.')) {
                return None;
            }
            let is_dir = e.path().is_dir();
            let shown = if is_dir { format!("{}/", name) } else { name };
            Some(Pair { replacement: format!("{}{}", dir_part, shown), display: shown })
        })
        .collect();
    out.sort_by(|a, b| a.display.cmp(&b.display));
    out
}

impl Highlighter for ShellHelper {
    fn highlight<'l>(&self, line: &'l str, _pos: usize) -> Cow<'l, str> {
        Cow::Owned(highlight_line(line, |w| self.is_command(w)))
    }

    fn highlight_char(&self, _line: &str, _pos: usize, _kind: CmdKind) -> bool {
        true
    }
}

/// Colours a line: valid command green / unknown red, redirections cyan, `~`/`*` words
/// magenta, flags yellow. Whitespace is preserved exactly.
pub fn highlight_line(line: &str, is_command: impl Fn(&str) -> bool) -> String {
    let mut out = String::with_capacity(line.len() * 2);
    let mut seen_command = false;
    let mut expect_target = false;

    let mut rest = line;
    while !rest.is_empty() {
        let ws_len = rest.len() - rest.trim_start().len();
        out.push_str(&rest[..ws_len]);
        rest = &rest[ws_len..];
        if rest.is_empty() {
            break;
        }
        let end = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let token = &rest[..end];
        rest = &rest[end..];

        let color = if let Some((op, attached)) = split_operator(token) {
            expect_target = attached.is_empty();
            out.push_str(CYAN);
            out.push_str(op);
            out.push_str(RESET);
            out.push_str(attached);
            continue;
        } else if expect_target {
            expect_target = false;
            ""
        } else if !seen_command {
            seen_command = true;
            if is_command(token) { GREEN } else { RED }
        } else if token.starts_with('~') || token.contains('*') || token == "." {
            MAGENTA
        } else if token.starts_with('-') {
            YELLOW
        } else {
            ""
        };

        if color.is_empty() {
            out.push_str(token);
        } else {
            out.push_str(color);
            out.push_str(token);
            out.push_str(RESET);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn known(w: &str) -> bool {
        w == "ls"
    }

    #[test]
    fn colours_command_by_validity() {
        assert_eq!(highlight_line("ls", known), format!("{GREEN}ls{RESET}"));
        assert_eq!(highlight_line("nope", known), format!("{RED}nope{RESET}"));
    }

    #[test]
    fn colours_args_and_redirects_preserving_spacing() {
        let got = highlight_line("ls  -l ~/x > out.txt", known);
        let want = format!(
            "{GREEN}ls{RESET}  {YELLOW}-l{RESET} {MAGENTA}~/x{RESET} {CYAN}>{RESET} out.txt"
        );
        assert_eq!(got, want);
    }

    #[test]
    fn strips_exe_extension() {
        assert_eq!(strip_exe_ext("git.exe"), "git");
        assert_eq!(strip_exe_ext("README"), "README");
    }

    #[test]
    fn completes_paths_in_dir() {
        let dir = std::env::temp_dir().join("rust_course_complete_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("sample.txt"), "").unwrap();
        let base = dir.to_string_lossy().replace('\\', "/");

        let got: Vec<String> = complete_path(&format!("{}/s", base))
            .into_iter()
            .map(|p| p.replacement)
            .collect();
        assert_eq!(got, vec![format!("{}/sample.txt", base), format!("{}/sub/", base)]);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
