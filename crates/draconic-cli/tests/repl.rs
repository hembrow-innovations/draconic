//! ROADMAP U08: `draconic repl` — read-eval-print; multi-line; last-value print.

use std::io::Write;
use std::process::{Command, Stdio};

fn draconic() -> Command {
    Command::new(env!("CARGO_BIN_EXE_draconic"))
}

fn run_repl(stdin_text: &str, extra_args: &[&str]) -> (i32, String, String) {
    let mut cmd = draconic();
    cmd.arg("repl");
    for a in extra_args {
        cmd.arg(a);
    }
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn draconic repl");
    {
        let mut stdin = child.stdin.take().expect("stdin");
        stdin
            .write_all(stdin_text.as_bytes())
            .expect("write stdin");
    }
    let output = child.wait_with_output().expect("wait repl");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn help_lists_repl() {
    let output = draconic()
        .arg("help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("help");
    assert_eq!(output.status.code(), Some(0));
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("repl"),
        "help should list repl:\n{stdout}"
    );
}

#[test]
fn repl_prints_last_expression_value() {
    let (code, stdout, stderr) = run_repl("1 + 2\n", &[]);
    assert_eq!(code, 0, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.lines().any(|l| l.trim() == "3"),
        "expected last value 3 in stdout:\n{stdout}\nstderr={stderr}"
    );
}

#[test]
fn repl_multiline_function_then_call() {
    let input = "function f() {\n  return 40 + 2;\n}\nf()\n";
    let (code, stdout, stderr) = run_repl(input, &[]);
    assert_eq!(code, 0, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.lines().any(|l| l.trim() == "42"),
        "expected 42 from multi-line input:\n{stdout}\nstderr={stderr}"
    );
}

#[test]
fn repl_session_binding_carries_forward() {
    let input = "let x = 10;\nx + 1\n";
    let (code, stdout, stderr) = run_repl(input, &[]);
    assert_eq!(code, 0, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.lines().any(|l| l.trim() == "11"),
        "expected session binding x → 11:\n{stdout}\nstderr={stderr}"
    );
}

#[test]
fn repl_embed_target_prints_expression() {
    let (code, stdout, stderr) = run_repl("2 * 3\n", &["--target", "embed"]);
    assert_eq!(code, 0, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stdout.lines().any(|l| l.trim() == "6"),
        "embed target should print 6:\n{stdout}\nstderr={stderr}"
    );
}

#[test]
fn repl_syntax_error_continues() {
    let input = "@@not-valid\n1 + 1\n";
    let (code, stdout, stderr) = run_repl(input, &[]);
    assert_eq!(code, 0, "stderr={stderr}\nstdout={stdout}");
    assert!(
        stderr.contains("error") || stderr.to_ascii_lowercase().contains("error"),
        "syntax error should be reported on stderr:\n{stderr}"
    );
    assert!(
        stdout.lines().any(|l| l.trim() == "2"),
        "repl should recover and print 2:\n{stdout}\nstderr={stderr}"
    );
}
