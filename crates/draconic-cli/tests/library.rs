//! `toolchain.cli:build-js-library-esm`, `toolchain.cli:build-js-library-default`,
//! and `toolchain.cli:build-js-library-export-star`.

use std::fs;
use std::path::{Path, PathBuf};
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
        "draconic-cli-library-{}-{}-{}",
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

fn write_program(dir: &Path, name: &str, src: &str) -> PathBuf {
    let path = dir.join(name);
    fs::write(&path, src).unwrap();
    path
}

fn run(cmd: &mut Command) -> (i32, String, String) {
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

fn node_import_named(artifact: &Path, public: &str, expected: &str) -> (i32, String, String) {
    let url = format!("file://{}", artifact.display());
    let script = format!(
        "import {{ {public} }} from '{url}'; if ({public} !== {expected:?}) process.exit(1); console.log('ok');"
    );
    run(Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script))
}

fn node_import_default(artifact: &Path, expected: &str) -> (i32, String, String) {
    let url = format!("file://{}", artifact.display());
    let script = format!(
        "import v from '{url}'; if (v !== {expected:?}) process.exit(1); console.log('ok');"
    );
    run(Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script))
}

#[test]
fn help_lists_library() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("--library"),
        "help should list --library:\n{stdout}"
    );
}

#[test]
fn build_js_library_named_export_imports() {
    let dir = temp_dir();
    let src = write_program(&dir, "lib.drac", "export const view = \"view\";\n");
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let (ncode, nout, nerr) = node_import_named(&out, "view", "view");
    assert_eq!(
        ncode, 0,
        "Node import {{ view }} failed\nstdout={nout}\nstderr={nerr}"
    );
    assert!(nout.contains("ok"), "stdout={nout}");
}

#[test]
fn build_js_library_reexport_imports_public_name() {
    let dir = temp_dir();
    write_program(&dir, "dep.drac", "export const view = \"view\";\n");
    let src = write_program(&dir, "lib.drac", "export { view } from \"./dep.drac\";\n");
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library re-export build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let (ncode, nout, nerr) = node_import_named(&out, "view", "view");
    assert_eq!(
        ncode, 0,
        "Node import {{ view }} from re-export failed\nstdout={nout}\nstderr={nerr}"
    );
}

#[test]
fn build_js_library_alias_imports_public_name() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "lib.drac",
        "const local = \"view\";\nexport { local as publicName };\n",
    );
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library alias build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let (ncode, nout, nerr) = node_import_named(&out, "publicName", "view");
    assert_eq!(
        ncode, 0,
        "Node import {{ publicName }} failed\nstdout={nout}\nstderr={nerr}"
    );
}

#[test]
fn build_native_library_flag_is_js_only() {
    let dir = temp_dir();
    let src = write_program(&dir, "prog.drac", "let x: i32 = 1;");
    let out = dir.join("prog");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("native")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_ne!(
        code, 0,
        "native --library must fail\nstdout={stdout}\nstderr={stderr}"
    );
    let combined = format!("{stdout}{stderr}");
    assert!(
        combined.contains("library") && combined.contains("js"),
        "native --library must say the flag is js-only:\n{combined}"
    );
    assert!(
        !combined.contains("unknown option"),
        "native --library must be recognized, not unknown:\n{combined}"
    );
}

#[test]
fn build_js_without_library_stays_script() {
    let dir = temp_dir();
    let src = write_program(&dir, "lib.drac", "export const view = \"view\";\n");
    let out = dir.join("lib.js");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(code, 0, "default js build failed\nstdout={stdout}\nstderr={stderr}");

    let js = fs::read_to_string(&out).expect("js");
    assert!(
        !js.contains("export"),
        "default js emit must stay a script:\n{js}"
    );

    let (ncode, nout, nerr) = run(Command::new("node").arg(&out));
    assert_eq!(
        ncode, 0,
        "default artifact must run as a Node script\nstdout={nout}\nstderr={nerr}\njs={js}"
    );
}

