use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_ast::print_program;
use draconic_backend_js::emit_js;
use draconic_backend_llvm::{build_native_binary_with_lto, emit_llvm_ir_with_debug, SourceDebug};
use draconic_diagnostics::Diagnostic;
use draconic_embed::{eval_source, EmbedValue};
use draconic_frontend::{check_path, compile_path, compile_source};
use draconic_ir::Stmt;
use draconic_parser::{parse, parse_module};
use draconic_pkg::ensure_locked_for_entry;

mod cmd_test;
mod doc;
mod extract;
mod strip_symbols;
mod toolchain_pin;

fn main() -> ExitCode {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprint_usage();
        return ExitCode::from(2);
    }

    let cmd = args.remove(0);
    match cmd.as_str() {
        "parse" => cmd_parse(&args),
        "check" => cmd_check(&args),
        "fmt" => cmd_fmt(&args),
        "doc" => cmd_doc(&args),
        "extract" => extract::cmd_extract(&args),
        "build" => cmd_build(&args),
        "run" => cmd_run(&args),
        "repl" => cmd_repl(&args),
        "test" => cmd_test::cmd_test(&args),
        "get" => cmd_get(&args),
        "mod" => cmd_mod(&args),
        "bindgen" => cmd_bindgen(&args),
        "help" | "-h" | "--help" => {
            print_usage();
            ExitCode::SUCCESS
        }
        "version" | "-V" | "--version" => {
            print!("{}", verbose_version());
            ExitCode::SUCCESS
        }
        // Shebang-friendly: `#!/usr/bin/env draconic` → `draconic <script> [args…]`
        other if looks_like_script_path(other) => {
            let mut run_args = Vec::with_capacity(args.len() + 1);
            run_args.push(other.to_string());
            run_args.extend(args);
            cmd_run(&run_args)
        }
        other => {
            eprintln!("unknown command: {other}");
            eprint_usage();
            ExitCode::from(2)
        }
    }
}

/// True when `arg` should be treated as a Program path (shebang / bare-file invoke).
fn looks_like_script_path(arg: &str) -> bool {
    if arg.starts_with('-') {
        return false;
    }
    let path = Path::new(arg);
    if path.is_file() {
        return true;
    }
    // Allow paths that clearly look like sources even if missing (better error later).
    matches!(
        path.extension().and_then(|e| e.to_str()),
        Some("drac") | Some("js")
    ) || arg.contains('/')
        || arg.contains('\\')
}

fn cmd_parse(args: &[String]) -> ExitCode {
    let path = match args.first() {
        Some(p) => p,
        None => {
            eprintln!("usage: draconic parse <file>");
            return ExitCode::from(2);
        }
    };
    if let Err(code) = toolchain_pin::enforce(Path::new(path)) {
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

fn cmd_check(args: &[String]) -> ExitCode {
    let parsed = match parse_check_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: draconic check [--watch] <file>");
            return ExitCode::from(2);
        }
    };

    if let Err(code) = toolchain_pin::enforce(&parsed.input) {
        return code;
    }

    if parsed.watch {
        return run_watch_loop(&parsed.input, || match check_path(&parsed.input) {
            Ok(_) => {
                touch_watch_marker();
                Ok(())
            }
            Err(d) => Err(d.to_string()),
        });
    }

    match check_path(&parsed.input) {
        Ok(_) => ExitCode::SUCCESS,
        Err(d) => {
            eprintln!("error: {d}");
            ExitCode::from(1)
        }
    }
}

#[derive(Debug)]
struct CheckArgs {
    input: PathBuf,
    watch: bool,
}

