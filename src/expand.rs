use std::path::Path;

/// Returns the user's home directory from `$HOME` (or `%USERPROFILE%` on Windows).
pub fn home_dir() -> Option<String> {
    std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .ok()
        .filter(|h| !h.is_empty())
}

/// Expands a single argument: `~` -> home, `.` -> current dir, `*` -> matching paths.
/// One argument may expand to many; a glob with no match is kept literally.
pub fn expand(arg: &str) -> Vec<String> {
    let arg = expand_tilde(arg);
    if arg == "." {
        if let Ok(cwd) = std::env::current_dir() {
            return vec![cwd.to_string_lossy().into_owned()];
        }
    }
    if arg.contains('*') {
        let matches = glob(&arg);
        if !matches.is_empty() {
            return matches;
        }
    }
    vec![arg]
}

fn expand_tilde(arg: &str) -> String {
    if arg == "~" || arg.starts_with("~/") {
        if let Some(home) = home_dir() {
            return format!("{}{}", home, &arg[1..]);
        }
    }
    arg.to_string()
}

/// Expands a glob pattern component by component. Only `*` is special.
fn glob(pattern: &str) -> Vec<String> {
    let (mut paths, rest) = match pattern.strip_prefix('/') {
        Some(rest) => (vec!["/".to_string()], rest),
        None => (vec![String::new()], pattern),
    };

    for segment in rest.split('/').filter(|s| !s.is_empty()) {
        let mut next = Vec::new();
        for base in &paths {
            if segment.contains('*') {
                let dir = if base.is_empty() { "." } else { base.as_str() };
                let Ok(entries) = std::fs::read_dir(dir) else { continue };
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().into_owned();
                    if name.starts_with('.') && !segment.starts_with('.') {
                        continue;
                    }
                    if wildcard_match(segment, &name) {
                        next.push(join(base, &name));
                    }
                }
            } else {
                let candidate = join(base, segment);
                if Path::new(&candidate).exists() {
                    next.push(candidate);
                }
            }
        }
        paths = next;
    }

    paths.sort();
    paths
}

fn join(base: &str, name: &str) -> String {
    if base.is_empty() {
        name.to_string()
    } else if base.ends_with('/') {
        format!("{}{}", base, name)
    } else {
        format!("{}/{}", base, name)
    }
}

/// Matches `text` against `pat` where `*` matches any run of characters.
fn wildcard_match(pat: &str, text: &str) -> bool {
    let p: Vec<char> = pat.chars().collect();
    let t: Vec<char> = text.chars().collect();
    let (mut pi, mut ti) = (0, 0);
    let mut star: Option<(usize, usize)> = None;

    while ti < t.len() {
        if pi < p.len() && p[pi] == '*' {
            star = Some((pi, ti));
            pi += 1;
        } else if pi < p.len() && p[pi] == t[ti] {
            pi += 1;
            ti += 1;
        } else if let Some((sp, st)) = star {
            pi = sp + 1;
            ti = st + 1;
            star = Some((sp, st + 1));
        } else {
            return false;
        }
    }
    while pi < p.len() && p[pi] == '*' {
        pi += 1;
    }
    pi == p.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wildcard_cases() {
        assert!(wildcard_match("*", ""));
        assert!(wildcard_match("*", "anything"));
        assert!(wildcard_match("a*", "abc"));
        assert!(!wildcard_match("a*", "bac"));
        assert!(wildcard_match("*.rs", "main.rs"));
        assert!(!wildcard_match("*.rs", "main.rs.bak"));
        assert!(wildcard_match("a*b*c", "aXXbYYc"));
        assert!(!wildcard_match("a*b*c", "aXXbYY"));
        assert!(wildcard_match("", ""));
        assert!(!wildcard_match("", "x"));
    }

    #[test]
    fn dot_expands_to_cwd() {
        let cwd = std::env::current_dir().unwrap().to_string_lossy().into_owned();
        assert_eq!(expand("."), vec![cwd]);
    }

    #[test]
    fn tilde_expands_to_home() {
        if let Some(home) = home_dir() {
            assert_eq!(expand("~"), vec![home.clone()]);
            assert_eq!(expand("~/x"), vec![format!("{}/x", home)]);
        }
        assert_eq!(expand("~bob"), vec!["~bob".to_string()]);
    }

    #[test]
    fn no_match_is_literal() {
        assert_eq!(expand("zzz_no_such_*"), vec!["zzz_no_such_*".to_string()]);
    }

    #[test]
    fn glob_in_temp_dir() {
        let dir = std::env::temp_dir().join("rust_course_glob_test");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        for f in ["a.rs", "b.rs", "c.txt", ".hidden.rs"] {
            std::fs::write(dir.join(f), "").unwrap();
        }
        std::fs::write(dir.join("sub").join("d.rs"), "").unwrap();

        let base = dir.to_string_lossy().replace('\\', "/");
        let rs = expand(&format!("{}/*.rs", base));
        assert_eq!(rs, vec![format!("{}/a.rs", base), format!("{}/b.rs", base)]);

        let nested = expand(&format!("{}/*/*.rs", base));
        assert_eq!(nested, vec![format!("{}/sub/d.rs", base)]);

        let _ = std::fs::remove_dir_all(&dir);
    }
}
