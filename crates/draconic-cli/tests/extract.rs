//! v1 JSON extract: `draconic extract <file>` prints one object on stdout.

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
        "draconic-cli-extract-{}-{}-{}",
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
        .expect("spawn draconic");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

/// Inner text of a top-level JSON array field (`"field":[...]`).
fn json_array_body(haystack: &str, field: &str) -> Option<String> {
    let needle = format!("\"{field}\":");
    let i = haystack.find(&needle)?;
    let rest = haystack[i + needle.len()..].trim_start();
    let rest = rest.strip_prefix('[')?;
    let end = rest.find(']')?;
    Some(rest[..end].to_string())
}

/// Collect JSON string values for `"field": "..."` (compact or spaced).
fn json_string_fields(haystack: &str, field: &str) -> Vec<String> {
    let needle = format!("\"{field}\":");
    let mut out = Vec::new();
    let mut rest = haystack;
    while let Some(i) = rest.find(&needle) {
        rest = rest[i + needle.len()..].trim_start();
        let Some(stripped) = rest.strip_prefix('"') else {
            break;
        };
        let Some(end) = stripped.find('"') else {
            break;
        };
        out.push(stripped[..end].to_string());
        rest = &stripped[end + 1..];
    }
    out
}

fn extract_stdout(src: &str) -> String {
    let dir = temp_dir();
    let path = write_program(&dir, "hash.drac", src);
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with("{\"version\":1,"),
        "draconic extract JSON version must stay integer 1: {stdout}",
    );
    stdout
}

#[test]
fn help_lists_extract_command() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("draconic extract") || stdout.contains("extract "),
        "help should list extract:\n{stdout}"
    );
}

#[test]
fn extract_missing_path_exits_usage() {
    let (code, _stdout, stderr) = run(draconic().arg("extract"));
    assert_eq!(code, 2, "stderr={stderr}");
    assert!(
        stderr.contains("usage") || stderr.contains("extract"),
        "stderr={stderr}"
    );
}

#[test]
fn extract_emits_import_specifier_for_relative_drac() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "check.drac",
        "import { hashPassword } from \"./hash.drac\";\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with('{'),
        "draconic extract stdout is not JSON: {stdout}"
    );
    assert!(
        stdout.contains("\"imports\""),
        "draconic extract missing imports key: {stdout}"
    );
    let specs = json_string_fields(&stdout, "name");
    assert!(
        !specs.is_empty(),
        "draconic extract emitted empty imports: {stdout}"
    );
    assert!(
        specs.iter().any(|name| name == "./hash.drac"),
        "draconic extract missed ./hash.drac in imports: {stdout}"
    );
}

#[test]
fn extract_emits_named_function() {
    let dir = temp_dir();
    let path = write_program(&dir, "hash.drac", "function hashPassword() {}\n");
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with('{'),
        "draconic extract stdout is not JSON: {stdout}"
    );
    let names = json_string_fields(&stdout, "name");
    assert!(
        names.iter().any(|name| name == "hashPassword"),
        "draconic extract missed hashPassword: {stdout}"
    );
}

#[test]
fn extract_emits_identifier_call_from_enclosing_function() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function encodePassword() {}\nfunction hashPassword() {\n  encodePassword();\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with('{'),
        "draconic extract stdout is not JSON: {stdout}"
    );
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        !calls.trim().is_empty(),
        "draconic extract emitted empty calls: {stdout}"
    );
    assert!(
        calls.contains("\"name\":\"encodePassword\""),
        "draconic extract missed encodePassword in calls: {stdout}"
    );
    assert!(
        calls.contains("\"enclosing\":\"hashPassword\""),
        "draconic extract missed enclosing hashPassword on calls: {stdout}"
    );
}

#[test]
fn extract_emits_extern_function_abi_from_ast() {
    let dir = temp_dir();
    let path = write_program(&dir, "hash.drac", "extern \"C\" function nativeHash();\n");
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let externs = json_array_body(&stdout, "externFunctions").unwrap_or_default();
    assert!(
        !externs.trim().is_empty(),
        "draconic extract emitted empty externFunctions: {stdout}"
    );
    assert!(
        externs.contains("\"name\":\"nativeHash\""),
        "draconic extract missed nativeHash in externFunctions: {stdout}"
    );
    assert!(
        externs.contains("\"abi\":\"C\""),
        "draconic extract missed abi from extern AST: {stdout}"
    );
}