fn parse_check_args(args: &[String]) -> Result<CheckArgs, String> {
    let mut watch = false;
    let mut input: Option<PathBuf> = None;

    for a in args {
        match a.as_str() {
            "-h" | "--help" => {
                return Err("usage: draconic check [--watch] <file>".into());
            }
            "--watch" => watch = true,
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
    }

    let input = input.ok_or_else(|| "missing input file".to_string())?;
    Ok(CheckArgs { input, watch })
}

/// ROADMAP F07.03: `draconic bindgen <header>` — write Draconic `extern "C"` module.
fn cmd_bindgen(args: &[String]) -> ExitCode {
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

/// ROADMAP U12: `draconic doc` — extract `/** … */` docs → markdown or HTML.
fn cmd_doc(args: &[String]) -> ExitCode {
    let mut format = doc::DocFormat::Markdown;
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
                match doc::DocFormat::parse(v) {
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

    if let Err(code) = toolchain_pin::enforce(&path) {
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
    let items = doc::extract_docs(&source);
    let rendered = match format {
        doc::DocFormat::Markdown => doc::render_markdown(title, &items),
        doc::DocFormat::Html => doc::render_html(title, &items),
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

/// ROADMAP U05: `draconic fmt` — parse → deterministic reprint (in-place).
/// `--check` exits 1 when the file is not already formatted (no write).
fn cmd_fmt(args: &[String]) -> ExitCode {
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

    if let Err(code) = toolchain_pin::enforce(&path) {
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

/// Parse Script-first, then Module (same policy as frontend load, without link).
fn format_source(source: &str) -> Result<String, Diagnostic> {
    let program = match parse(source) {
        Ok(p) => p,
        Err(script_err) => match parse_module(source) {
            Ok(p) => p,
            Err(_) => return Err(script_err),
        },
    };
    Ok(print_program(&program))
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
    watch: bool,
    /// K07.02: cache-only package ensure; no network fetch on miss.
    offline: bool,
    /// F04.01: extra static archives (`.a`) for native link.
    link_libs: Vec<PathBuf>,
    /// D05.01: strip symbols from the native artifact.
    strip: bool,
    /// D05.02: LTO (size-opt) native link.
    lto: bool,
}

fn cmd_build(args: &[String]) -> ExitCode {
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

    if let Err(code) = toolchain_pin::enforce(&parsed.input) {
        return code;
    }

    if parsed.watch {
        return run_watch_loop(&parsed.input, || {
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
                strip_symbols::strip_native_binary(&out).map_err(|d| d.to_string())?;
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
        if let Err(d) = strip_symbols::strip_native_binary(&out) {
            eprintln!("error: {d}");
            return ExitCode::from(1);
        }
    }

    ExitCode::SUCCESS
}

/// Poll `path` mtime and re-run `action` on change. Initial run is immediate.
/// Errors are printed; the loop continues. Exit with Ctrl-C (or kill in tests).
fn run_watch_loop(path: &Path, mut action: impl FnMut() -> Result<(), String>) -> ExitCode {
    let poll_ms = env::var("DRACONIC_WATCH_POLL_MS")
        .ok()
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(200)
        .max(10);

    eprintln!("watching {} (poll {poll_ms}ms)", path.display());

    let mut last_stamp = file_watch_stamp(path);
    if let Err(msg) = action() {
        eprintln!("error: {msg}");
    }

    loop {
        std::thread::sleep(std::time::Duration::from_millis(poll_ms));
        let stamp = file_watch_stamp(path);
        if stamp != last_stamp {
            last_stamp = stamp;
            if let Err(msg) = action() {
                eprintln!("error: {msg}");
            }
        }
    }
}

fn file_watch_stamp(path: &Path) -> Option<(u64, u64)> {
    let meta = fs::metadata(path).ok()?;
    let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
    let len = meta.len();
    Some((modified.as_secs(), modified.subsec_nanos() as u64 ^ len))
}

/// Test hook: when `DRACONIC_WATCH_MARKER` is set, write an incrementing counter
/// after each successful check (used by U10 integration tests).
fn touch_watch_marker() {
    let Some(path) = env::var_os("DRACONIC_WATCH_MARKER") else {
        return;
    };
    let path = PathBuf::from(path);
    let next = fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.trim().parse::<u64>().ok())
        .unwrap_or(0)
        .saturating_add(1);
    let _ = fs::write(&path, format!("{next}\n"));
}

/// ROADMAP U14: `draconic run` — build to a temp artifact and execute immediately.
/// Default target is `js` (Node). Use `--target native` for the LLVM path.
/// Remaining args after the input file are forwarded to the program.
fn cmd_run(args: &[String]) -> ExitCode {
    let parsed = match parse_run_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: draconic run [--target js|native] <file> [args...]");
            return ExitCode::from(2);
        }
    };

    if let Err(code) = toolchain_pin::enforce(&parsed.input) {
        return code;
    }

    let work = match run_work_dir() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(1);
        }
    };

    let artifact = match parsed.target {
        Target::Js => work.join("out.js"),
        Target::Native => work.join("out"),
    };

    if let Err(d) = build_program(&parsed.input, parsed.target, &artifact, false, &[], false) {
        let _ = fs::remove_dir_all(&work);
        eprintln!("error: {d}");
        return ExitCode::from(1);
    }

    let status = execute_artifact(parsed.target, &artifact, &parsed.program_args);
    let _ = fs::remove_dir_all(&work);

    match status {
        Ok(code) => exit_from_code(code),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

#[derive(Debug)]
struct RunArgs {
    target: Target,
    input: PathBuf,
    program_args: Vec<String>,
}

fn parse_run_args(args: &[String]) -> Result<RunArgs, String> {
    let mut target = Target::Js; // default for shebang / quick scripts
    let mut input: Option<PathBuf> = None;
    let mut program_args: Vec<String> = Vec::new();
    let mut i = 0;
    let mut saw_separator = false;

    while i < args.len() {
        let a = &args[i];
        if saw_separator {
            program_args.push(a.clone());
            i += 1;
            continue;
        }
        match a.as_str() {
            "--" => {
                saw_separator = true;
            }
            "--target" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| "missing value for --target".to_string())?;
                target = parse_target(val)?;
            }
            t if let Some(rest) = t.strip_prefix("--target=") => {
                target = parse_target(rest)?;
            }
            "-h" | "--help" => {
                return Err("usage: draconic run [--target js|native] <file> [args...]".into());
            }
            other if other.starts_with('-') && input.is_none() => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                if input.is_none() {
                    input = Some(PathBuf::from(other));
                } else {
                    // After the Program path, remaining tokens are program argv.
                    program_args.push(other.to_string());
                }
            }
        }
        i += 1;
    }

    let input = input.ok_or_else(|| "missing input file".to_string())?;
    Ok(RunArgs {
        target,
        input,
        program_args,
    })
}

fn run_work_dir() -> Result<PathBuf, String> {
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(0);
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    let dir = env::temp_dir().join(format!(
        "draconic-run-{}-{}-{}",
        std::process::id(),
        nanos,
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).map_err(|e| format!("create temp dir failed: {e}"))?;
    Ok(dir)
}

fn execute_artifact(
    target: Target,
    artifact: &Path,
    program_args: &[String],
) -> Result<i32, String> {
    let mut cmd = match target {
        Target::Js => {
            let mut c = Command::new("node");
            c.arg(artifact);
            c
        }
        Target::Native => Command::new(artifact),
    };
    cmd.args(program_args);
    // Inherit stdio so run feels like a real process (shebang-friendly).
    cmd.stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());
    let status = cmd.status().map_err(|e| match target {
        Target::Js => format!("spawn node failed: {e}"),
        Target::Native => format!("spawn binary failed: {e}"),
    })?;
    Ok(status.code().unwrap_or(1))
}

fn exit_from_code(code: i32) -> ExitCode {
    if code == 0 {
        ExitCode::SUCCESS
    } else if (1..=255).contains(&code) {
        ExitCode::from(code as u8)
    } else {
        ExitCode::from(1)
    }
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

fn parse_target(s: &str) -> Result<Target, String> {
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
        Target::Js => parent.join(format!("{stem}.js")),
        Target::Native => parent.join(stem),
    }
}

fn build_program(
    input: &Path,
    target: Target,
    out: &Path,
    offline: bool,
    link_libs: &[PathBuf],
    lto: bool,
) -> Result<(), Diagnostic> {
    // K07.01: auto-fetch missing locked package checkouts before link/compile.
    // K07.02: `--offline` → cache only; miss → fixit (no network).
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

/// ROADMAP U08: interactive read-eval-print (js default; optional embed).
/// Multi-line when parse fails with Eof; prints last expression value.
fn cmd_repl(args: &[String]) -> ExitCode {
    let target = match parse_repl_args(args) {
        Ok(t) => t,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: draconic repl [--target js|embed]");
            return ExitCode::from(2);
        }
    };

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if let Err(code) = toolchain_pin::enforce(&cwd) {
        return code;
    }

    let stdin = io::stdin();
    let mut stdin = stdin.lock();
    let mut stdout = io::stdout();
    let interactive = atty_stdout();

    let mut session = String::new();
    let mut buffer = String::new();

    loop {
        if interactive {
            let prompt = if buffer.is_empty() { "> " } else { "... " };
            let _ = write!(stdout, "{prompt}");
            let _ = stdout.flush();
        }

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                eprintln!("error: read stdin failed: {e}");
                return ExitCode::from(1);
            }
        }

        let trimmed = line.trim_end_matches(['\r', '\n']);
        if buffer.is_empty() {
            let t = trimmed.trim();
            if t.is_empty() {
                continue;
            }
            if matches!(t, ".exit" | ".quit") {
                break;
            }
        }

        if !buffer.is_empty() {
            buffer.push('\n');
        }
        buffer.push_str(trimmed);

        match repl_buffer_status(&buffer) {
            ReplBufferStatus::Incomplete => continue,
            ReplBufferStatus::Error(d) => {
                eprintln!("error: {d}");
                buffer.clear();
                continue;
            }
            ReplBufferStatus::Complete => {}
        }

        let chunk = std::mem::take(&mut buffer);
        match target {
            ReplTarget::Js => match repl_eval_js(&session, &chunk) {
                Ok(ReplEval {
                    printed,
                    new_session,
                }) => {
                    if let Some(text) = printed {
                        println!("{text}");
                    }
                    session = new_session;
                }
                Err(msg) => eprintln!("error: {msg}"),
            },
            ReplTarget::Embed => match repl_eval_embed(&chunk) {
                Ok(Some(text)) => println!("{text}"),
                Ok(None) => {}
                Err(msg) => eprintln!("error: {msg}"),
            },
        }
    }

    ExitCode::SUCCESS
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReplTarget {
    Js,
    Embed,
}

fn parse_repl_args(args: &[String]) -> Result<ReplTarget, String> {
    let mut target = ReplTarget::Js;
    let mut i = 0;
    while i < args.len() {
        let a = &args[i];
        match a.as_str() {
            "-h" | "--help" => {
                return Err("usage: draconic repl [--target js|embed]".into());
            }
            "--target" => {
                i += 1;
                let val = args
                    .get(i)
                    .ok_or_else(|| "missing value for --target".to_string())?;
                target = parse_repl_target(val)?;
            }
            t if let Some(rest) = t.strip_prefix("--target=") => {
                target = parse_repl_target(rest)?;
            }
            other => {
                return Err(format!("unknown option: {other}"));
            }
        }
        i += 1;
    }
    Ok(target)
}

fn parse_repl_target(s: &str) -> Result<ReplTarget, String> {
    match s {
        "js" => Ok(ReplTarget::Js),
        "embed" => Ok(ReplTarget::Embed),
        other => Err(format!("unknown target: {other} (expected js or embed)")),
    }
}

enum ReplBufferStatus {
    Incomplete,
    Complete,
    Error(Diagnostic),
}

fn repl_buffer_status(source: &str) -> ReplBufferStatus {
    match parse(source) {
        Ok(_) => ReplBufferStatus::Complete,
        Err(d) => {
            let msg = d.to_string();
            if msg.contains("Eof") || msg.contains("end of file") || msg.contains("end of input") {
                ReplBufferStatus::Incomplete
            } else {
                // Script may fail where Module would succeed; try module before rejecting.
                match parse_module(source) {
                    Ok(_) => ReplBufferStatus::Complete,
                    Err(d2) => {
                        let msg2 = d2.to_string();
                        if msg2.contains("Eof")
                            || msg2.contains("end of file")
                            || msg2.contains("end of input")
                        {
                            ReplBufferStatus::Incomplete
                        } else {
                            ReplBufferStatus::Error(d)
                        }
                    }
                }
            }
        }
    }
}

struct ReplEval {
    printed: Option<String>,
    new_session: String,
}

fn repl_eval_js(session: &str, chunk: &str) -> Result<ReplEval, String> {
    let full = if session.is_empty() {
        chunk.to_string()
    } else {
        format!("{session}\n{chunk}")
    };

    let module = compile_source(&full).map_err(|d| d.to_string())?;
    let (js, has_last_expr) = emit_js_repl(&module).map_err(|d| d.to_string())?;

    let work = run_work_dir()?;
    let artifact = work.join("repl.js");
    fs::write(&artifact, &js).map_err(|e| format!("write temp JS failed: {e}"))?;

    let output = Command::new("node")
        .arg(&artifact)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("spawn node failed: {e}"))?;
    let _ = fs::remove_dir_all(&work);

    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    if !output.status.success() {
        let msg = stderr.trim();
        if msg.is_empty() {
            return Err(format!("node exited {}", output.status.code().unwrap_or(1)));
        }
        return Err(msg.to_string());
    }

    // User console output (if any) already on stdout of node; last value is last line when we printed.
    // We only inject a trailing print for last expression — surface it as the REPL result.
    let printed = if has_last_expr {
        let line = stdout.lines().last().unwrap_or("").trim();
        // If the program itself printed, show full stdout; last line is still the value.
        if stdout.lines().count() > 1 {
            print!("{stdout}");
            // Avoid double-printing last line via println below when we already printed all.
            // Actually we printed full stdout including last value — return None.
            let _ = line;
            None
        } else if line.is_empty() {
            Some("undefined".to_string())
        } else {
            Some(line.to_string())
        }
    } else if !stdout.is_empty() {
        print!("{stdout}");
        None
    } else {
        None
    };

    Ok(ReplEval {
        printed,
        new_session: full,
    })
}

/// Emit JS for REPL: if last top-level stmt is an expression, assign/print its value.
fn emit_js_repl(module: &draconic_ir::Module) -> Result<(String, bool), Diagnostic> {
    let mut module = module.clone();
    let has_last_expr = matches!(module.body.last(), Some(Stmt::Expr { .. }));
    if has_last_expr {
        let expr_stmt = module.body.pop().expect("last expr");
        let Stmt::Expr { expr } = expr_stmt else {
            unreachable!();
        };
        if !module.body_spans.is_empty() {
            module.body_spans.pop();
        }
        let prefix = emit_js(&module)?;
        let expr_only = draconic_ir::Module {
            locals: module.locals.clone(),
            body: vec![Stmt::Expr { expr }],
            body_spans: vec![draconic_diagnostics::Span::dummy()],
            shapes: module.shapes.clone(),
            has_extern_ffi: module.has_extern_ffi,
        };
        let expr_js = emit_js(&expr_only)?;
        let expr_js = expr_js.trim().trim_end_matches(';').trim();
        let mut out = String::new();
        out.push_str(&prefix);
        if !prefix.is_empty() && !prefix.ends_with('\n') {
            out.push('\n');
        }
        // Inspect so strings/objects print like a REPL (not raw console.log quotes only).
        out.push_str("const __draconic_util = require(\"util\");\n");
        out.push_str("console.log(__draconic_util.inspect((\n");
        out.push_str(expr_js);
        out.push_str("\n), { depth: null, colors: false, compact: true }));\n");
        Ok((out, true))
    } else {
        Ok((emit_js(&module)?, false))
    }
}

fn repl_eval_embed(chunk: &str) -> Result<Option<String>, String> {
    let value = eval_source(chunk).map_err(|d| d.to_string())?;
    Ok(Some(format_embed_value(&value)))
}

fn format_embed_value(v: &EmbedValue) -> String {
    match v {
        EmbedValue::Undefined => "undefined".to_string(),
        EmbedValue::Null => "null".to_string(),
        EmbedValue::Boolean(b) => b.to_string(),
        EmbedValue::Number(n) => {
            if n.is_nan() {
                "NaN".to_string()
            } else if *n == f64::INFINITY {
                "Infinity".to_string()
            } else if *n == f64::NEG_INFINITY {
                "-Infinity".to_string()
            } else if *n == 0.0 && n.is_sign_negative() {
                "-0".to_string()
            } else {
                // Prefer integer display when exact.
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    format!("{}", *n as i64)
                } else {
                    format!("{n}")
                }
            }
        }
        EmbedValue::String(s) => format!("'{s}'"),
    }
}

fn atty_stdout() -> bool {
    // Avoid extra crate: treat non-piped CI/tests as non-interactive (no prompts).
    #[cfg(unix)]
    {
        use std::os::unix::io::AsRawFd;
        extern "C" {
            fn isatty(fd: i32) -> i32;
        }
        // SAFETY: POSIX isatty on a valid stdout fd.
        unsafe { isatty(io::stdout().as_raw_fd()) != 0 }
    }
    #[cfg(not(unix))]
    {
        false
    }
}

/// Verbose version text for `draconic -V` / `--version` / `version` (U13).
fn verbose_version() -> String {
    let pkg = env!("CARGO_PKG_VERSION");
    let commit = env!("DRACONIC_GIT_COMMIT");
    let host = env!("DRACONIC_TARGET");
    let llvm = detect_llvm_version().unwrap_or_else(|| "unknown".to_string());
    format!(
        "draconic {pkg}\n\
         commit: {commit}\n\
         host: {host}\n\
         LLVM: {llvm}\n"
    )
}

fn detect_llvm_version() -> Option<String> {
    for bin in [
        "llvm-config",
        "/opt/homebrew/opt/llvm@22/bin/llvm-config",
        "/opt/homebrew/opt/llvm/bin/llvm-config",
        "/usr/local/opt/llvm/bin/llvm-config",
    ] {
        if let Some(v) = run_version_line(bin, &["--version"]) {
            let v = v.trim();
            if !v.is_empty() {
                return Some(v.to_string());
            }
        }
    }

    let mut clang_candidates: Vec<String> = Vec::new();
    if let Ok(p) = env::var("CLANG") {
        clang_candidates.push(p);
    }
    clang_candidates.extend(
        [
            "clang",
            "/usr/bin/clang",
            "/opt/homebrew/opt/llvm@22/bin/clang",
            "/opt/homebrew/opt/llvm/bin/clang",
        ]
        .into_iter()
        .map(str::to_string),
    );

    for clang in clang_candidates {
        if let Some(text) = run_version_text(&clang, &["--version"]) {
            if let Some(v) = parse_clang_llvm_version(&text) {
                return Some(v);
            }
        }
    }
    None
}

fn run_version_line(bin: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(bin)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let text = String::from_utf8_lossy(&output.stdout);
    text.lines().next().map(|l| l.trim().to_string())
}

fn run_version_text(bin: &str, args: &[&str]) -> Option<String> {
    let output = std::process::Command::new(bin)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    if text.trim().is_empty() {
        text = String::from_utf8_lossy(&output.stderr).into_owned();
    }
    Some(text)
}

fn parse_clang_llvm_version(text: &str) -> Option<String> {
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(idx) = lower.find("llvm version") {
            let rest = line[idx + "llvm version".len()..].trim();
            let ver = rest.split_whitespace().next().unwrap_or("").trim();
            if !ver.is_empty() {
                return Some(ver.to_string());
            }
        }
    }
    for line in text.lines() {
        let lower = line.to_ascii_lowercase();
        if let Some(idx) = lower.find("version ") {
            let rest = &line[idx + "version ".len()..];
            let ver = rest
                .split(|c: char| c.is_whitespace() || c == '(')
                .next()
                .unwrap_or("")
                .trim();
            if !ver.is_empty() && ver.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return Some(ver.to_string());
            }
        }
    }
    None
}

