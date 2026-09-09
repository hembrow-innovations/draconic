use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

use draconic_ast::print_program;
use draconic_diagnostics::Diagnostic;
use draconic_frontend::parse_source;

/// ROADMAP U05: `draconic fmt` — parse → deterministic reprint (in-place).
/// `--check` exits 1 when the file is not already formatted (no write).
pub fn cmd_fmt(args: &[String]) -> ExitCode {
    let mut check_only = false;
    let mut path: Option<PathBuf> = None;

    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                eprintln!("usage: draconic fmt [--check] <file>");
                return ExitCode::from(2);
            }
            "--check" => check_only = true,
            other if other.starts_with('-') => {
                eprintln!("unknown option: {other}");
                eprintln!("usage: draconic fmt [--check] <file>");
                return ExitCode::from(2);
            }
            other => {
                if path.is_some() {
                    eprintln!("usage: draconic fmt [--check] <file>");
                    return ExitCode::from(2);
                }
                path = Some(PathBuf::from(other));
            }
        }
    }

    let path = match path {
        Some(p) => p,
        None => {
            eprintln!("usage: draconic fmt [--check] <file>");
            return ExitCode::from(2);
        }
    };

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
        eprintln!("{}: would reformat", path.display());
        return ExitCode::from(1);
    }

    if source != formatted {
        if let Err(e) = fs::write(&path, &formatted) {
            eprintln!("failed to write {}: {e}", path.display());
            return ExitCode::from(1);
        }
    }
    ExitCode::SUCCESS
}

/// Format via Frontend Script-then-Module parse (no link).
fn format_source(source: &str) -> Result<String, Diagnostic> {
    Ok(print_program(&parse_source(source)?))
}