#[test]
fn extract_emits_native_true_for_named_native_scalar_type_alias() {
    let dir = temp_dir();
    let path = write_program(&dir, "hash.drac", "type HashBuf = i32;\n");
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let aliases = json_array_body(&stdout, "typeAliases").unwrap_or_default();
    assert!(
        aliases.contains("\"name\":\"HashBuf\""),
        "draconic extract missed HashBuf in typeAliases: {stdout}"
    );
    assert!(
        aliases.contains("\"native\":true"),
        "draconic extract missed native: true on HashBuf: {stdout}"
    );
}

#[test]
fn extract_emits_native_true_for_pointer_type_alias() {
    let dir = temp_dir();
    let path = write_program(&dir, "hash.drac", "type HashPtr = *i32;\n");
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let aliases = json_array_body(&stdout, "typeAliases").unwrap_or_default();
    assert!(
        aliases.contains("\"name\":\"HashPtr\""),
        "draconic extract missed HashPtr in typeAliases: {stdout}"
    );
    assert!(
        aliases.contains("\"native\":true"),
        "draconic extract missed native: true on pointer type alias: {stdout}"
    );
}

#[test]
fn extract_omits_native_for_ordinary_type_alias() {
    let dir = temp_dir();
    let path = write_program(&dir, "hash.drac", "type Password = string;\n");
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let aliases = json_array_body(&stdout, "typeAliases").unwrap_or_default();
    assert!(
        aliases.contains("\"name\":\"Password\""),
        "draconic extract missed Password in typeAliases: {stdout}"
    );
    assert!(
        !aliases.contains("\"native\":true"),
        "draconic extract should omit native on ordinary type alias: {stdout}"
    );
}

#[test]
fn extract_emits_native_true_for_each_named_native_scalar() {
    const SCALARS: &[&str] = &[
        "i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "f32", "f64",
    ];
    let dir = temp_dir();
    let mut src = String::new();
    for (i, scalar) in SCALARS.iter().enumerate() {
        src.push_str(&format!("type N{i} = {scalar};\n"));
    }
    let path = write_program(&dir, "hash.drac", &src);
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let aliases = json_array_body(&stdout, "typeAliases").unwrap_or_default();
    for (i, scalar) in SCALARS.iter().enumerate() {
        let name = format!("\"name\":\"N{i}\"");
        assert!(
            aliases.contains(&name),
            "draconic extract missed N{i} ({scalar}) in typeAliases: {stdout}"
        );
    }
    let native_count = aliases.matches("\"native\":true").count();
    assert_eq!(
        native_count,
        SCALARS.len(),
        "draconic extract missed native: true on a named native scalar: {stdout}"
    );
}

#[test]
fn extract_omits_native_for_nested_struct_and_tuple_type_aliases() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "type Point = { x: i32 };\ntype Pair = [i32, i32];\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let aliases = json_array_body(&stdout, "typeAliases").unwrap_or_default();
    assert!(
        aliases.contains("\"name\":\"Point\""),
        "draconic extract missed Point in typeAliases: {stdout}"
    );
    assert!(
        aliases.contains("\"name\":\"Pair\""),
        "draconic extract missed Pair in typeAliases: {stdout}"
    );
    assert!(
        !aliases.contains("\"native\":true"),
        "draconic extract should omit native on nested struct and tuple aliases: {stdout}"
    );
}

#[test]
fn extract_emits_static_block_nested_class_as_outer_inner() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class PasswordHasher {\n  static {\n    class Salt {}\n  }\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let classes = json_array_body(&stdout, "classes").unwrap_or_default();
    assert!(
        classes.contains("\"name\":\"PasswordHasher\""),
        "draconic extract missed PasswordHasher in classes: {stdout}"
    );
    assert!(
        classes.contains("\"name\":\"PasswordHasher.Salt\""),
        "draconic extract missed PasswordHasher.Salt in classes: {stdout}"
    );
    assert!(
        !classes.contains("\"name\":\"Salt\""),
        "draconic extract should not emit bare Salt for static-block nested class: {stdout}"
    );
}