#[test]
fn run_module_named_export_stays_script() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "p.drac",
        "export const view = \"view\";\nconsole.log(\"script-ok\");\n",
    );

    let (code, stdout, stderr) = run(draconic()
        .arg("run")
        .arg("--target")
        .arg("js")
        .arg(&src));
    assert_eq!(code, 0, "run failed\nstdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("script-ok"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn build_js_library_empty_exports_omits_export_braces() {
    let dir = temp_dir();
    let src = write_program(&dir, "lib.drac", "let x = 1;\n");
    let out = dir.join("lib.js");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "empty-export library build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let js = fs::read_to_string(&out).expect("js");
    assert!(
        !js.contains("export {}"),
        "empty named-export table must not force export {{}}:\n{js}"
    );

    let (ncode, nout, nerr) = run(Command::new("node").arg(&out));
    assert_eq!(
        ncode, 0,
        "empty-export library artifact must still run as a script\nstdout={nout}\nstderr={nerr}\njs={js}"
    );
}

#[test]
fn build_js_library_default_export_imports() {
    let dir = temp_dir();
    let src = write_program(&dir, "lib.drac", "export default \"def\";\n");
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library default build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let (ncode, nout, nerr) = node_import_default(&out, "def");
    assert_eq!(
        ncode, 0,
        "Node import default failed\nstdout={nout}\nstderr={nerr}"
    );
    assert!(nout.contains("ok"), "stdout={nout}");
}

#[test]
fn build_js_library_default_and_named_imports() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "lib.drac",
        "export default \"def\";\nexport const named = \"n\";\n",
    );
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library default+named build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let url = format!("file://{}", out.display());
    let script = format!(
        "import v, {{ named }} from '{url}'; if (v !== 'def' || named !== 'n') process.exit(1); console.log('ok');"
    );
    let (ncode, nout, nerr) = run(Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script));
    assert_eq!(
        ncode, 0,
        "Node import default + named failed\nstdout={nout}\nstderr={nerr}"
    );
}

#[test]
fn build_js_library_anonymous_default_function_imports() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "lib.drac",
        "export default function() { return \"def\"; }\n",
    );
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library anonymous default function build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let url = format!("file://{}", out.display());
    let script = format!(
        "import v from '{url}'; if (typeof v !== 'function' || v() !== 'def') process.exit(1); console.log('ok');"
    );
    let (ncode, nout, nerr) = run(Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script));
    assert_eq!(
        ncode, 0,
        "Node import anonymous default function failed\nstdout={nout}\nstderr={nerr}"
    );
}

#[test]
fn build_js_library_anonymous_default_class_imports() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "lib.drac",
        "export default class { constructor() { this.n = 1; } }\n",
    );
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library anonymous default class build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let url = format!("file://{}", out.display());
    let script = format!(
        "import v from '{url}'; if (new v().n !== 1) process.exit(1); console.log('ok');"
    );
    let (ncode, nout, nerr) = run(Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script));
    assert_eq!(
        ncode, 0,
        "Node import anonymous default class failed\nstdout={nout}\nstderr={nerr}"
    );
}

#[test]
fn build_js_library_alias_as_default_imports() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "lib.drac",
        "const local = \"def\";\nexport { local as default };\n",
    );
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library alias-as-default build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let (ncode, nout, nerr) = node_import_default(&out, "def");
    assert_eq!(
        ncode, 0,
        "Node import {{ local as default }} failed\nstdout={nout}\nstderr={nerr}"
    );
}

