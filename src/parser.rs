/// Where a stream should be redirected to.
#[derive(Debug, PartialEq, Clone)]
pub struct Redirect {
    pub path: String,
    pub append: bool,
}

/// A parsed command line: program, raw (unexpanded) args and optional redirections.
#[derive(Debug, PartialEq, Default)]
pub struct Command {
    pub name: String,
    pub args: Vec<String>,
    pub stdout: Option<Redirect>,
    pub stderr: Option<Redirect>,
}

/// Redirect operators, longest first so `2>>` wins over `2>` and `>`.
pub const OPERATORS: [&str; 4] = ["2>>", "2>", ">>", ">"];

/// Splits `token` into `(operator, attached_target)` if it starts with a redirect operator.
pub fn split_operator(token: &str) -> Option<(&'static str, &str)> {
    OPERATORS
        .iter()
        .find_map(|op| token.strip_prefix(op).map(|rest| (*op, rest)))
}

/// Parses a line. `Ok(None)` for blank lines, `Err` for malformed redirections.
pub fn parse(line: &str) -> Result<Option<Command>, String> {
    let mut tokens = line.split_whitespace();
    let mut words: Vec<String> = Vec::new();
    let mut cmd = Command::default();

    while let Some(token) = tokens.next() {
        let Some((op, attached)) = split_operator(token) else {
            words.push(token.to_string());
            continue;
        };
        let target = if attached.is_empty() {
            tokens
                .next()
                .ok_or_else(|| format!("syntax error: expected a file after `{}`", op))?
        } else {
            attached
        };
        let redirect = Redirect {
            path: target.to_string(),
            append: op.ends_with(">>"),
        };
        if op.starts_with('2') {
            cmd.stderr = Some(redirect);
        } else {
            cmd.stdout = Some(redirect);
        }
    }

    let mut words = words.into_iter();
    let Some(name) = words.next() else {
        return if cmd.stdout.is_some() || cmd.stderr.is_some() {
            Err("syntax error: missing command".to_string())
        } else {
            Ok(None)
        };
    };
    cmd.name = name;
    cmd.args = words.collect();
    Ok(Some(cmd))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn r(path: &str, append: bool) -> Option<Redirect> {
        Some(Redirect { path: path.to_string(), append })
    }

    #[test]
    fn blank_line() {
        assert_eq!(parse("   \n"), Ok(None));
    }

    #[test]
    fn command_and_args() {
        let c = parse("ls  -l   foo\n").unwrap().unwrap();
        assert_eq!(c.name, "ls");
        assert_eq!(c.args, vec!["-l", "foo"]);
        assert_eq!((c.stdout, c.stderr), (None, None));
    }

    #[test]
    fn redirects() {
        let c = parse("cargo build > out.txt 2>> err.txt").unwrap().unwrap();
        assert_eq!(c.args, vec!["build"]);
        assert_eq!(c.stdout, r("out.txt", false));
        assert_eq!(c.stderr, r("err.txt", true));
    }

    #[test]
    fn attached_targets_and_leading_redirect() {
        let c = parse(">>log.txt echo hi 2>e").unwrap().unwrap();
        assert_eq!(c.name, "echo");
        assert_eq!(c.args, vec!["hi"]);
        assert_eq!(c.stdout, r("log.txt", true));
        assert_eq!(c.stderr, r("e", false));
    }

    #[test]
    fn errors() {
        assert!(parse("ls >").is_err());
        assert!(parse("> file").is_err());
    }
}
