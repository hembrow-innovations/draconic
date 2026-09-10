use std::fs;
use std::path::Path;
use std::process::ExitCode;

use draconic_ast::dump_program;
use draconic_frontend::parse_source;

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
    match parse_source(&source) {
        Ok(program) => {
            print!("{}", dump_program(&program));
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
    use draconic_ast::dump_program;
    use draconic_frontend::parse_source;

    #[test]
    fn parse_sample_program() {
        let dump = dump_program(&parse_source("let x = 1 + 2;").unwrap());
        assert!(dump.starts_with("Program\n"));
        assert!(dump.contains("name: x"));
    }

    #[test]
    fn parse_retries_module_on_export() {
        let dump = dump_program(&parse_source("export default 1;").unwrap());
        assert!(dump.starts_with("Program\n"));
    }
}