#[test]
fn run_module_default_export_stays_script() {
    let dir = temp_dir();
    let src = write_program(
        &dir,
        "p.drac",
        "export default \"def\";\nconsole.log(\"script-ok\");\n",
    );

    let (code, stdout, stderr) = run(draconic()
        .arg("run")
        .arg("--target")
        .arg("js")
        .arg(&src));
    assert_eq!(code, 0, "run failed\nstdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("script-ok"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn build_js_library_export_star_imports() {
    let dir = temp_dir();
    write_program(&dir, "dep.drac", "export const view = \"view\";\n");
    let src = write_program(&dir, "lib.drac", "export * from \"./dep.drac\";\n");
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library export-star build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let (ncode, nout, nerr) = node_import_named(&out, "view", "view");
    assert_eq!(
        ncode, 0,
        "Node import {{ view }} from export * failed\nstdout={nout}\nstderr={nerr}"
    );
    assert!(nout.contains("ok"), "stdout={nout}");
}

#[test]
fn build_js_library_export_star_omits_default() {
    let dir = temp_dir();
    write_program(
        &dir,
        "dep.drac",
        "export default \"def\";\nexport const view = \"view\";\n",
    );
    let src = write_program(&dir, "lib.drac", "export * from \"./dep.drac\";\n");
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "library export-star default-skip build failed\nstdout={stdout}\nstderr={stderr}"
    );

    let (ncode, nout, nerr) = node_import_named(&out, "view", "view");
    assert_eq!(
        ncode, 0,
        "Node import {{ view }} from export * failed\nstdout={nout}\nstderr={nerr}"
    );

    let url = format!("file://{}", out.display());
    let script = format!(
        "const m = await import('{url}'); if (m.default !== undefined) process.exit(1); console.log('ok');"
    );
    let (dcode, dout, derr) = run(Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script));
    assert_eq!(
        dcode, 0,
        "star must not re-export default\nstdout={dout}\nstderr={derr}"
    );
}

#[test]
fn build_js_library_export_star_omits_ambiguous() {
    let dir = temp_dir();
    write_program(
        &dir,
        "a.drac",
        "export const first = \"first\";\nexport const both = \"a\";\n",
    );
    write_program(
        &dir,
        "b.drac",
        "export const second = \"second\";\nexport const both = \"b\";\n",
    );
    let src = write_program(
        &dir,
        "lib.drac",
        "export * from \"./a.drac\";\nexport * from \"./b.drac\";\n",
    );
    let out = dir.join("lib.mjs");

    let (code, stdout, stderr) = run(draconic()
        .arg("build")
        .arg("--target")
        .arg("js")
        .arg("--library")
        .arg(&src)
        .arg("-o")
        .arg(&out));
    assert_eq!(
        code, 0,
        "ambiguous star library build must succeed\nstdout={stdout}\nstderr={stderr}"
    );

    let (ncode, nout, nerr) = node_import_named(&out, "first", "first");
    assert_eq!(
        ncode, 0,
        "Node import {{ first }} from ambiguous star failed\nstdout={nout}\nstderr={nerr}"
    );

    let url = format!("file://{}", out.display());
    let script = format!(
        "const m = await import('{url}'); if (m.both !== undefined) process.exit(1); if (m.second !== 'second') process.exit(1); console.log('ok');"
    );
    let (acode, aout, aerr) = run(Command::new("node")
        .arg("--input-type=module")
        .arg("-e")
        .arg(script));
    assert_eq!(
        acode, 0,
        "ambiguous star name both must be omitted\nstdout={aout}\nstderr={aerr}"
    );
}

#[test]
fn run_module_export_star_stays_script() {
    let dir = temp_dir();
    write_program(&dir, "dep.drac", "export const view = \"view\";\n");
    let src = write_program(
        &dir,
        "p.drac",
        "export * from \"./dep.drac\";\nconsole.log(\"script-ok\");\n",
    );

    let (code, stdout, stderr) = run(draconic()
        .arg("run")
        .arg("--target")
        .arg("js")
        .arg(&src));
    assert_eq!(code, 0, "run failed\nstdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("script-ok"),
        "stdout={stdout}\nstderr={stderr}"
    );
}
