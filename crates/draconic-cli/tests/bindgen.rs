//! ROADMAP F07 / F07.01–F07.04: `draconic bindgen <header>` writes an extern module.
//! F07 parent locks parse + emit + CLI write for scalar/pointer fns, structs, and typedefs.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn draconic() -> Command {
    Command::new(env!("CARGO_BIN_EXE_draconic"))
}

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-cli-bindgen-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn run(cmd: &mut Command) -> (i32, String, String) {
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn draconic");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

const HEADER_SRC: &str = "int add(int a, int b);\nvoid free(void *p);\n";
const EXPECTED_MODULE: &str = concat!(
    "extern \"C\" function add(a: i32, b: i32): i32;\n",
    "extern \"C\" function free(p: *u8): void;\n",
);

#[test]
fn help_lists_bindgen_command() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("draconic bindgen"),
        "help should list bindgen:\n{stdout}"
    );
}

#[test]
fn bindgen_missing_header_exits_usage() {
    let (code, _stdout, stderr) = run(draconic().arg("bindgen"));
    assert_eq!(code, 2, "stderr={stderr}");
    assert!(
        stderr.contains("usage: draconic bindgen"),
        "stderr={stderr}"
    );
}

#[test]
fn bindgen_writes_sibling_drac() {
    let dir = temp_dir();
    let header = dir.join("api.h");
    fs::write(&header, HEADER_SRC).unwrap();
    let (code, stdout, stderr) = run(draconic().arg("bindgen").arg(&header));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    let got = fs::read_to_string(dir.join("api.drac")).expect("wrote api.drac");
    assert_eq!(got, EXPECTED_MODULE);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bindgen_dash_o_writes_named_module() {
    let dir = temp_dir();
    let header = dir.join("api.h");
    let dest = dir.join("externs.drac");
    fs::write(&header, HEADER_SRC).unwrap();
    let (code, stdout, stderr) = run(draconic().arg("bindgen").arg(&header).arg("-o").arg(&dest));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(!dir.join("api.drac").exists());
    let got = fs::read_to_string(&dest).expect("wrote -o path");
    assert_eq!(got, EXPECTED_MODULE);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bindgen_parse_error_exits_one() {
    let dir = temp_dir();
    let header = dir.join("bad.h");
    fs::write(&header, "union U { int x; };\n").unwrap();
    let (code, _stdout, stderr) = run(draconic().arg("bindgen").arg(&header));
    assert_eq!(code, 1, "stderr={stderr}");
    assert!(stderr.contains("bindgen:"), "stderr={stderr}");
    let _ = fs::remove_dir_all(&dir);
}

const SURFACE_HEADER_SRC: &str = concat!(
    "struct Point { int x; int y; };\n",
    "typedef int Int;\n",
    "typedef struct { int a; int b; } Pair;\n",
    "int add(int a, int b);\n",
    "void free(void *p);\n",
    "char *strdup(const char *s);\n",
    "Int ident(Int n);\n",
    "int take(struct Point p);\n",
    "Pair *make_pair(Int a, Int b);\n",
);

const SURFACE_EXPECTED_MODULE: &str = concat!(
    "type Point = { x: i32; y: i32 };\n",
    "type Int = i32;\n",
    "type Pair = { a: i32; b: i32 };\n",
    "extern \"C\" function add(a: i32, b: i32): i32;\n",
    "extern \"C\" function free(p: *u8): void;\n",
    "extern \"C\" function strdup(s: *u8): *u8;\n",
    "extern \"C\" function ident(n: Int): Int;\n",
    "extern \"C\" function take(p: Point): i32;\n",
    "extern \"C\" function make_pair(a: Int, b: Int): *Pair;\n",
);

#[test]
fn bindgen_combined_header_surface() {
    let dir = temp_dir();
    let header = dir.join("api.h");
    fs::write(&header, SURFACE_HEADER_SRC).unwrap();
    let (code, stdout, stderr) = run(draconic().arg("bindgen").arg(&header));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    let got = fs::read_to_string(dir.join("api.drac")).expect("wrote api.drac");
    assert_eq!(got, SURFACE_EXPECTED_MODULE);
    let _ = fs::remove_dir_all(&dir);
}