#[test]
fn extract_emits_nested_function_as_bare_name() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function outer() {\n  function inner() {}\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    assert!(
        functions.contains("\"name\":\"outer\""),
        "draconic extract missed outer in functions: {stdout}"
    );
    assert!(
        functions.contains("\"name\":\"inner\""),
        "draconic extract missed nested inner in functions: {stdout}"
    );
    assert!(
        !functions.contains("outer.inner"),
        "draconic extract should emit nested function as a bare name: {stdout}"
    );
}

#[test]
fn extract_emits_class_in_function_and_method_as_bare_name() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function outer() {\n  class InFn {}\n}\nclass Foo {\n  method() {\n    class Nested {}\n  }\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let classes = json_array_body(&stdout, "classes").unwrap_or_default();
    assert!(
        classes.contains("\"name\":\"InFn\""),
        "draconic extract missed class-in-function InFn: {stdout}"
    );
    assert!(
        classes.contains("\"name\":\"Nested\""),
        "draconic extract missed class-in-method Nested: {stdout}"
    );
    assert!(
        !classes.contains("outer.InFn") && !classes.contains("Foo.Nested"),
        "draconic extract should emit class-in-function and class-in-method as bare names: {stdout}"
    );
}

#[test]
fn extract_emits_nested_in_nested_class_chain() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class Outer {\n  static {\n    class Mid {\n      static {\n        class Inner {}\n      }\n    }\n  }\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let classes = json_array_body(&stdout, "classes").unwrap_or_default();
    assert!(
        classes.contains("\"name\":\"Outer\""),
        "draconic extract missed Outer in classes: {stdout}"
    );
    assert!(
        classes.contains("\"name\":\"Outer.Mid\""),
        "draconic extract missed Outer.Mid in classes: {stdout}"
    );
    assert!(
        classes.contains("\"name\":\"Outer.Mid.Inner\""),
        "draconic extract missed Outer.Mid.Inner in classes: {stdout}"
    );
}

#[test]
fn extract_walks_nested_decls_in_constructor_method_accessor_static_block() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class Outer {\n  constructor() {\n    class InCtor {}\n    function fromCtor() {}\n  }\n  method() {\n    class InMethod {}\n    function fromMethod() {}\n  }\n  get value() {\n    class InGetter {}\n  }\n  set value(v) {\n    function fromSetter() {}\n  }\n  static {\n    function fromStatic() {}\n  }\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let classes = json_array_body(&stdout, "classes").unwrap_or_default();
    for name in ["fromCtor", "fromMethod", "fromSetter", "fromStatic"] {
        let needle = format!("\"name\":\"{name}\"");
        assert!(
            functions.contains(&needle),
            "draconic extract missed {name} in functions: {stdout}"
        );
    }
    for name in ["InCtor", "InMethod", "InGetter"] {
        let needle = format!("\"name\":\"{name}\"");
        assert!(
            classes.contains(&needle),
            "draconic extract missed {name} in classes: {stdout}"
        );
    }
}

#[test]
fn extract_omits_methods_constructors_accessors_fields_and_class_expressions() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class Outer {\n  constructor() {}\n  ping() {}\n  get value() { return 1; }\n  x = 1;\n  static {\n    class Inner {}\n  }\n}\nlet C = class Expr {};\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let classes = json_array_body(&stdout, "classes").unwrap_or_default();
    assert!(
        functions.trim().is_empty(),
        "draconic extract should not emit methods as functions: {stdout}"
    );
    assert!(
        classes.contains("\"name\":\"Outer\""),
        "draconic extract missed Outer in classes: {stdout}"
    );
    assert!(
        classes.contains("\"name\":\"Outer.Inner\""),
        "draconic extract missed Outer.Inner in classes: {stdout}"
    );
    assert!(
        !classes.contains("constructor")
            && !classes.contains("ping")
            && !classes.contains("value")
            && !classes.contains("\"name\":\"x\""),
        "draconic extract should not emit methods, constructors, accessors, or fields: {stdout}"
    );
    assert!(
        !classes.contains("Expr"),
        "draconic extract should not emit class expressions: {stdout}"
    );
}

