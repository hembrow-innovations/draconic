//! ROADMAP F07 / F07.01–F07.04: bindgen-ish C header subset → Draconic extern module.
//! F07 parent locks parse + emit + CLI write for scalar/pointer fns, structs, and typedefs.

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_cli::c_header::{emit_externs, parse_header, CType, Param};

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-integration-f07-01-{}-{}-{}",
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

#[test]
fn parse_header_file_scalar_and_pointer_fns() {
    let dir = temp_dir();
    let path = dir.join("api.h");
    fs::write(
        &path,
        r#"
            int add(int a, int b);
            void free(void *p);
            char *strdup(const char *s);
        "#,
    )
    .unwrap();
    let src = fs::read_to_string(&path).unwrap();
    let h = parse_header(&src).expect("parse_header");
    assert_eq!(h.functions.len(), 3);
    assert_eq!(h.functions[0].name, "add");
    assert_eq!(h.functions[0].return_ty, CType::Int);
    assert_eq!(
        h.functions[1].params,
        vec![Param {
            name: Some("p".into()),
            ty: CType::Pointer(Box::new(CType::Void)),
        }]
    );
    assert_eq!(
        h.functions[2].return_ty,
        CType::Pointer(Box::new(CType::Char))
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn emit_externs_from_header_file_is_parseable_draconic() {
    let dir = temp_dir();
    let path = dir.join("api.h");
    fs::write(
        &path,
        r#"
            int add(int a, int b);
            void free(void *p);
            char *strdup(const char *s);
            int getpid(void);
            int abs(int);
        "#,
    )
    .unwrap();
    let src = fs::read_to_string(&path).unwrap();
    let h = parse_header(&src).expect("parse_header");
    let emitted = emit_externs(&h);
    assert_eq!(
        emitted,
        concat!(
            "extern \"C\" function add(a: i32, b: i32): i32;\n",
            "extern \"C\" function free(p: *u8): void;\n",
            "extern \"C\" function strdup(s: *u8): *u8;\n",
            "extern \"C\" function getpid(): i32;\n",
            "extern \"C\" function abs(p0: i32): i32;\n",
        )
    );
    draconic_parser::parse(&emitted).expect("emitted Draconic must parse");
    let _ = fs::remove_dir_all(&dir);
}

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .expect("repo root")
}

fn draconic_bin() -> PathBuf {
    let profile = if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    };
    let bin = repo_root().join("target").join(profile).join("draconic");
    assert!(
        bin.is_file(),
        "missing {} (build draconic-cli first)",
        bin.display()
    );
    bin
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

const HEADER_SRC: &str = r#"
    int add(int a, int b);
    void free(void *p);
    char *strdup(const char *s);
    int getpid(void);
    int abs(int);
"#;

const EXPECTED_MODULE: &str = concat!(
    "extern \"C\" function add(a: i32, b: i32): i32;\n",
    "extern \"C\" function free(p: *u8): void;\n",
    "extern \"C\" function strdup(s: *u8): *u8;\n",
    "extern \"C\" function getpid(): i32;\n",
    "extern \"C\" function abs(p0: i32): i32;\n",
);

#[test]
fn bindgen_cli_writes_sibling_drac_module() {
    let dir = temp_dir();
    let header = dir.join("api.h");
    fs::write(&header, HEADER_SRC).unwrap();
    let (code, stdout, stderr) = run(Command::new(draconic_bin()).arg("bindgen").arg(&header));
    assert_eq!(
        code, 0,
        "bindgen failed\nstdout={stdout}\nstderr={stderr}"
    );
    let out = dir.join("api.drac");
    let got = fs::read_to_string(&out).expect("wrote api.drac");
    assert_eq!(got, EXPECTED_MODULE);
    draconic_parser::parse(&got).expect("written module must parse");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bindgen_cli_dash_o_writes_named_module() {
    let dir = temp_dir();
    let header = dir.join("api.h");
    let dest = dir.join("externs.drac");
    fs::write(&header, HEADER_SRC).unwrap();
    let (code, stdout, stderr) = run(
        Command::new(draconic_bin())
            .arg("bindgen")
            .arg(&header)
            .arg("-o")
            .arg(&dest),
    );
    assert_eq!(
        code, 0,
        "bindgen -o failed\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(!dir.join("api.drac").exists(), "default sibling must not be written when -o is set");
    let got = fs::read_to_string(&dest).expect("wrote -o path");
    assert_eq!(got, EXPECTED_MODULE);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bindgen_cli_missing_header_exits_usage() {
    let (code, stdout, stderr) = run(Command::new(draconic_bin()).arg("bindgen"));
    assert_eq!(code, 2, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stderr.contains("usage: draconic bindgen"),
        "stderr={stderr}"
    );
}

const STRUCT_HEADER_SRC: &str = r#"
    struct Point { int x; int y; };
    typedef int Int;
    typedef struct { int a; int b; } Pair;
    int take(struct Point p);
    Int ident(Int n);
    Pair *make_pair(Int a, Int b);
"#;

const STRUCT_EXPECTED_MODULE: &str = concat!(
    "type Point = { x: i32; y: i32 };\n",
    "type Int = i32;\n",
    "type Pair = { a: i32; b: i32 };\n",
    "extern \"C\" function take(p: Point): i32;\n",
    "extern \"C\" function ident(n: Int): Int;\n",
    "extern \"C\" function make_pair(a: Int, b: Int): *Pair;\n",
);

#[test]
fn emit_externs_structs_and_typedefs_is_parseable_draconic() {
    let dir = temp_dir();
    let path = dir.join("api.h");
    fs::write(&path, STRUCT_HEADER_SRC).unwrap();
    let src = fs::read_to_string(&path).unwrap();
    let h = parse_header(&src).expect("parse_header");
    let emitted = emit_externs(&h);
    assert_eq!(emitted, STRUCT_EXPECTED_MODULE);
    draconic_parser::parse(&emitted).expect("emitted Draconic must parse");
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn bindgen_cli_writes_struct_typedef_module() {
    let dir = temp_dir();
    let header = dir.join("api.h");
    fs::write(&header, STRUCT_HEADER_SRC).unwrap();
    let (code, stdout, stderr) = run(Command::new(draconic_bin()).arg("bindgen").arg(&header));
    assert_eq!(
        code, 0,
        "bindgen failed\nstdout={stdout}\nstderr={stderr}"
    );
    let got = fs::read_to_string(dir.join("api.drac")).expect("wrote api.drac");
    assert_eq!(got, STRUCT_EXPECTED_MODULE);
    draconic_parser::parse(&got).expect("written module must parse");
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
fn bindgen_cli_writes_combined_header_surface() {
    let dir = temp_dir();
    let header = dir.join("api.h");
    fs::write(&header, SURFACE_HEADER_SRC).unwrap();
    let src = fs::read_to_string(&header).unwrap();
    let h = parse_header(&src).expect("parse_header");
    let emitted = emit_externs(&h);
    assert_eq!(emitted, SURFACE_EXPECTED_MODULE);
    draconic_parser::parse(&emitted).expect("emitted Draconic must parse");

    let (code, stdout, stderr) = run(Command::new(draconic_bin()).arg("bindgen").arg(&header));
    assert_eq!(
        code, 0,
        "bindgen failed\nstdout={stdout}\nstderr={stderr}"
    );
    let got = fs::read_to_string(dir.join("api.drac")).expect("wrote api.drac");
    assert_eq!(got, SURFACE_EXPECTED_MODULE);
    draconic_parser::parse(&got).expect("written module must parse");
    let _ = fs::remove_dir_all(&dir);
}
