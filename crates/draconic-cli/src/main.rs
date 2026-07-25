use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprint_usage();
        return ExitCode::from(2);
    }

    let cmd = args.remove(0);
    match cmd.as_str() {
        "parse" => cmd_parse(&args),
        "help" | "-h" | "--help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        "version" | "-V" | "--version" => {
            println!("draconic {}", env!("CARGO_PKG_VERSION"));
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command: {other}");
            eprint_usage();
            ExitCode::from(2)
        }
    }
}

fn cmd_parse(args: &[String]) -> ExitCode {
    let path = match args.first() {
        Some(p) => p,
        None => {
            eprintln!("usage: draconic parse <file>");
            return ExitCode::from(2);
        }
    };
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

fn print_usage() {
    println!(
        "\
draconic — the Draconic toolchain

Usage:
  draconic parse <file>   Parse a Program and print the AST dump
  draconic version        Print version
  draconic help           Show this help
"
    );
}

fn eprint_usage() {
    eprintln!("Run `draconic help` for usage.");
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
