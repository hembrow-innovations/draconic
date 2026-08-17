use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_ast::print_program;
use draconic_backend_js::emit_js;
use draconic_backend_llvm::{build_native_binary, emit_llvm_ir};
use draconic_conformance::{load_path, run_fixture};
use draconic_diagnostics::Diagnostic;
use draconic_frontend::{check_path, compile_path};
use draconic_parser::{parse, parse_module};

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
        "build" => cmd_build(&args),
        "run" => cmd_run(&args),
        "test" => cmd_test(&args),
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
    let path = match args.first() {
        Some(p) if p != "-h" && p != "--help" => PathBuf::from(p),
        _ => {
            eprintln!("usage: draconic check <file>");
            return ExitCode::from(2);
        }
    };

    if args.len() > 1 {
        eprintln!("usage: draconic check <file>");
        return ExitCode::from(2);
    }

    match check_path(&path) {
        Ok(_) => ExitCode::SUCCESS,
        Err(d) => {
            eprintln!("error: {d}");
            ExitCode::from(1)
        }
    }
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

    let out = match &parsed.output {
        Some(p) => p.clone(),
        None => default_output(&parsed.input, parsed.target),
    };

    if let Err(d) = build_program(&parsed.input, parsed.target, &out) {
        eprintln!("error: {d}");
        return ExitCode::from(1);
    }

    ExitCode::SUCCESS
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

    if let Err(d) = build_program(&parsed.input, parsed.target, &artifact) {
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
    let status = cmd
        .status()
        .map_err(|e| match target {
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

fn build_program(input: &Path, target: Target, out: &Path) -> Result<(), Diagnostic> {
    let module = compile_path(input)?;

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

fn print_usage() {
    println!(
        "\
draconic — the Draconic toolchain

Usage:
  draconic parse <file>                          Parse a Program and print the AST dump
  draconic check <file>                          Typecheck + bind a Program (no emit)
  draconic fmt [--check] <file>                  Format a Program in-place (or check only)
  draconic build --target js|native <file> [-o <out>]
                                                 Compile a Program to JS or a native binary
  draconic run [--target js|native] <file> [args...]
                                                 Build and execute a Program (default target: js)
  draconic test <path>                           Run conformance fixtures (dir or .drac file)
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
    }

    #[test]
    fn parse_build_args_requires_target() {
        let args = vec!["a.drac".into()];
        let err = parse_build_args(&args).unwrap_err();
        assert!(err.contains("target"), "{err}");
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
        let args = vec![
            "--target".into(),
            "native".into(),
            "a.drac".into(),
        ];
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
        let dir = std::env::temp_dir().join(format!(
            "draconic-cli-unit-js-{}",
            std::process::id()
        ));
        fs::create_dir_all(&dir).unwrap();
        let out = dir.join("t.js");
        let input = dir.join("t.drac");
        fs::write(&input, "let x = 1;").unwrap();
        build_program(&input, Target::Js, &out).unwrap();
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
        assert_eq!(
            parse_clang_llvm_version(text).as_deref(),
            Some("18.1.8")
        );
    }

    #[test]
    fn parse_clang_llvm_version_apple_banner() {
        let text =
            "Apple clang version 21.0.0 (clang-2100.1.1.101)\nTarget: arm64-apple-darwin25.5.0\n";
        assert_eq!(
            parse_clang_llvm_version(text).as_deref(),
            Some("21.0.0")
        );
    }
}