#[test]
fn extract_emits_nested_class_method_as_type_method() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class PasswordHasher {\n  static {\n    class Salt {\n      encode() {}\n    }\n  }\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    assert!(
        methods.contains("\"name\":\"PasswordHasher.Salt.encode\""),
        "draconic extract missed PasswordHasher.Salt.encode in methods: {stdout}",
    );
    assert!(
        !functions.contains("encode"),
        "draconic extract should not emit methods as functions: {stdout}",
    );
}

#[test]
fn extract_emits_class_in_function_method_as_local_method() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function outer() {\n  class Local {\n    method() {}\n  }\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    let classes = json_array_body(&stdout, "classes").unwrap_or_default();
    assert!(
        classes.contains("\"name\":\"Local\""),
        "draconic extract missed class-in-function Local: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"Local.method\""),
        "draconic extract missed Local.method in methods: {stdout}",
    );
    assert!(
        !methods.contains("outer.Local.method"),
        "draconic extract should emit class-in-function methods as Local.method: {stdout}",
    );
}

#[test]
fn extract_emits_nested_in_nested_method_with_class_chain() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class Outer {\n  static {\n    class Mid {\n      static {\n        class Inner {\n          ping() {}\n        }\n      }\n    }\n  }\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    assert!(
        methods.contains("\"name\":\"Outer.Mid.Inner.ping\""),
        "draconic extract missed Outer.Mid.Inner.ping in methods: {stdout}",
    );
}

#[test]
fn extract_emits_string_key_method_and_skips_constructor_accessor_static_private_computed() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class Outer {\n  constructor() {}\n  ping() {}\n  \"named\"() {}\n  get value() { return 1; }\n  static skipped() {}\n  #priv() {}\n  [k]() {}\n  x = 1;\n  static {\n    class Inner {\n      encode() {}\n    }\n  }\n}\nlet C = class Expr { method() {} };\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let classes = json_array_body(&stdout, "classes").unwrap_or_default();
    assert!(
        functions.trim().is_empty(),
        "draconic extract should not emit methods as functions: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"Outer.ping\""),
        "draconic extract missed Outer.ping in methods: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"Outer.named\""),
        "draconic extract missed string-key Outer.named in methods: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"Outer.Inner.encode\""),
        "draconic extract missed Outer.Inner.encode in methods: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"Outer.skipped\"") && methods.contains("\"static\":true"),
        "draconic extract missed static Outer.skipped in methods: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"Outer.#priv\""),
        "draconic extract missed private Outer.#priv in methods: {stdout}",
    );
    assert!(
        !methods.contains("constructor")
            && !methods.contains("value")
            && !methods.contains("Expr")
            && !classes.contains("Expr"),
        "draconic extract should skip constructor/accessor/computed/class expressions: {stdout}",
    );
}

#[test]
fn extract_method_calls_use_method_enclosing() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class Outer {\n  static {\n    class Inner {\n      ping() {\n        digest();\n      }\n    }\n  }\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        calls.contains("\"name\":\"digest\""),
        "draconic extract missed digest in calls: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"Outer.Inner.ping\""),
        "draconic extract missed enclosing Outer.Inner.ping on method calls: {stdout}",
    );
}

#[test]
fn extract_nested_function_inside_method_stays_bare_name() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class Outer {\n  method() {\n    function inner() {\n      digest();\n    }\n  }\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        functions.contains("\"name\":\"inner\""),
        "draconic extract missed nested inner in functions: {stdout}",
    );
    assert!(
        !functions.contains("Outer.method.inner") && !functions.contains("method.inner"),
        "draconic extract should emit nested functions inside methods as a bare name: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"inner\""),
        "draconic extract missed enclosing inner on nested function calls: {stdout}",
    );
}

#[test]
fn extract_nested_class_calls_use_outer_inner_enclosing() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class Outer {\n  static {\n    class Inner {\n      static {\n        ping();\n      }\n    }\n  }\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        calls.contains("\"name\":\"ping\""),
        "draconic extract missed ping in calls: {stdout}"
    );
    assert!(
        calls.contains("\"enclosing\":\"Outer.Inner\""),
        "draconic extract missed enclosing Outer.Inner on nested class calls: {stdout}"
    );
}

