fn main() -> Result<(), std::io::Error> {
    let mut buffer = String::new();
    let stdin = std::io::stdin();

    loop {
        buffer.clear();
        if stdin.read_line(&mut buffer)? == 0 {
            break;
        }

        let mut parts = buffer.split_whitespace();
        let Some(command) = parts.next() else { continue; };
        let args: Vec<&str> = parts.collect();

        match command {
            "exit" => break,
            "cd" => {
                let target = args.first().copied().unwrap_or(".");
                if let Err(e) = std::env::set_current_dir(target) {
                    eprintln!("cd: {}: {}", target, e);
                }
            }
            "ls" => {
                let dir = args.first().copied().unwrap_or(".");
                match std::fs::read_dir(dir) {
                    Ok(entries) => {
                        for entry in entries.flatten() {
                            println!("{}", entry.file_name().to_string_lossy());
                        }
                    }
                    Err(e) => eprintln!("ls: {}: {}", dir, e),
                }
            }
            _ => eprintln!("{}: command not found", command), 
        }
        
    }

    Ok(())
}
