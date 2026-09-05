//! Integration: minimal Program → IR → JS → run with Node (ROADMAP B07).

use std::process::{Command, Stdio};

use draconic_backend_js::emit_js;
use draconic_frontend::compile_source;

fn compile_to_js(src: &str) -> String {
    let module = compile_source(src).expect("compile");
    emit_js(&module).expect("emit_js")
}

fn run_node(script: &str) -> (i32, String, String) {
    let child = Command::new("node")
        .arg("-e")
        .arg(script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("node must be available to run JS backend integration tests");

    let output = child.wait_with_output().expect("wait node");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn assert_node_ok(script: &str) {
    let (code, stdout, stderr) = run_node(script);
    assert_eq!(
        code, 0,
        "node failed\n--- script ---\n{script}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    );
}

#[test]
fn minimal_let_add_runs() {
    let js = compile_to_js("let x = 1 + 2;");
    let script =
        format!("{js}\nif (x !== 3) {{ console.error('want 3 got', x); process.exit(1); }}");
    assert_node_ok(&script);
}

#[test]
fn string_concat_runs() {
    let js = compile_to_js(r#"let s = "a" + "b";"#);
    let script = format!("{js}\nif (s !== 'ab') {{ console.error(s); process.exit(1); }}");
    assert_node_ok(&script);
}

#[test]
fn unary_and_bool_runs() {
    let js = compile_to_js("let a = -1; let b = !false; let c = null; let d = true;");
    let script = format!(
        "{js}\n\
         if (a !== -1) process.exit(1);\n\
         if (b !== true) process.exit(1);\n\
         if (c !== null) process.exit(1);\n\
         if (d !== true) process.exit(1);\n"
    );
    assert_node_ok(&script);
}

#[test]
fn comparison_logic_runs() {
    let js = compile_to_js("let ok = 1 < 2 && true;");
    let script = format!("{js}\nif (ok !== true) process.exit(1);");
    assert_node_ok(&script);
}

#[test]
fn binding_use_runs() {
    let js = compile_to_js("let x = 1; let y = x + 2;");
    let script = format!("{js}\nif (y !== 3) process.exit(1);");
    assert_node_ok(&script);
}

#[test]
fn emitted_js_is_parseable_empty() {
    let js = compile_to_js("");
    assert_eq!(js, "");
    assert_node_ok("");
}

#[test]
fn node_available() {
    let status = Command::new("node")
        .arg("-e")
        .arg("process.exit(0)")
        .status()
        .expect("spawn node");
    assert!(status.success());
}