/// ROADMAP K05.02: `draconic mod tidy` — lock matches manifest; fetch missing; prune unused.
fn cmd_mod(args: &[String]) -> ExitCode {
    let sub = match args.first().map(String::as_str) {
        Some("tidy") => "tidy",
        Some(other) => {
            eprintln!("unknown mod subcommand: {other}");
            eprintln!("usage: draconic mod tidy [--dir <path>] [--cache-dir <path>]");
            return ExitCode::from(2);
        }
        None => {
            eprintln!("usage: draconic mod tidy [--dir <path>] [--cache-dir <path>]");
            return ExitCode::from(2);
        }
    };
    debug_assert_eq!(sub, "tidy");
    let rest = &args[1..];
    let parsed = match parse_mod_tidy_args(rest) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: draconic mod tidy [--dir <path>] [--cache-dir <path>]");
            return ExitCode::from(2);
        }
    };

    let workspace = parsed
        .dir
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if let Err(code) = toolchain_pin::enforce(&workspace) {
        return code;
    }
    let cache_root = parsed
        .cache_dir
        .unwrap_or_else(|| draconic_pkg::default_cache_root(&workspace));
    let cache = draconic_pkg::ModuleCache::new(cache_root);

    match draconic_pkg::mod_tidy(&workspace, &cache) {
        Ok(r) => {
            println!(
                "mod tidy: kept {} fetched {} pruned {}",
                r.kept.len(),
                r.fetched.len(),
                r.pruned.len()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

#[derive(Debug)]
struct ModTidyArgs {
    dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
}

fn parse_mod_tidy_args(args: &[String]) -> Result<ModTidyArgs, String> {
    let mut dir: Option<PathBuf> = None;
    let mut cache_dir: Option<PathBuf> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                return Err("usage: draconic mod tidy [--dir <path>] [--cache-dir <path>]".into());
            }
            "--dir" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("missing value for --dir".into());
                };
                dir = Some(PathBuf::from(v));
            }
            t if let Some(rest) = t.strip_prefix("--dir=") => {
                dir = Some(PathBuf::from(rest));
            }
            "--cache-dir" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("missing value for --cache-dir".into());
                };
                cache_dir = Some(PathBuf::from(v));
            }
            t if let Some(rest) = t.strip_prefix("--cache-dir=") => {
                cache_dir = Some(PathBuf::from(rest));
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                return Err(format!("unexpected argument: {other}"));
            }
        }
        i += 1;
    }
    Ok(ModTidyArgs { dir, cache_dir })
}

