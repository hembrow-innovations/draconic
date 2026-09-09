use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

/// ROADMAP U12: `draconic doc` — extract `/** … */` docs → markdown or HTML.
pub fn cmd_doc(args: &[String]) -> ExitCode {
    let mut format = crate::doc::DocFormat::Markdown;
    let mut output: Option<PathBuf> = None;
    let mut path: Option<PathBuf> = None;
    let mut i = 0usize;

    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                eprintln!("usage: draconic doc [--format md|html] [-o <out>] <file>");
                return ExitCode::from(2);
            }
            "--format" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    eprintln!("usage: draconic doc [--format md|html] [-o <out>] <file>");
                    return ExitCode::from(2);
                };
                match crate::doc::DocFormat::parse(v) {
                    Some(f) => format = f,
                    None => {
                        eprintln!("unknown format: {v} (expected md or html)");
                        return ExitCode::from(2);
                    }
                }
            }
            "-o" | "--output" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    eprintln!("usage: draconic doc [--format md|html] [-o <out>] <file>");
                    return ExitCode::from(2);
                };
                output = Some(PathBuf::from(v));
            }
            other if other.starts_with('-') => {
                eprintln!("unknown option: {other}");
                eprintln!("usage: draconic doc [--format md|html] [-o <out>] <file>");
                return ExitCode::from(2);
            }
            other => {
                if path.is_some() {
                    eprintln!("usage: draconic doc [--format md|html] [-o <out>] <file>");
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
            eprintln!("usage: draconic doc [--format md|html] [-o <out>] <file>");
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

    let title = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("Program");
    let items = crate::doc::extract_docs(&source);
    let rendered = match format {
        crate::doc::DocFormat::Markdown => crate::doc::render_markdown(title, &items),
        crate::doc::DocFormat::Html => crate::doc::render_html(title, &items),
    };

    if let Some(out) = output {
        if let Err(e) = fs::write(&out, &rendered) {
            eprintln!("failed to write {}: {e}", out.display());
            return ExitCode::from(1);
        }
    } else {
        print!("{rendered}");
    }
    ExitCode::SUCCESS
}
