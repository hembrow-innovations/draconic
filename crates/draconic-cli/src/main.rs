use std::env;
use std::path::Path;
use std::process::ExitCode;

mod cmd;
mod doc;
mod extract;
mod strip_symbols;
mod toolchain_pin;
mod watch;
mod work_dir;

fn main() -> ExitCode {
    let mut args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() {
        eprint_usage();
        return ExitCode::from(2);
    }

    let cmd = args.remove(0);
    match cmd.as_str() {
        "parse" => cmd::cmd_parse::cmd_parse(&args),
        "check" => cmd::cmd_check::cmd_check(&args),
        "fmt" => cmd::cmd_fmt::cmd_fmt(&args),
        "doc" => cmd::cmd_doc::cmd_doc(&args),
        "extract" => extract::cmd_extract(&args),
        "build" => cmd::cmd_build::cmd_build(&args),
        "run" => cmd::cmd_run::cmd_run(&args),
        "repl" => cmd::cmd_repl::cmd_repl(&args),
        "test" => cmd::cmd_test::cmd_test(&args),
        "get" => cmd::cmd_get::cmd_get(&args),
        "mod" => cmd::cmd_mod::cmd_mod(&args),
        "bindgen" => cmd::cmd_bindgen::cmd_bindgen(&args),
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
            cmd::cmd_run::cmd_run(&run_args)
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
  draconic extract <file>                        Print v1 JSON extract for one Program
  draconic check [--watch] <file>                Typecheck + bind a Program (no emit)
  draconic fmt [--check] <file>                  Format a Program in-place (or check only)
  draconic doc [--format md|html] [-o <out>] <file>
                                                 Extract /** doc comments */ to markdown or HTML
  draconic build --target js|native [--watch] [--strip] [--lto] [--link <lib.a>] <file> [-o <out>]
                                                  Compile a Program to JS or a native binary
  draconic run [--target js|native] [--allow-fs-read] [--allow-fs-write]
               [--allow-net-listen] [--allow-net-connect] <file> [args...]
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

    #[test]
    fn looks_like_script_path_for_shebang() {
        assert!(looks_like_script_path("hello.drac"));
        assert!(looks_like_script_path("./bin/tool"));
        assert!(!looks_like_script_path("--target"));
        assert!(!looks_like_script_path("build"));
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
