use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::ExitCode;

use draconic_ast::print_program;
use draconic_diagnostics::Diagnostic;
use draconic_frontend::parse_source;

const USAGE: &str = "usage: draconic fmt [--check] (<file> | --stdin)";

/// ROADMAP U05: `draconic fmt` — parse → deterministic reprint (in-place).
/// `--check` exits 1 when the file is not already formatted (no write).
pub fn cmd_fmt(args: &[String]) -> ExitCode {
    let mut check_only = false;
    let mut stdin_mode = false;
    let mut path: Option<PathBuf> = None;

    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
            "--check" => check_only = true,
            "--stdin" => stdin_mode = true,
            other if other.starts_with('-') => {
                eprintln!("unknown option: {other}");
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
            other => {
                if path.is_some() {
                    eprintln!("{USAGE}");
                    return ExitCode::from(2);
                }
                path = Some(PathBuf::from(other));
            }
        }
    }

    if stdin_mode == path.is_some() {
        eprintln!("{USAGE}");
        return ExitCode::from(2);
    }

    let (source, label, pin) = if stdin_mode {
        if let Err(code) = crate::toolchain_pin::enforce(std::path::Path::new(".")) {
            return code;
        }
        let mut buf = String::new();
        if let Err(e) = io::stdin().read_to_string(&mut buf) {
            eprintln!("failed to read stdin: {e}");
            return ExitCode::from(1);
        }
        (buf, "-".to_string(), PathBuf::from("."))
    } else {
        let path = path.expect("file xor --stdin");
        if let Err(code) = crate::toolchain_pin::enforce(&path) {
            return code;
        }
        let source = match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("failed to read {}: {e}", path.display());
                return ExitCode::from(1);
            }
        };
        let label = path.display().to_string();
        (source, label, path)
    };

    let formatted = match format_source(&source) {
        Ok(s) => s,
        Err(d) => {
            eprintln!("error: {d}");
            return ExitCode::from(1);
        }
    };

    if check_only {
        if source == formatted {
            return ExitCode::SUCCESS;
        }
        eprintln!("{label}: would reformat");
        return ExitCode::from(1);
    }

    if stdin_mode {
        if let Err(e) = io::stdout().write_all(formatted.as_bytes()) {
            eprintln!("failed to write stdout: {e}");
            return ExitCode::from(1);
        }
        return ExitCode::SUCCESS;
    }

    if source != formatted {
        if let Err(e) = fs::write(&pin, &formatted) {
            eprintln!("failed to write {}: {e}", pin.display());
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}

/// Format via Frontend Script-then-Module parse (no link).
fn format_source(source: &str) -> Result<String, Diagnostic> {
    Ok(print_program(&parse_source(source)?))
}