/// ROADMAP K05.01: `draconic get <module_path>@<ver>` — fetch, update manifest+lock+cache.
fn cmd_get(args: &[String]) -> ExitCode {
    let parsed = match parse_get_args(args) {
        Ok(p) => p,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!(
                "usage: draconic get <module_path>@<ver> [--url <git-url>] [--dir <path>] [--cache-dir <path>]"
            );
            return ExitCode::from(2);
        }
    };

    let workspace = parsed
        .dir
        .unwrap_or_else(|| env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
    if let Err(code) = toolchain_pin::enforce(&workspace) {
        return code;
    }
    let cache_root = parsed
        .cache_dir
        .unwrap_or_else(|| draconic_pkg::default_cache_root(&workspace));
    let cache = draconic_pkg::ModuleCache::new(cache_root);

    match draconic_pkg::get_package_spec(&workspace, &parsed.spec, parsed.url.as_deref(), &cache) {
        Ok(r) => {
            println!(
                "got {}@{} (resolved {}) oid={}",
                r.path, r.version_req, r.resolved_version, r.commit_oid
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(1)
        }
    }
}

#[derive(Debug)]
struct GetArgs {
    spec: String,
    url: Option<String>,
    dir: Option<PathBuf>,
    cache_dir: Option<PathBuf>,
}

fn parse_get_args(args: &[String]) -> Result<GetArgs, String> {
    let mut spec: Option<String> = None;
    let mut url: Option<String> = None;
    let mut dir: Option<PathBuf> = None;
    let mut cache_dir: Option<PathBuf> = None;
    let mut i = 0usize;
    while i < args.len() {
        match args[i].as_str() {
            "-h" | "--help" => {
                return Err(
                    "usage: draconic get <module_path>@<ver> [--url <git-url>] [--dir <path>] [--cache-dir <path>]"
                        .into(),
                );
            }
            "--url" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("missing value for --url".into());
                };
                url = Some(v.clone());
            }
            t if let Some(rest) = t.strip_prefix("--url=") => {
                url = Some(rest.to_string());
            }
            "--dir" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("missing value for --dir".into());
                };
                dir = Some(PathBuf::from(v));
            }
            t if let Some(rest) = t.strip_prefix("--dir=") => {
                dir = Some(PathBuf::from(rest));
            }
            "--cache-dir" => {
                i += 1;
                let Some(v) = args.get(i) else {
                    return Err("missing value for --cache-dir".into());
                };
                cache_dir = Some(PathBuf::from(v));
            }
            t if let Some(rest) = t.strip_prefix("--cache-dir=") => {
                cache_dir = Some(PathBuf::from(rest));
            }
            other if other.starts_with('-') => {
                return Err(format!("unknown option: {other}"));
            }
            other => {
                if spec.is_some() {
                    return Err(format!("unexpected argument: {other}"));
                }
                spec = Some(other.to_string());
            }
        }
        i += 1;
    }
    let spec = spec.ok_or_else(|| "missing <module_path>@<ver>".to_string())?;
    Ok(GetArgs {
        spec,
        url,
        dir,
        cache_dir,
    })
}

