use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use draconic_backend_js::emit_js;
use draconic_backend_llvm::{build_native_binary, emit_llvm_ir};
use draconic_check::check;
use draconic_conformance::{load_path, run_fixture};
use draconic_diagnostics::Diagnostic;
use draconic_ir::lower;
use draconic_parser::{link_entry, parse};

fn main() -> ExitCode {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprint_usage();
        return ExitCode::from(2);
    }

    let cmd = args.remove(0);
    match cmd.as_str() {
        "parse" => cmd_parse(&args),
        "build" => cmd_build(&args),
        "test" => cmd_test(&args),
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Target {
    Js,
    Native,
}

#[derive(Debug)]
struct BuildArgs {
    target: Target,
    input: PathBuf,
    output: Option<PathBuf>,
}

fn cmd_build(args: &[String]) -> ExitCode {
    let parsed = match parse_build_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: draconic build --target js|native <file> [-o <out>]");
            return ExitCode::from(2);
        }
    };

    let source = match fs::read_to_string(&parsed.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {}: {e}", parsed.input.display());
            return ExitCode::from(1);
        }
    };

    let out = match &parsed.output {
        Some(p) => p.clone(),
        None => default_output(&parsed.input, parsed.target),
    };

    if let Err(d) = build_program(&source, &parsed.input, parsed.target, &out) {
        eprintln!("error: {d}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
}

fn parse_build_args(args: &[String]) -> Result<BuildArgs, String> {
    let mut target: Option<Target> = None;
    let mut output: Option<PathBuf> = None;
    let mut input: Option<PathBuf> = None;

    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "--target" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| "missing value for --target".to_string())?;
                target = Some(parse_target(val)?);
            }
            t if let Some(rest) = t.strip_prefix("--target=") => {
                target = Some(parse_target(rest)?);
            }
            "-o" | "--out" | "--output" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| "missing value for -o".to_string())?;
                output = Some(PathBuf::from(val));
            }
            o if let Some(rest) = o.strip_prefix("--out=") => {
                output = Some(PathBuf::from(rest));
            }
            o if let Some(rest) = o.strip_prefix("--output=") => {
                output = Some(PathBuf::from(rest));
            }
            "-h" | "--help" => {
                return Err("usage: draconic build --target js|native <file> [-o <out>]".into());
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                if input.is_some() {
                    return Err(format!("unexpected argument: {other}"));
                }
                input = Some(PathBuf::from(other));
            }
        }
        i += 1;
    }

    let target = target.ok_or_else(|| "missing required --target js|native".to_string())?;
    let input = input.ok_or_else(|| "missing input file".to_string())?;
    Ok(BuildArgs {
        target,
        input,
        output,
    })
}

fn parse_target(s: &str) -> Result<Target, String> {
    match s {
        "js" => Ok(Target::Js),
        "native" => Ok(Target::Native),
        other => Err(format!(
            "unknown target: {other} (expected js or native)"
        )),
    }
}

fn default_output(input: &Path, target: Target) -> PathBuf {
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("out");
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    match target {
        Target::Js => parent.join(format!("{stem}.js")),
        Target::Native => parent.join(stem),
    }
}

fn build_program(
    source: &str,
    input: &Path,
    target: Target,
    out: &Path,
) -> Result<(), Diagnostic> {
    let program = if source.contains("import ") || source.contains("export ") {
        link_entry(input)?
    } else {
        parse(source)?
    };
    let checked = check(program)?;
    let module = lower(&checked);

    match target {
        Target::Js => {
            let js = emit_js(&module)?;
            if let Some(parent) = out.parent() {
                if !parent.as_os_str().is_empty() {
                    fs::create_dir_all(parent).map_err(|e| {
                        Diagnostic::new(
                            format!("create output dir failed: {e}"),
                            draconic_diagnostics::Span::dummy(),
                        )
                    })?;
                }
            }
            fs::write(out, js).map_err(|e| {
                Diagnostic::new(
                    format!("write JS output failed: {e}"),
                    draconic_diagnostics::Span::dummy(),
                )
            })?;
        }
        Target::Native => {
            let ll = emit_llvm_ir(&module)?;
            build_native_binary(&ll, out)?;
        }
    }
    Ok(())
}

fn cmd_test(args: &[String]) -> ExitCode {
    let path = match args.first() {
        Some(p) if p != "-h" && p != "--help" => PathBuf::from(p),
        _ => {
            eprintln!("usage: draconic test <path>");
            eprintln!("  <path>  fixture directory or single .drac file (with optional .meta)");
            return ExitCode::from(2);
        }
    };

    if args.len() > 1 {
        eprintln!("usage: draconic test <path>");
        return ExitCode::from(2);
    }

    let fixtures = match load_path(&path) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };

    if fixtures.is_empty() {
        eprintln!("error: no .drac fixtures under {}", path.display());
        return ExitCode::from(1);
    }

    let mut passed = 0u32;
    let mut failed = 0u32;
    for fixture in &fixtures {
        for result in run_fixture(fixture) {
            if result.ok {
                passed += 1;
                println!(
                    "ok {} {}",
                    result.fixture_id,
                    result.target.as_str()
                );
            } else {
                failed += 1;
                println!(
                    "FAIL {} {}: {}",
                    result.fixture_id,
                    result.target.as_str(),
                    result.message
                );
            }
        }
    }

    let total = passed + failed;
    if failed == 0 {
        println!("{passed} passed");
        ExitCode::SUCCESS
    } else {
        println!("{passed} passed, {failed} failed, {total} total");
        ExitCode::from(1)
    }
}

fn print_usage() {
    println!(
        "\
draconic — the Draconic toolchain

Usage:
  draconic parse <file>                          Parse a Program and print the AST dump
  draconic build --target js|native <file> [-o <out>]
                                                 Compile a Program to JS or a native binary
  draconic test <path>                           Run conformance fixtures (dir or .drac file)
  draconic version                               Print version
  draconic help                                  Show this help
"
    );
}

fn eprint_usage() {
    eprintln!("Run `draconic help` for usage.");
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_parser::parse_and_dump;

    #[test]
    fn parse_sample_program() {
        let dump = parse_and_dump("let x = 1 + 2;").unwrap();
        assert!(dump.starts_with("Program\n"));
        assert!(dump.contains("name: x"));
    }

    #[test]
    fn parse_build_args_js_with_out() {
        let args = vec![
            "--target".into(),
            "js".into(),
            "a.drac".into(),
            "-o".into(),
            "a.js".into(),
        ];
        let p = parse_build_args(&args).unwrap();
        assert_eq!(p.target, Target::Js);
        assert_eq!(p.input, PathBuf::from("a.drac"));
        assert_eq!(p.output, Some(PathBuf::from("a.js")));
    }

    #[test]
    fn parse_build_args_requires_target() {
        let args = vec!["a.drac".into()];
        let err = parse_build_args(&args).unwrap_err();
        assert!(err.contains("target"), "{err}");
    }

    #[test]
    fn default_output_paths() {
        let input = Path::new("/tmp/hello.drac");
        assert_eq!(
            default_output(input, Target::Js),
            PathBuf::from("/tmp/hello.js")
        );
        assert_eq!(
            default_output(input, Target::Native),
            PathBuf::from("/tmp/hello")
        );
    }

    #[test]
    fn build_program_js_smoke() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-cli-unit-js-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let out = dir.join("t.js");
        let input = dir.join("t.drac");
        build_program("let x = 1;", &input, Target::Js, &out).unwrap();
        let js = fs::read_to_string(&out).unwrap();
        assert!(js.contains("let x"));
        let _ = fs::remove_dir_all(&dir);
    }
}