#[test]
fn extract_emits_unassigned_arrow_as_functions_arrow_line() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "void (() => {});
",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("\"version\":1"),
        "draconic extract JSON version should stay integer 1: {stdout}",
    );
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    assert!(
        functions.contains("\"name\":\"arrow:1\""),
        "draconic extract missed arrow:1 in functions: {stdout}",
    );
    assert!(
        !functions.contains("\"name\":\"arrow\""),
        "draconic extract should not emit a bare arrow: {stdout}",
    );
    assert!(
        !methods.contains("arrow"),
        "draconic extract should not emit unnamed arrows as methods: {stdout}",
    );
}

#[test]
fn extract_arrow_body_calls_use_arrow_line_enclosing() {
    let dir = temp_dir();
    let path = write_program(&dir, "hash.drac", "void (() => {\n  hit();\n});\n");
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        functions.contains("\"name\":\"arrow:1\""),
        "draconic extract missed arrow:1 in functions: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"hit\""),
        "draconic extract missed hit in calls: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"arrow:1\""),
        "draconic extract missed enclosing arrow:1 on arrow body calls: {stdout}",
    );
}

#[test]
fn extract_nested_function_inside_arrow_stays_bare() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "void (() => {\n  function inner() {\n    hit();\n  }\n});\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        functions.contains("\"name\":\"arrow:1\""),
        "draconic extract missed arrow:1 in functions: {stdout}",
    );
    assert!(
        functions.contains("\"name\":\"inner\""),
        "draconic extract missed nested inner in functions: {stdout}",
    );
    assert!(
        !functions.contains("arrow:1.inner"),
        "draconic extract should emit nested FunctionDeclaration inside an arrow as a bare name: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"inner\""),
        "draconic extract missed enclosing inner on nested function calls: {stdout}",
    );
}

#[test]
fn extract_emits_unassigned_function_expression_as_functions_function_line() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "void (function () { hit(); });\nvoid (function* () { ping(); });\nvoid (async function () { done(); });\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        functions.contains("\"name\":\"function:1\""),
        "draconic extract missed function:1 in functions: {stdout}",
    );
    assert!(
        functions.contains("\"name\":\"function:2\""),
        "draconic extract missed function:2 in functions: {stdout}",
    );
    assert!(
        functions.contains("\"name\":\"function:3\""),
        "draconic extract missed function:3 in functions: {stdout}",
    );
    assert!(
        !functions.contains("\"name\":\"function\""),
        "draconic extract should not emit a bare function: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"function:1\""),
        "draconic extract missed enclosing function:1 on function body calls: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"function:2\""),
        "draconic extract missed enclosing function:2 on generator body calls: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"function:3\""),
        "draconic extract missed enclosing function:3 on async function body calls: {stdout}",
    );
}

#[test]
fn extract_skips_named_function_expression() {
    let dir = temp_dir();
    let path = write_program(&dir, "hash.drac", "void (function foo() { hit(); });\n");
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        !functions.contains("function:1"),
        "draconic extract should skip named FunctionExpression line-names: {stdout}",
    );
    assert!(
        !functions.contains("\"name\":\"foo\""),
        "draconic extract should skip named FunctionExpression inner names: {stdout}",
    );
    assert!(
        !functions.contains("\"name\":\"function\""),
        "draconic extract should not emit a bare function: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"hit\""),
        "draconic extract missed hit in named FunctionExpression body: {stdout}",
    );
    assert!(
        !calls.contains("\"enclosing\":\"foo\"") && !calls.contains("\"enclosing\":\"function:1\""),
        "draconic extract should not enclose named FunctionExpression body calls: {stdout}",
    );
}

#[test]
fn extract_does_not_steal_simple_identifier_assignment() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "const digest = () => { digestHit(); };\nconst wrapped = (() => { wrappedHit(); });\nlet named = function () { namedHit(); };\nvar gen = function* () { genHit(); };\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    assert!(
        !functions.contains("arrow:"),
        "draconic extract should not steal assigned arrow line-names: {stdout}",
    );
    assert!(
        !functions.contains("function:"),
        "draconic extract should not steal assigned function line-names: {stdout}",
    );
    assert!(
        !functions.contains("\"name\":\"digest\"")
            && !functions.contains("\"name\":\"wrapped\"")
            && !functions.contains("\"name\":\"named\"")
            && !functions.contains("\"name\":\"gen\""),
        "draconic extract should not emit const/let/var lhs: {stdout}",
    );
}

