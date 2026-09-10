use std::env;
use std::fs;
use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::process::{Command, ExitCode, Stdio};

use draconic_backend_js::emit_js_repl;
use draconic_diagnostics::Diagnostic;
use draconic_embed::{eval_source, EmbedValue};
use draconic_frontend::{compile_source, parse_source};

use crate::work_dir::run_work_dir;

/// ROADMAP U08: interactive read-eval-print (js default; optional embed).
/// Multi-line when parse fails with Eof; prints last expression value.
pub fn cmd_repl(args: &[String]) -> ExitCode {
    let target = match parse_repl_args(args) {
        Ok(t) => t,
        Err(msg) => {
            eprintln!("{msg}");
            eprintln!("usage: draconic repl [--target js|embed]");
            return ExitCode::from(2);
        }
    };

    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    if let Err(code) = crate::toolchain_pin::enforce(&cwd) {
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
    match parse_source(source) {
        Ok(_) => ReplBufferStatus::Complete,
        Err(d) => {
            let msg = d.to_string();
            if msg.contains("Eof") || msg.contains("end of file") || msg.contains("end of input") {
                ReplBufferStatus::Incomplete
            } else {
                ReplBufferStatus::Error(d)
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
