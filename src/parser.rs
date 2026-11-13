/// Splits a line into a command name and its raw (unexpanded) arguments.
/// Returns `None` for blank lines.
pub fn parse(line: &str) -> Option<(String, Vec<String>)> {
    let mut parts = line.split_whitespace();
    let command = parts.next()?.to_string();
    let args = parts.map(str::to_string).collect();
    Some((command, args))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blank_line() {
        assert_eq!(parse("   \n"), None);
    }

    #[test]
    fn command_and_args() {
        assert_eq!(
            parse("ls  -l   foo\n"),
            Some(("ls".to_string(), vec!["-l".to_string(), "foo".to_string()]))
        );
    }
}