#[test]
fn extract_destructured_and_export_default_unnamed_arrows_get_line_names() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "const { foo } = () => { bar(); };\nexport default () => { hit(); };\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        functions.contains("\"name\":\"arrow:1\""),
        "draconic extract missed destructured unnamed arrow line-name: {stdout}",
    );
    assert!(
        functions.contains("\"name\":\"arrow:2\""),
        "draconic extract missed export default unnamed arrow line-name: {stdout}",
    );
    assert!(
        !functions.contains("\"name\":\"foo\"") && !functions.contains("\"name\":\"__default\""),
        "draconic extract should not emit destructured or default lhs: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"arrow:1\""),
        "draconic extract missed enclosing arrow:1 on destructured arrow calls: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"arrow:2\""),
        "draconic extract missed enclosing arrow:2 on export default arrow calls: {stdout}",
    );
}

#[test]
fn extract_waits_on_class_field_and_object_literal_property_arrows() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class Hasher { save = () => { hit(); } }\nconst api = { digest: () => { ping(); }, hash() { done(); } };\nvoid (class { });\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let classes = json_array_body(&stdout, "classes").unwrap_or_default();
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    assert!(
        !functions.contains("arrow:"),
        "draconic extract should wait on class-field and object-literal property arrows: {stdout}",
    );
    assert!(
        !functions.contains("function:"),
        "draconic extract should wait on object-literal methods: {stdout}",
    );
    assert!(
        !functions.contains("Hasher.save")
            && !functions.contains("api.digest")
            && !functions.contains("api.hash"),
        "draconic extract should not stamp Class.field or api.digest this slice: {stdout}",
    );
    assert!(
        classes.contains("\"name\":\"Hasher\""),
        "draconic extract missed Hasher class: {stdout}",
    );
    assert!(
        !classes.contains("\"name\":\"class\""),
        "draconic extract should wait on class expressions: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"api.digest\""),
        "draconic extract missed api.digest in methods: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"api.hash\""),
        "draconic extract missed api.hash in methods: {stdout}",
    );
    assert!(
        !methods.contains("Hasher.save"),
        "draconic extract should wait on class-field objects: {stdout}",
    );
}

#[test]
fn extract_emits_assigned_object_method_shorthand_as_lhs_key() {
    for src in [
        "const api = { hash() {} };\n",
        "let api = { hash() {} };\n",
        "var api = { hash() {} };\n",
    ] {
        let stdout = extract_stdout(src);
        let methods = json_array_body(&stdout, "methods").unwrap_or_default();
        let functions = json_array_body(&stdout, "functions").unwrap_or_default();
        assert!(
            methods.contains("\"name\":\"api.hash\""),
            "draconic extract missed api.hash in methods for {src}: {stdout}",
        );
        assert!(
            !functions.contains("api.hash") && !functions.contains("function:"),
            "draconic extract should keep assigned object methods in methods[] for {src}: {stdout}",
        );
    }
}

#[test]
fn extract_emits_assigned_object_function_valued_properties() {
    let stdout = extract_stdout("const api = { digest: () => {}, hash: function () {} };\n");
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    assert!(
        methods.contains("\"name\":\"api.digest\""),
        "draconic extract missed api.digest in methods: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"api.hash\""),
        "draconic extract missed api.hash in methods: {stdout}",
    );
    assert!(
        !functions.contains("arrow:") && !functions.contains("function:"),
        "draconic extract should not line-name assigned object function properties: {stdout}",
    );
}

#[test]
fn extract_emits_parenthesized_assigned_object_methods() {
    let stdout = extract_stdout("const api = ({ hash() {} });\n");
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    assert!(
        methods.contains("\"name\":\"api.hash\""),
        "draconic extract missed parenthesized api.hash in methods: {stdout}",
    );
}

#[test]
fn extract_emits_string_literal_computed_assigned_object_methods() {
    let stdout = extract_stdout("const api = { [\"hash\"]() {}, [\"digest\"]: () => {} };\n");
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    assert!(
        methods.contains("\"name\":\"api.hash\""),
        "draconic extract missed computed api.hash in methods: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"api.digest\""),
        "draconic extract missed computed api.digest in methods: {stdout}",
    );
    assert!(
        !functions.contains("arrow:"),
        "draconic extract should not line-name computed assigned object arrows: {stdout}",
    );
}
