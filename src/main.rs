mod builtins;
mod expand;
mod external;
mod parser;

use builtins::Flow;

fn main() -> Result<(), std::io::Error> {
    let mut buffer = String::new();
    let stdin = std::io::stdin();

    loop {
        buffer.clear();
        if stdin.read_line(&mut buffer)? == 0 {
            break;
        }

        let Some((command, raw_args)) = parser::parse(&buffer) else {
            continue;
        };
        let args: Vec<String> = raw_args.iter().flat_map(|a| expand::expand(a)).collect();

        match builtins::run(&command, &args) {
            Some(Flow::Exit) => break,
            Some(Flow::Continue) => {}
            None => external::run(&command, &args),
        }
    }

    Ok(())
}
