use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

/// ROADMAP F07 / F07.03: `draconic bindgen <header>` — write Draconic `extern "C"` module.
pub fn cmd_bindgen(args: &[String]) -> ExitCode {
    const USAGE: &str = "usage: draconic bindgen <header> [-o <out>]";
    let mut output: Option<PathBuf> = None;
    let mut path: Option<PathBuf> = None;
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                eprintln!("{USAGE}");
                return ExitCode::from(2);
            }
            "-o" | "--output" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    eprintln!("{USAGE}");
                    return ExitCode::from(2);
                };
                output = Some(PathBuf::from(v));
            }
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
        i += 1;
    }

    let path = match path {
        Some(p) => p,
        None => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    let source = match fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {}: {e}", path.display());
            return ExitCode::from(1);
        }
    };

    let header = match draconic_cli::c_header::parse_header(&source) {
        Ok(h) => h,
        Err(e) => {
            eprintln!("bindgen: {e}");
            return ExitCode::from(1);
        }
    };
    let rendered = draconic_cli::c_header::emit_externs(&header);
    let dest = output.unwrap_or_else(|| draconic_cli::c_header::default_extern_module_path(&path));
    if let Err(e) = fs::write(&dest, &rendered) {
        eprintln!("failed to write {}: {e}", dest.display());
        return ExitCode::from(1);
    }
    ExitCode::SUCCESS
}