fn print_usage() {
    println!(
        "\
draconic — the Draconic toolchain

Usage:
  draconic parse <file>                          Parse a Program and print the AST dump
  draconic extract <file>                        Print v1 JSON extract for one Program
  draconic check [--watch] <file>                Typecheck + bind a Program (no emit)
  draconic fmt [--check] <file>                  Format a Program in-place (or check only)
  draconic doc [--format md|html] [-o <out>] <file>
                                                 Extract /** doc comments */ to markdown or HTML
  draconic build --target js|native [--watch] [--strip] [--lto] [--link <lib.a>] <file> [-o <out>]
                                                  Compile a Program to JS or a native binary
  draconic run [--target js|native] <file> [args...]
                                                  Build and execute a Program (default target: js)
  draconic repl [--target js|embed]              Interactive read-eval-print (multi-line; last value)
  draconic test [--coverage] [--jobs <n>] <path> Run conformance fixtures (dir or .drac file)
  draconic get <module_path>@<ver> [--url <git-url>] [--dir <path>] [--cache-dir <path>]
                                                  Add/update a git package dep; fetch; write lock
  draconic mod tidy [--dir <path>] [--cache-dir <path>]
                                                   Align lock with manifest; fetch missing; prune unused
  draconic bindgen <header> [-o <out>]           Write Draconic extern \"C\" decls from a C header
  draconic version | -V | --version              Print verbose version (commit, host, LLVM)
  draconic help                                  Show this help

Shebang: #!/usr/bin/env draconic  (invokes run on the script path)
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
    fn parse_check_args_watch() {
        let args = vec!["--watch".into(), "a.drac".into()];
        let p = parse_check_args(&args).unwrap();
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
    fn parse_run_args_defaults_js_and_forwards() {
        let args = vec!["a.drac".into(), "x".into(), "y".into()];
        let p = parse_run_args(&args).unwrap();
        assert_eq!(p.target, Target::Js);
        assert_eq!(p.input, PathBuf::from("a.drac"));
        assert_eq!(p.program_args, vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn parse_run_args_native_target() {
        let args = vec!["--target".into(), "native".into(), "a.drac".into()];
        let p = parse_run_args(&args).unwrap();
        assert_eq!(p.target, Target::Native);
        assert_eq!(p.input, PathBuf::from("a.drac"));
        assert!(p.program_args.is_empty());
    }

    #[test]
    fn looks_like_script_path_for_shebang() {
        assert!(looks_like_script_path("hello.drac"));
        assert!(looks_like_script_path("./bin/tool"));
        assert!(!looks_like_script_path("--target"));
        assert!(!looks_like_script_path("build"));
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

    #[test]
    fn verbose_version_contains_required_fields() {
        let v = verbose_version();
        assert!(v.starts_with("draconic "), "{v}");
        assert!(v.contains("commit:"), "{v}");
        assert!(v.contains("host:"), "{v}");
        assert!(v.contains("LLVM:"), "{v}");
        let host = v
            .lines()
            .find(|l| l.starts_with("host:"))
            .expect("host line");
        assert!(
            host.contains('-') || host.contains("unknown"),
            "host should be a triple or unknown: {host}"
        );
    }

    #[test]
    fn parse_clang_llvm_version_prefers_llvm_line() {
        let text = "clang version 18.1.8\nTarget: x86_64-unknown-linux-gnu\nLLVM version 18.1.8\n";
        assert_eq!(parse_clang_llvm_version(text).as_deref(), Some("18.1.8"));
    }

    #[test]
    fn parse_clang_llvm_version_apple_banner() {
        let text =
            "Apple clang version 21.0.0 (clang-2100.1.1.101)\nTarget: arm64-apple-darwin25.5.0\n";
        assert_eq!(parse_clang_llvm_version(text).as_deref(), Some("21.0.0"));
    }
}
