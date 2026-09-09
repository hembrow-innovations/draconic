use std::fs;
use std::path::Path;
use std::process::ExitCode;

pub fn cmd_parse(args: &[String]) -> ExitCode {
    let path = match args.first() {
        Some(p) => p,
        None => {
            eprintln!("usage: draconic parse <file>");
            return ExitCode::from(2);
        }
    };
    if let Err(code) = crate::toolchain_pin::enforce(Path::new(path)) {
        return code;
    }
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {path}: {e}");
            return ExitCode::from(1);
        }
    };
    match draconic_parser::parse_and_dump(&source) {
        Ok(dump) => {
            print!("{dump}");
            ExitCode::SUCCESS
        }
        Err(d) => {
            eprintln!("error: {d}");
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use draconic_parser::parse_and_dump;

    #[test]
    fn parse_sample_program() {
        let dump = parse_and_dump("let x = 1 + 2;").unwrap();
        assert!(dump.starts_with("Program\n"));
        assert!(dump.contains("name: x"));
    }
}
