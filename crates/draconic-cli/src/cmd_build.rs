use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use draconic_backend_js::emit_js;
use draconic_backend_llvm::{build_native_binary_with_lto, emit_llvm_ir_with_debug, SourceDebug};
use draconic_diagnostics::Diagnostic;
use draconic_frontend::compile_path;
use draconic_pkg::ensure_locked_for_entry;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Target {
    Js,
    Native,
}

#[derive(Debug)]
struct BuildArgs {
    target: Target,
    input: PathBuf,
    output: Option<PathBuf>,
    watch: bool,
    /// K07 / K07.02: cache-only package ensure; no network fetch on miss.
    offline: bool,
    /// F04.01: extra static archives (`.a`) for native link.
    link_libs: Vec<PathBuf>,
    /// D05.01: strip symbols from the native artifact.
    strip: bool,
    /// D05.02: LTO (size-opt) native link.
    lto: bool,
}

pub fn cmd_build(args: &[String]) -> ExitCode {
    let parsed = match parse_build_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!(
                "usage: draconic build --target js|native [--watch] [--offline] [--strip] [--lto] [--link <lib.a>] <file> [-o <out>]"
            );
            return ExitCode::from(2);
        }
    };

    let out = match &parsed.output {
        Some(p) => p.clone(),
        None => default_output(&parsed.input, parsed.target),
    };

    if let Err(code) = crate::toolchain_pin::enforce(&parsed.input) {
        return code;
    }

    if parsed.watch {
        return crate::watch::run_watch_loop(&parsed.input, || {
            build_program(
                &parsed.input,
                parsed.target,
                &out,
                parsed.offline,
                &parsed.link_libs,
                parsed.lto,
            )
            .map_err(|d| d.to_string())?;
            if parsed.strip {
                crate::strip_symbols::strip_native_binary(&out).map_err(|d| d.to_string())?;
            }
            Ok(())
        });
    }

    if let Err(d) = build_program(
        &parsed.input,
        parsed.target,
        &out,
        parsed.offline,
        &parsed.link_libs,
        parsed.lto,
    ) {
        eprintln!("error: {d}");
        return ExitCode::from(1);
    }
    if parsed.strip {
        if let Err(d) = crate::strip_symbols::strip_native_binary(&out) {
            eprintln!("error: {d}");
            return ExitCode::from(1);
        }
    }

    ExitCode::SUCCESS
}

fn parse_build_args(args: &[String]) -> Result<BuildArgs, String> {
    let mut target: Option<Target> = None;
    let mut output: Option<PathBuf> = None;
    let mut input: Option<PathBuf> = None;
    let mut watch = false;
    let mut offline = false;
    let mut strip = false;
    let mut lto = false;
    let mut link_libs: Vec<PathBuf> = Vec::new();

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
            "--watch" => watch = true,
            "--offline" => offline = true,
            "--strip" | "--strip-symbols" => strip = true,
            "--lto" => lto = true,
            "--link" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| "missing value for --link".to_string())?;
                link_libs.push(PathBuf::from(val));
            }
            l if let Some(rest) = l.strip_prefix("--link=") => {
                if rest.is_empty() {
                    return Err("missing value for --link".to_string());
                }
                link_libs.push(PathBuf::from(rest));
            }
            "-h" | "--help" => {
                return Err(
                    "usage: draconic build --target js|native [--watch] [--offline] [--strip] [--lto] [--link <lib.a>] <file> [-o <out>]".into(),
                );
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
    if strip && target != Target::Native {
        return Err("--strip is only valid with --target native".to_string());
    }
    if lto && target != Target::Native {
        return Err("--lto is only valid with --target native".to_string());
    }
    Ok(BuildArgs {
        target,
        input,
        output,
        watch,
        offline,
        link_libs,
        strip,
        lto,
    })
}

pub(crate) fn parse_target(s: &str) -> Result<Target, String> {
    match s {
        "js" => Ok(Target::Js),
        "native" => Ok(Target::Native),
        other => Err(format!("unknown target: {other} (expected js or native)")),
    }
}

fn default_output(input: &Path, target: Target) -> PathBuf {
    let stem = input.file_stem().and_then(|s| s.to_str()).unwrap_or("out");
    let parent = input.parent().unwrap_or_else(|| Path::new("."));
    match target {
        Target::Js => parent.join(format!("{stem}.out.js")),
        Target::Native => parent.join(format!("{stem}.out")),
    }
}

pub(crate) fn build_program(
    input: &Path,
    target: Target,
    out: &Path,
    offline: bool,
    link_libs: &[PathBuf],
    lto: bool,
) -> Result<(), Diagnostic> {
    // K07: auto-fetch missing locked package checkouts before link/compile.
    // K07.01: materialise missing pins. K07.02: `--offline` → cache only; miss → fixit.
    // K07.03: lock pins are authoritative (commit OID); do not float versions.
    if let Err(e) = ensure_locked_for_entry(input, offline) {
        return Err(Diagnostic::new(
            e.to_string(),
            draconic_diagnostics::Span::dummy(),
        ));
    }

    let module = compile_path(input)?;

    match target {
        Target::Js => {
            if !link_libs.is_empty() {
                return Err(Diagnostic::new(
                    "--link is only valid with --target native",
                    draconic_diagnostics::Span::dummy(),
                ));
            }
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
            let source = fs::read_to_string(input).map_err(|e| {
                Diagnostic::new(
                    format!("read {}: {e}", input.display()),
                    draconic_diagnostics::Span::dummy(),
                )
            })?;
            let debug = SourceDebug::from_path(input, source);
            let ll = emit_llvm_ir_with_debug(&module, &debug)?;
            build_native_binary_with_lto(&ll, out, link_libs, lto)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(!p.watch);
    }

    #[test]
    fn parse_build_args_watch() {
        let args = vec![
            "--target".into(),
            "js".into(),
            "--watch".into(),
            "a.drac".into(),
        ];
        let p = parse_build_args(&args).unwrap();
        assert!(p.watch);
        assert_eq!(p.input, PathBuf::from("a.drac"));
    }

    #[test]
    fn parse_build_args_requires_target() {
        let args = vec!["a.drac".into()];
        let err = parse_build_args(&args).unwrap_err();
        assert!(err.contains("target"), "{err}");
    }

    #[test]
    fn parse_build_args_link_static() {
        let args = vec![
            "--target".into(),
            "native".into(),
            "--link".into(),
            "libfoo.a".into(),
            "--link=libbar.a".into(),
            "a.drac".into(),
        ];
        let p = parse_build_args(&args).unwrap();
        assert_eq!(p.target, Target::Native);
        assert_eq!(
            p.link_libs,
            vec![PathBuf::from("libfoo.a"), PathBuf::from("libbar.a")]
        );
        assert_eq!(p.input, PathBuf::from("a.drac"));
    }

    #[test]
    fn default_output_paths() {
        let input = Path::new("/tmp/hello.drac");
        assert_eq!(
            default_output(input, Target::Js),
            PathBuf::from("/tmp/hello.out.js")
        );
        assert_eq!(
            default_output(input, Target::Native),
            PathBuf::from("/tmp/hello.out")
        );
    }

    #[test]
    fn build_program_js_smoke() {
        let dir = std::env::temp_dir().join(format!("draconic-cli-unit-js-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let out = dir.join("t.js");
        let input = dir.join("t.drac");
        fs::write(&input, "let x = 1;").unwrap();
        build_program(&input, Target::Js, &out, false, &[], false).unwrap();
        let js = fs::read_to_string(&out).unwrap();
        assert!(js.contains("let x"));
        let _ = fs::remove_dir_all(&dir);
    }
}
