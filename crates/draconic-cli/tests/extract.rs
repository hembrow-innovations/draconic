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

#[test]
fn extract_keeps_class_methods_in_methods() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "class Hasher {\n  encode() { hit(); }\n}\nvoid (function () { ping(); });\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    let functions = json_array_body(&stdout, "functions").unwrap_or_default();
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    assert!(
        methods.contains("\"name\":\"Hasher.encode\""),
        "draconic extract missed Hasher.encode in methods: {stdout}",
    );
    assert!(
        !functions.contains("Hasher.encode"),
        "draconic extract should keep methods in methods[]: {stdout}",
    );
    assert!(
        functions.contains("\"name\":\"function:4\""),
        "draconic extract missed function:4 after class methods: {stdout}",
    );
}

#[test]
fn extract_emits_member_call_last_identifier_with_member_true() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function encodePassword() {}\nfunction hashPassword() {\n  encodePassword(); hasher.digest();\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with("{\"version\":1,"),
        "draconic extract JSON version must stay integer 1: {stdout}",
    );
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        calls.contains("\"name\":\"digest\""),
        "draconic extract missed last-id digest in calls: {stdout}",
    );
    assert!(
        calls.contains("\"member\":true"),
        "draconic extract missed member: true on digest: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"hashPassword\""),
        "draconic extract missed enclosing hashPassword on member calls: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"encodePassword\""),
        "draconic extract missed encodePassword in calls: {stdout}",
    );
    assert!(
        !calls.contains("hasher.digest"),
        "draconic extract should emit last identifier digest, not hasher.digest: {stdout}",
    );
}

#[test]
fn extract_emits_optional_chain_member_call_same_as_member() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function hashPassword() {\n  hasher?.digest();\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with("{\"version\":1,"),
        "draconic extract JSON version must stay integer 1: {stdout}",
    );
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        calls.contains("\"name\":\"digest\""),
        "draconic extract missed last-id digest on optional-chain: {stdout}",
    );
    assert!(
        calls.contains("\"member\":true"),
        "draconic extract missed member: true on optional-chain digest: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"hashPassword\""),
        "draconic extract missed enclosing hashPassword on optional-chain: {stdout}",
    );
    assert!(
        !calls.contains("hasher.digest") && !calls.contains("hasher?.digest"),
        "draconic extract should emit last identifier digest for optional-chain: {stdout}",
    );
}

#[test]
fn extract_emits_nested_call_as_member_call_dunder() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function hashPassword() {\n  encodePassword(); hasher.digest(); factory()();\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with("{\"version\":1,"),
        "draconic extract JSON version must stay integer 1: {stdout}",
    );
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        calls.contains("\"name\":\"__call__\""),
        "draconic extract missed name __call__ for nested factory()(): {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"__call__\"") && calls.contains("\"member\":true"),
        "draconic extract missed member: true on nested __call__: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"factory\""),
        "draconic extract missed inner identifier factory for nested factory()(): {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"factory\",\"startLine\":2,\"endLine\":2,\"enclosing\":\"hashPassword\",\"member\":true"),
        "draconic extract should keep inner factory as a D3 identifier item: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"encodePassword\""),
        "draconic extract missed encodePassword in calls: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"digest\""),
        "draconic extract missed last-id digest in calls: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"hashPassword\""),
        "draconic extract missed enclosing hashPassword on nested calls: {stdout}",
    );
}

#[test]
fn extract_emits_parenthesized_nested_call_as_member_call_dunder() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function hashPassword() {\n  (factory())();\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with("{\"version\":1,"),
        "draconic extract JSON version must stay integer 1: {stdout}",
    );
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        calls.contains("\"name\":\"__call__\""),
        "draconic extract missed name __call__ for parenthesized nested call: {stdout}",
    );
    assert!(
        calls.contains("\"member\":true"),
        "draconic extract missed member: true on parenthesized nested __call__: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"factory\""),
        "draconic extract missed inner identifier factory for parenthesized nested call: {stdout}",
    );
}

#[test]
fn extract_emits_computed_string_member_call() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function hashPassword() {\n  foo[\"bar\"]();\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with("{\"version\":1,"),
        "draconic extract JSON version must stay integer 1: {stdout}",
    );
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        calls.contains("\"name\":\"bar\""),
        "draconic extract missed computed-string bar in calls: {stdout}",
    );
    assert!(
        calls.contains("\"member\":true"),
        "draconic extract missed member: true on computed-string bar: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"hashPassword\""),
        "draconic extract missed enclosing hashPassword on computed-string call: {stdout}",
    );
}

#[test]
fn extract_emits_optional_computed_string_member_call() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function hashPassword() {\n  foo?.[\"bar\"]();\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with("{\"version\":1,"),
        "draconic extract JSON version must stay integer 1: {stdout}",
    );
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        calls.contains("\"name\":\"bar\""),
        "draconic extract missed optional computed-string bar in calls: {stdout}",
    );
    assert!(
        calls.contains("\"member\":true"),
        "draconic extract missed member: true on optional computed-string bar: {stdout}",
    );
}

#[test]
fn extract_skips_non_string_computed_and_does_not_emit_new() {
    let dir = temp_dir();
    let path = write_program(
        &dir,
        "hash.drac",
        "function hashPassword() {\n  foo[bar]();\n  foo[1]();\n  foo[``]();\n  foo[\"\"]();\n  foo[\"bar\"];\n  new Foo()();\n}\n",
    );
    let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.trim_start().starts_with("{\"version\":1,"),
        "draconic extract JSON version must stay integer 1: {stdout}",
    );
    let calls = json_array_body(&stdout, "calls").unwrap_or_default();
    assert!(
        !calls.contains("\"name\":\"bar\""),
        "draconic extract should skip identifier computed keys: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"1\""),
        "draconic extract should skip number computed keys: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"Foo\""),
        "draconic extract should not emit new this slice: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract should not emit nested __call__ for new Foo()(): {stdout}",
    );
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

fn calls_json(stdout: &str) -> String {
    json_array_body(stdout, "calls").unwrap_or_default()
}

#[test]
fn extract_tracks_const_new_identifier_as_member_call() {
    let stdout = extract_stdout(
        "function runDracInstance() {\n  const hasher = new PasswordHasher();\n  hasher();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"")
            && calls.contains("\"enclosing\":\"runDracInstance\"")
            && calls.contains("\"member\":true"),
        "draconic extract missed member: true __call__ for runDracInstance: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"hasher\""),
        "draconic extract should not unique-name hasher() after new PasswordHasher: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"PasswordHasher\""),
        "draconic extract should not emit new as a calls[] item: {stdout}",
    );
}

#[test]
fn extract_tracks_let_var_and_assignment_new_instance_calls() {
    let stdout = extract_stdout(
        "function hit() {\n  let a = new Ctor();\n  var b = new Ctor();\n  c = new Ctor();\n  a();\n  b();\n  c();\n}\n",
    );
    let calls = calls_json(&stdout);
    let call_count = calls.matches("\"name\":\"__call__\"").count();
    assert_eq!(
        call_count, 3,
        "draconic extract missed let/var/assignment instance __call__: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"a\"")
            && !calls.contains("\"name\":\"b\"")
            && !calls.contains("\"name\":\"c\""),
        "draconic extract should not unique-name instance calls: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"Ctor\""),
        "draconic extract should not emit new as a calls[] item: {stdout}",
    );
}

#[test]
fn extract_tracks_member_expression_ctor() {
    let stdout =
        extract_stdout("function hit() {\n  const hasher = new Foo.Bar();\n  hasher();\n}\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"") && calls.contains("\"member\":true"),
        "draconic extract missed member ctor instance __call__: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"hasher\""),
        "draconic extract should not unique-name hasher after new Foo.Bar: {stdout}",
    );
}

#[test]
fn extract_instance_last_wins_and_non_shape_clear() {
    let stdout = extract_stdout(
        "function hit() {\n  let hasher = other;\n  hasher();\n  hasher = new Ctor();\n  hasher();\n  hasher = other;\n  hasher();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert_eq!(
        calls.matches("\"name\":\"hasher\"").count(),
        2,
        "draconic extract should unique-name hasher before bind and after clear: {stdout}",
    );
    assert_eq!(
        calls.matches("\"name\":\"__call__\"").count(),
        1,
        "draconic extract should last-wins instance __call__ once: {stdout}",
    );
}

#[test]
fn extract_alias_snapshot_survives_source_clear() {
    let stdout = extract_stdout(
        "function hit() {\n  let hasher = new Ctor();\n  const y = hasher;\n  hasher = other;\n  y();\n  hasher();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"") && calls.contains("\"member\":true"),
        "draconic extract missed alias snapshot __call__: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"y\""),
        "draconic extract should not unique-name alias y(): {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"hasher\""),
        "draconic extract should unique-name hasher after clear: {stdout}",
    );
}

#[test]
fn extract_nested_copy_at_def_inner_clear_stays_local() {
    let stdout = extract_stdout(
        "function hit() {\n  const hasher = new Ctor();\n  function inner() {\n    hasher();\n    const hasher = other;\n    hasher();\n  }\n  hasher();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"enclosing\":\"inner\"") && calls.contains("\"name\":\"__call__\""),
        "draconic extract missed nested copy-at-def __call__: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"hasher\"") && calls.contains("\"enclosing\":\"inner\""),
        "draconic extract missed inner last-wins unique-name: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"hit\"") && calls.contains("\"name\":\"__call__\""),
        "draconic extract inner clear should stay local to nested fn: {stdout}",
    );
}

#[test]
fn extract_param_catch_loop_destructure_unique_name() {
    let stdout = extract_stdout(
        "function hit() {\n  const hasher = new Ctor();\n  function withParam(hasher) { hasher(); }\n  function withDestructure({hasher}) { hasher(); }\n  try {} catch (hasher) { hasher(); }\n  for (const hasher of xs) { hasher(); }\n  hasher();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"enclosing\":\"withParam\"") && calls.contains("\"name\":\"hasher\""),
        "draconic extract param should unique-name hasher: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"withDestructure\"")
            && calls.contains("\"name\":\"hasher\""),
        "draconic extract destructure param should unique-name hasher: {stdout}",
    );
    assert_eq!(
        calls.matches("\"name\":\"hasher\"").count(),
        4,
        "draconic extract param/catch/loop/destructure should unique-name hasher four times: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"hit\"") && calls.contains("\"name\":\"__call__\""),
        "draconic extract outer hasher() should stay instance __call__: {stdout}",
    );
    assert!(
        !calls.contains("\"enclosing\":\"withParam\",\"member\":true")
            && !calls.contains("\"enclosing\":\"withDestructure\",\"member\":true"),
        "draconic extract param/destructure should not emit instance __call__: {stdout}",
    );
}

#[test]
fn extract_use_before_assignment_unique_names() {
    let stdout = extract_stdout(
        "function hit() {\n  hasher();\n  const hasher = new Ctor();\n  hasher();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert_eq!(
        calls.matches("\"name\":\"hasher\"").count(),
        1,
        "draconic extract should unique-name use before binding: {stdout}",
    );
    assert_eq!(
        calls.matches("\"name\":\"__call__\"").count(),
        1,
        "draconic extract should track hasher after new: {stdout}",
    );
}

#[test]
fn extract_obj_without_binding_unique_names() {
    let stdout = extract_stdout("function hit() { obj(); }\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"obj\""),
        "draconic extract missed unique-name obj(): {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract should not instance-call unbound obj(): {stdout}",
    );
}

#[test]
fn extract_nested_factory_call_stays_and_digest_stays_member() {
    let stdout = extract_stdout(
        "function hit() {\n  const hasher = new Ctor();\n  hasher.digest();\n  factory()();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"digest\"") && calls.contains("\"member\":true"),
        "draconic extract hasher.digest() should stay member: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"factory\""),
        "draconic extract missed identifier factory for nested factory()(): {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"__call__\""),
        "draconic extract missed nested factory()() __call__: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"hasher\""),
        "draconic extract should not unique-name hasher.digest: {stdout}",
    );
}

#[test]
fn extract_skips_computed_nested_call_subscript_ctor() {
    let stdout = extract_stdout(
        "function hit() {\n  const a = new foo[k]();\n  const b = new (factory());\n  a();\n  b();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"a\"") && calls.contains("\"name\":\"b\""),
        "draconic extract should unique-name skipped ctor RHS: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract should not instance-call skipped ctor RHS: {stdout}",
    );
}

#[test]
fn extract_named_import_seeds_later_function_instance_call() {
    let stdout = extract_stdout("import { hasher } from \"./x\";\nfunction hit() { hasher(); }\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"")
            && calls.contains("\"enclosing\":\"hit\"")
            && calls.contains("\"member\":true"),
        "draconic extract missed member: true __call__ for named import local: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"hasher\""),
        "draconic extract should not unique-name hasher after named import seed: {stdout}",
    );
}

#[test]
fn extract_default_import_seeds_instance_call() {
    let stdout = extract_stdout("import hasher from \"./x\";\nfunction hit() { hasher(); }\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"")
            && calls.contains("\"enclosing\":\"hit\"")
            && calls.contains("\"member\":true"),
        "draconic extract missed member: true __call__ for default import local: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"hasher\""),
        "draconic extract should not unique-name hasher after default import seed: {stdout}",
    );
}

#[test]
fn extract_namespace_import_seeds_instance_call() {
    let stdout = extract_stdout("import * as hasher from \"./x\";\nfunction hit() { hasher(); }\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"")
            && calls.contains("\"enclosing\":\"hit\"")
            && calls.contains("\"member\":true"),
        "draconic extract missed member: true __call__ for namespace import local: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"hasher\""),
        "draconic extract should not unique-name hasher after namespace import seed: {stdout}",
    );
}

#[test]
fn extract_alias_import_seeds_local_not_imported_name() {
    let stdout = extract_stdout(
        "import { foo as hasher } from \"./x\";\nfunction hit() { hasher(); foo(); }\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"")
            && calls.contains("\"enclosing\":\"hit\"")
            && calls.contains("\"member\":true"),
        "draconic extract missed member: true __call__ for alias import local: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"hasher\""),
        "draconic extract should not unique-name hasher after alias import seed: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"foo\""),
        "draconic extract should unique-name unimported foo after alias seed: {stdout}",
    );
}

#[test]
fn extract_import_type_does_not_seed_instance_map() {
    let stdout =
        extract_stdout("import type { hasher } from \"./x\";\nfunction hit() { hasher(); }\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"hasher\"") && calls.contains("\"enclosing\":\"hit\""),
        "draconic extract should unique-name hasher after import type: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract should not seed import type locals: {stdout}",
    );
}

#[test]
fn extract_inline_type_specifier_does_not_seed() {
    let stdout = extract_stdout(
        "import { type hasher, other } from \"./x\";\nfunction hit() { hasher(); other(); }\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"hasher\"") && calls.contains("\"enclosing\":\"hit\""),
        "draconic extract should unique-name inline type hasher: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"__call__\"") && calls.contains("\"member\":true"),
        "draconic extract missed member: true __call__ for value specifier other: {stdout}",
    );
}

#[test]
fn extract_side_effect_import_seeds_nothing() {
    let stdout = extract_stdout("import \"./x\";\nfunction hit() { hasher(); }\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"hasher\""),
        "draconic extract should unique-name hasher after side-effect import: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract side-effect import should seed nothing: {stdout}",
    );
}

#[test]
fn extract_reexport_from_does_not_seed() {
    let stdout = extract_stdout("export { hasher } from \"./x\";\nfunction hit() { hasher(); }\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"hasher\""),
        "draconic extract should unique-name hasher after re-export: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract re-export should not seed instance map: {stdout}",
    );
}

#[test]
fn extract_require_and_dynamic_import_locals_wait() {
    let stdout = extract_stdout(
        "const a = require(\"x\");\nconst b = await import(\"x\");\nfunction hit() { a(); b(); }\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"a\"") && calls.contains("\"name\":\"b\""),
        "draconic extract should unique-name require/import() locals: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract require/import() locals should wait: {stdout}",
    );
}

#[test]
fn extract_import_is_not_a_call() {
    let stdout = extract_stdout("import { hasher } from \"./x\";\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.is_empty(),
        "draconic extract should not visit the import node as a call: {stdout}",
    );
}

#[test]
fn extract_imported_name_miss_unique_names() {
    let stdout = extract_stdout(
        "import { hashPassword, PasswordHasher } from \"./hash.drac\";\nfunction runDracImported() {\n  PasswordHasher();\n}\nfunction runDracImportedMiss() {\n  missingDracImported();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"")
            && calls.contains("\"enclosing\":\"runDracImported\"")
            && calls.contains("\"member\":true"),
        "draconic extract missed member: true __call__ for runDracImported: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"runDracImportedMiss\"")
            && calls.contains("\"name\":\"missingDracImported\""),
        "draconic extract missed unique-name missingDracImported: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"PasswordHasher\""),
        "draconic extract should not unique-name PasswordHasher after import seed: {stdout}",
    );
    assert!(
        !calls.split('{').any(|item| {
            item.contains("\"enclosing\":\"runDracImportedMiss\"")
                && item.contains("\"name\":\"__call__\"")
                && item.contains("\"member\":true")
        }),
        "draconic extract runDracImportedMiss should not member-call __call__: {stdout}",
    );
}

#[test]
fn extract_class_body_does_not_copy_module_instance_map() {
    let stdout = extract_stdout(
        "const hasher = new Ctor();\nclass Host { method() { hasher(); } }\nhasher();\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"enclosing\":\"Host.method\"") && calls.contains("\"name\":\"hasher\""),
        "draconic extract class-body should unique-name hasher without copying module map: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"__call__\"") && calls.contains("\"member\":true"),
        "draconic extract missed module-level hasher() __call__: {stdout}",
    );
}

#[test]
fn extract_module_new_later_function_copies_at_def() {
    let stdout = extract_stdout(
        "const moduleHasher = new PasswordHasher();\nfunction runDracModuleInstance() {\n  moduleHasher();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"")
            && calls.contains("\"enclosing\":\"runDracModuleInstance\"")
            && calls.contains("\"member\":true"),
        "draconic extract missed member: true __call__ for runDracModuleInstance: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"moduleHasher\""),
        "draconic extract should not unique-name moduleHasher() after module new: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"PasswordHasher\""),
        "draconic extract should not emit new as a calls[] item: {stdout}",
    );
}

#[test]
fn extract_module_later_arrow_function_line_and_assigned_copy_at_def() {
    let stdout = extract_stdout(
        "const hasher = new Ctor();\nvoid (() => { hasher(); });\n(function() { hasher(); });\nconst assigned = function() { hasher(); };\n",
    );
    let calls = calls_json(&stdout);
    assert_eq!(
        calls.matches("\"name\":\"__call__\"").count(),
        3,
        "draconic extract missed module copy-at-def __call__ for arrow/function:line/assigned: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"arrow:2\"")
            && calls.contains("\"enclosing\":\"function:3\""),
        "draconic extract missed arrow:2 / function:3 enclosing for module copy-at-def: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"hasher\""),
        "draconic extract should not unique-name hasher() after module new: {stdout}",
    );
}

#[test]
fn extract_module_inner_clear_stays_local() {
    let stdout = extract_stdout(
        "const hasher = new Ctor();\nfunction inner() {\n  hasher();\n  hasher = other;\n  hasher();\n}\nhasher();\n",
    );
    let calls = calls_json(&stdout);
    assert_eq!(
        calls.matches("\"name\":\"__call__\"").count(),
        2,
        "draconic extract inner clear should stay local to nested fn: {stdout}",
    );
    assert_eq!(
        calls.matches("\"name\":\"hasher\"").count(),
        1,
        "draconic extract missed inner last-wins unique-name: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"inner\"") && calls.contains("\"name\":\"__call__\""),
        "draconic extract missed nested copy-at-def __call__: {stdout}",
    );
}

#[test]
fn extract_module_level_identifier_emits_call_from_file_id() {
    let stdout = extract_stdout("const hasher = new Ctor();\nhasher();\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"") && calls.contains("\"member\":true"),
        "draconic extract missed module-level member: true __call__: {stdout}",
    );
    assert!(
        !calls.contains("\"enclosing\""),
        "draconic extract module-level hasher() should emit from the file id: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"hasher\""),
        "draconic extract should not unique-name module-level hasher(): {stdout}",
    );
}

#[test]
fn extract_module_function_before_new_unique_names() {
    let stdout = extract_stdout(
        "function early() { hasher(); }\nconst hasher = new Ctor();\nfunction late() { hasher(); }\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"enclosing\":\"early\"") && calls.contains("\"name\":\"hasher\""),
        "draconic extract should unique-name hasher from earlier def id: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"late\"") && calls.contains("\"name\":\"__call__\""),
        "draconic extract missed later function copy-at-def __call__: {stdout}",
    );
}

#[test]
fn extract_module_let_var_and_assignment_new_instance_calls() {
    let stdout = extract_stdout(
        "let a = new Ctor();\nvar b = new Ctor();\nc = new Ctor();\na();\nb();\nc();\n",
    );
    let calls = calls_json(&stdout);
    assert_eq!(
        calls.matches("\"name\":\"__call__\"").count(),
        3,
        "draconic extract missed module let/var/assignment instance __call__: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"a\"")
            && !calls.contains("\"name\":\"b\"")
            && !calls.contains("\"name\":\"c\"")
            && !calls.contains("\"name\":\"Ctor\""),
        "draconic extract should not unique-name module instance calls or emit new: {stdout}",
    );
}

#[test]
fn extract_class_field_then_method_instance_calls() {
    let stdout = extract_stdout(
        "class DracHasherBox {\n  box = new PasswordHasher();\n  runDracClassInstance() {\n    box();\n  }\n}\nvoid box();\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"")
            && calls.contains("\"enclosing\":\"DracHasherBox.runDracClassInstance\"")
            && calls.contains("\"member\":true"),
        "draconic extract missed member: true __call__ for DracHasherBox.runDracClassInstance: {stdout}",
    );
    assert_eq!(
        calls.matches("\"name\":\"__call__\"").count(),
        1,
        "draconic extract should not emit member __call__ from the file id for box: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"box\""),
        "draconic extract missed unique-name box() from the file id: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"PasswordHasher\""),
        "draconic extract should not emit new as a calls[] item: {stdout}",
    );
}

#[test]
fn extract_class_method_before_field_unique_names() {
    let stdout = extract_stdout("class Host {\n  method() { box(); }\n  box = new Ctor();\n}\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"enclosing\":\"Host.method\"") && calls.contains("\"name\":\"box\""),
        "draconic extract should unique-name box from the earlier method id: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract should not instance-call a method defined before the class-body field: {stdout}",
    );
}

#[test]
fn extract_class_in_function_does_not_copy_parent_map() {
    let stdout = extract_stdout(
        "function hit() {\n  const hasher = new Ctor();\n  class Host {\n    box = new Ctor();\n    method() {\n      box();\n      hasher();\n    }\n  }\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"enclosing\":\"Host.method\"") && calls.contains("\"name\":\"__call__\""),
        "draconic extract missed class-body box() __call__: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"Host.method\"") && calls.contains("\"name\":\"hasher\""),
        "draconic extract class-body should unique-name hasher without copying parent map: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"box\""),
        "draconic extract should not unique-name box() after class-body new: {stdout}",
    );
}

#[test]
fn extract_emits_nested_constructor_as_outer_inner_constructor() {
    let stdout = extract_stdout(
        "class Outer {\n  static {\n    class Inner {\n      constructor() {}\n    }\n  }\n}\n",
    );
    let constructors = json_array_body(&stdout, "constructors").unwrap_or_default();
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    assert!(
        constructors.contains("\"name\":\"Outer.Inner.constructor\""),
        "draconic extract missed Outer.Inner.constructor in constructors: {stdout}",
    );
    assert!(
        !constructors.contains("\"name\":\"Outer.constructor\""),
        "draconic extract should skip Outer's implicit default constructor: {stdout}",
    );
    assert!(
        !methods.contains("constructor"),
        "draconic extract should not emit constructors as methods: {stdout}",
    );
}

#[test]
fn extract_emits_class_in_function_and_method_constructor_as_local_constructor() {
    let stdout = extract_stdout(
        "function outer() {\n  class Local {\n    constructor() {}\n  }\n}\nclass Host {\n  method() {\n    class Local {\n      constructor() {}\n    }\n  }\n}\n",
    );
    let constructors = json_array_body(&stdout, "constructors").unwrap_or_default();
    assert!(
        constructors.contains("\"name\":\"Local.constructor\""),
        "draconic extract missed Local.constructor in constructors: {stdout}",
    );
    assert!(
        !constructors.contains("outer.Local.constructor")
            && !constructors.contains("Host.Local.constructor"),
        "draconic extract should emit class-in-function-or-method constructors as Local.constructor: {stdout}",
    );
}

#[test]
fn extract_skips_implicit_default_and_static_constructor() {
    let stdout = extract_stdout(
        "class Hasher {}\nclass Box {\n  constructor() {}\n  static constructor() {}\n}\n",
    );
    let constructors = json_array_body(&stdout, "constructors").unwrap_or_default();
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    assert!(
        constructors.contains("\"name\":\"Box.constructor\""),
        "draconic extract missed Box.constructor in constructors: {stdout}",
    );
    assert!(
        !constructors.contains("Hasher.constructor"),
        "draconic extract should skip implicit default constructors: {stdout}",
    );
    assert_eq!(
        constructors.matches("constructor").count(),
        1,
        "draconic extract should not emit static constructor in constructors: {stdout}",
    );
    assert!(
        constructors.contains("\"name\":\"Box.constructor\",\"startLine\":3,\"endLine\":3")
            && !constructors.contains("\"static\":true"),
        "draconic extract should keep instance constructors in constructors[] without static: {stdout}",
    );
    assert!(
        methods.contains(
            "\"name\":\"Box.constructor\",\"startLine\":4,\"endLine\":4,\"static\":true",
        ),
        "draconic extract missed static constructor in methods[] with static: {stdout}",
    );
    assert!(
        stdout.contains("\"version\":1"),
        "draconic extract JSON version should stay integer 1: {stdout}",
    );
}

#[test]
fn extract_emits_get_set_accessors_not_methods() {
    let stdout = extract_stdout(
        "class Foo {\n  get value() { return 1; }\n  set value(v) {}\n  get() {}\n}\n",
    );
    let accessors = json_array_body(&stdout, "accessors").unwrap_or_default();
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    assert!(
        accessors.contains("\"name\":\"Foo.get.value\"")
            && accessors.contains("\"accessor\":\"get\""),
        "draconic extract missed Foo.get.value in accessors: {stdout}",
    );
    assert!(
        accessors.contains("\"name\":\"Foo.set.value\"")
            && accessors.contains("\"accessor\":\"set\""),
        "draconic extract missed Foo.set.value in accessors: {stdout}",
    );
    assert!(
        !methods.contains("Foo.get.value") && !methods.contains("Foo.set.value"),
        "draconic extract should not emit accessors as methods: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"Foo.get\"") && !methods.contains("\"accessor\":\"get\""),
        "draconic extract should keep ordinary method named get as Type.get with no accessor: {stdout}",
    );
    assert!(
        stdout.contains("\"version\":1"),
        "draconic extract JSON version should stay integer 1: {stdout}",
    );
}

#[test]
fn extract_emits_private_string_key_accessors_and_skips_computed() {
    let stdout = extract_stdout(
        "class Foo {\n  get #foo() { return 1; }\n  set #foo(v) {}\n  get \"named\"() { return 1; }\n  get [k]() { return 1; }\n}\nclass Outer {\n  static {\n    class Inner {\n      get value() { return 1; }\n    }\n  }\n}\n",
    );
    let accessors = json_array_body(&stdout, "accessors").unwrap_or_default();
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    assert!(
        accessors.contains("\"name\":\"Foo.get.#foo\"")
            && accessors.contains("\"accessor\":\"get\""),
        "draconic extract missed Foo.get.#foo in accessors: {stdout}",
    );
    assert!(
        accessors.contains("\"name\":\"Foo.set.#foo\"")
            && accessors.contains("\"accessor\":\"set\""),
        "draconic extract missed Foo.set.#foo in accessors: {stdout}",
    );
    assert!(
        accessors.contains("\"name\":\"Foo.get.named\""),
        "draconic extract missed string-key Foo.get.named in accessors: {stdout}",
    );
    assert!(
        accessors.contains("\"name\":\"Outer.Inner.get.value\""),
        "draconic extract missed Outer.Inner.get.value in accessors: {stdout}",
    );
    assert!(
        !accessors.contains("[k]") && !methods.contains("get.value"),
        "draconic extract should skip computed accessor keys and not emit accessors as methods: {stdout}",
    );
}

#[test]
fn extract_emits_static_and_private_function_members() {
    let stdout = extract_stdout(
        "class Foo {\n  static ping() {}\n  static save = () => {}\n  static {\n    class Inner {}\n  }\n  bar() {}\n  #probe() {}\n  #run = () => {}\n  get #foo() { return 1; }\n  #data = 1;\n}\n",
    );
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    let accessors = json_array_body(&stdout, "accessors").unwrap_or_default();
    let classes = json_array_body(&stdout, "classes").unwrap_or_default();
    assert!(
        methods.contains("\"name\":\"Foo.ping\",\"startLine\":2,\"endLine\":2,\"static\":true",),
        "draconic extract missed static Foo.ping in methods: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"Foo.save\",\"startLine\":3,\"endLine\":3,\"static\":true",),
        "draconic extract missed static function field Foo.save in methods: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"Foo.bar\",\"startLine\":7,\"endLine\":7")
            && !methods.contains("Foo.bar\",\"startLine\":7,\"endLine\":7,\"static\":true"),
        "draconic extract missed instance Foo.bar in methods without static: {stdout}",
    );
    assert!(
        methods.contains("\"name\":\"Foo.#probe\"") && methods.contains("\"name\":\"Foo.#run\""),
        "draconic extract missed private Foo.#probe / Foo.#run in methods: {stdout}",
    );
    assert!(
        accessors.contains("\"name\":\"Foo.get.#foo\"")
            && accessors.contains("\"accessor\":\"get\""),
        "draconic extract missed Foo.get.#foo in accessors: {stdout}",
    );
    assert!(
        classes.contains("\"name\":\"Foo.Inner\"") && !classes.contains("\"static\":true"),
        "draconic extract nested class in static block should stay D5 with no static: {stdout}",
    );
    assert!(
        !methods.contains("Foo.#data") && !classes.contains("static_block"),
        "draconic extract should skip private data fields and static blocks: {stdout}",
    );
}

#[test]
fn extract_skips_static_private_data_computed_and_class_expressions() {
    let stdout = extract_stdout(
        "class Foo {\n  static data = 1;\n  static [k]() {}\n  #count = 1;\n}\nvoid (class { static ping() {} #priv() {} });\nclass Outer {\n  static {\n    class Inner {}\n  }\n}\n",
    );
    let methods = json_array_body(&stdout, "methods").unwrap_or_default();
    let classes = json_array_body(&stdout, "classes").unwrap_or_default();
    assert!(
        methods.trim().is_empty(),
        "draconic extract should skip data fields, computed keys, and class-expression members: {stdout}",
    );
    assert!(
        classes.contains("\"name\":\"Foo\"")
            && classes.contains("\"name\":\"Outer\"")
            && classes.contains("\"name\":\"Outer.Inner\""),
        "draconic extract missed Foo / Outer / Outer.Inner in classes: {stdout}",
    );
    assert!(
        !classes.contains("Expr") && !methods.contains("ping") && !methods.contains("#priv"),
        "draconic extract should skip class expressions: {stdout}",
    );
}

#[test]
fn extract_class_constructor_does_not_instance_call() {
    let stdout = extract_stdout(
        "function hit() {\n  class Host {\n    box = new Ctor();\n    constructor() { box(); }\n  }\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract constructor bodies wait for D15: {stdout}",
    );
}

#[test]
fn extract_class_accessor_static_method_wait_for_d15() {
    let stdout = extract_stdout(
        "function hit() {\n  class Host {\n    box = new Ctor();\n    get x() { box(); return 1; }\n    static run() { box(); }\n  }\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract accessors and static methods wait for D15: {stdout}",
    );
}

#[test]
fn extract_class_private_and_static_fields_do_not_seed() {
    let stdout = extract_stdout(
        "class Host {\n  #box = new Ctor();\n  static other = new Ctor();\n  method() { box(); other(); }\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"box\"") && calls.contains("\"name\":\"other\""),
        "draconic extract private/static fields should unique-name until D15: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract private/static fields wait for D15: {stdout}",
    );
}

#[test]
fn extract_class_field_member_ctor_instance_calls() {
    let stdout = extract_stdout("class Host {\n  box = new Foo.Bar();\n  method() { box(); }\n}\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"__call__\"") && calls.contains("\"member\":true"),
        "draconic extract missed member ctor class-field instance __call__: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"box\""),
        "draconic extract should not unique-name box after new Foo.Bar: {stdout}",
    );
}

#[test]
fn extract_class_field_skipped_ctor_unique_names() {
    let stdout = extract_stdout(
        "class Host {\n  a = new foo[k]();\n  b = new (factory());\n  method() { a(); b(); }\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"a\"") && calls.contains("\"name\":\"b\""),
        "draconic extract should unique-name skipped class-field ctor RHS: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"__call__\""),
        "draconic extract should not instance-call skipped class-field ctor RHS: {stdout}",
    );
}

#[test]
fn extract_constructor_accessor_static_private_calls_use_member_enclosing() {
    let stdout = extract_stdout(
        "class Foo {\n  constructor() { hit(); new Bar(); }\n  get value() { ping(); return 1; }\n  static run() { digest(); }\n  #probe() { encode(); }\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"hit\"") && calls.contains("\"enclosing\":\"Foo.constructor\""),
        "draconic extract missed enclosing Foo.constructor on constructor body calls: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"ping\"") && calls.contains("\"enclosing\":\"Foo.get.value\""),
        "draconic extract missed enclosing Foo.get.value on accessor body calls: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"digest\"") && calls.contains("\"enclosing\":\"Foo.run\""),
        "draconic extract missed enclosing Foo.run on static method calls: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"encode\"") && calls.contains("\"enclosing\":\"Foo.#probe\""),
        "draconic extract missed enclosing Foo.#probe on private method calls: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"Bar\""),
        "draconic extract should not emit new C() as a calls item: {stdout}",
    );
}

#[test]
fn extract_emits_identifier_tagged_template() {
    let stdout = extract_stdout("function hit() {\n  tag`x`;\n}\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"tag\"") && calls.contains("\"enclosing\":\"hit\""),
        "draconic extract missed identifier tag in calls: {stdout}",
    );
    assert!(
        !calls.contains("\"member\":true"),
        "identifier tagged template should not set member: true: {stdout}",
    );
}

#[test]
fn extract_emits_member_tagged_template_last_identifier() {
    let stdout = extract_stdout("function hit() {\n  foo.bar`x`;\n}\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"bar\"") && calls.contains("\"member\":true"),
        "draconic extract missed member: true bar tagged template: {stdout}",
    );
    assert!(
        calls.contains("\"enclosing\":\"hit\""),
        "draconic extract missed enclosing hit on member tagged template: {stdout}",
    );
    assert!(
        !calls.contains("foo.bar"),
        "draconic extract should emit last identifier bar, not foo.bar: {stdout}",
    );
}

#[test]
fn extract_unwraps_parenthesized_tagged_template() {
    let stdout = extract_stdout("function hit() {\n  (tag)`x`;\n}\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"tag\"") && calls.contains("\"enclosing\":\"hit\""),
        "draconic extract missed parenthesized identifier tag: {stdout}",
    );
    assert!(
        !calls.contains("\"member\":true"),
        "parenthesized identifier tagged template should not set member: true: {stdout}",
    );
}

#[test]
fn extract_tagged_template_does_not_rewrite_instance_name() {
    let stdout = extract_stdout(
        "function hit() {\n  const hasher = new Ctor();\n  hasher`x`;\n  hasher();\n}\n",
    );
    let calls = calls_json(&stdout);
    assert_eq!(
        calls.matches("\"name\":\"hasher\"").count(),
        1,
        "draconic extract should unique-name hasher`x`, not rewrite to __call__: {stdout}",
    );
    assert_eq!(
        calls.matches("\"name\":\"__call__\"").count(),
        1,
        "draconic extract hasher() should still instance-call __call__: {stdout}",
    );
}

#[test]
fn extract_walks_tagged_template_interpolations() {
    let stdout = extract_stdout("function hit() {\n  tag`${foo()}`;\n}\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"tag\""),
        "draconic extract missed identifier tag around interpolation: {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"foo\""),
        "draconic extract missed foo() inside tagged template interpolation: {stdout}",
    );
}

#[test]
fn extract_emits_nested_call_on_tagged_template() {
    let stdout = extract_stdout("function hit() {\n  tag`x`();\n}\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"tag\"") && calls.contains("\"enclosing\":\"hit\""),
        "draconic extract missed tag call for tag`x`(): {stdout}",
    );
    assert!(
        calls.contains("\"name\":\"__call__\"") && calls.contains("\"member\":true"),
        "draconic extract missed nested member: true __call__ for tag`x`(): {stdout}",
    );
    assert!(
        !calls.contains(
            "\"name\":\"tag\",\"startLine\":2,\"endLine\":2,\"enclosing\":\"hit\",\"member\":true"
        ),
        "draconic extract should keep tag as a D3 identifier item: {stdout}",
    );
}

#[test]
fn extract_skips_private_and_computed_tagged_templates() {
    let stdout = extract_stdout(
        "class Foo {\n  #priv = 1;\n  hit() {\n    this.#priv`x`;\n    foo[\"bar\"]`x`;\n    foo[k]`x`;\n    tag`x`;\n  }\n}\n",
    );
    let calls = calls_json(&stdout);
    assert!(
        calls.contains("\"name\":\"tag\"") && calls.contains("\"enclosing\":\"Foo.hit\""),
        "draconic extract missed identifier tag beside skipped tags: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"priv\"") && !calls.contains("#priv"),
        "draconic extract should skip private #name tagged templates: {stdout}",
    );
    assert!(
        !calls.contains("\"name\":\"bar\""),
        "draconic extract should skip computed tagged templates: {stdout}",
    );
}

#[test]
fn extract_skips_untagged_template_literals_as_calls() {
    let stdout = extract_stdout("function hit() {\n  `a${foo()}b`;\n}\n");
    let calls = calls_json(&stdout);
    assert!(
        calls.is_empty(),
        "draconic extract should not emit untagged template interpolations as calls: {stdout}",
    );
}

#[test]
fn extract_emits_one_import_for_reexport_from_forms() {
    let cases = [
        "export { x } from \"./m\";\n",
        "export { x as y } from \"./m\";\n",
        "export * from \"./m\";\n",
        "export * as ns from \"./m\";\n",
    ];
    for src in cases {
        let stdout = extract_stdout(src);
        let imports = json_array_body(&stdout, "imports").unwrap_or_default();
        assert!(
            imports.contains("\"name\":\"./m\""),
            "draconic extract missed ./m in imports for {src}: {stdout}",
        );
        assert_eq!(
            imports.matches("\"name\":").count(),
            1,
            "draconic extract should emit one imports item for {src}: {stdout}",
        );
        assert!(
            !stdout.contains("\"reexport\""),
            "draconic extract should not emit a reexport key for {src}: {stdout}",
        );
    }
}

#[test]
fn extract_skips_named_export_without_from() {
    let stdout = extract_stdout("export { x };\n");
    let imports = json_array_body(&stdout, "imports").unwrap_or_default();
    assert!(
        imports.trim().is_empty(),
        "draconic extract should not emit imports for named export without from: {stdout}",
    );
}

#[test]
fn extract_skips_type_only_reexports_when_parser_has_those_trees() {
    let cases = [
        "export type { x } from \"./m\";\n",
        "export type * from \"./m\";\n",
    ];
    for src in cases {
        let dir = temp_dir();
        let path = write_program(&dir, "hash.drac", src);
        let (code, stdout, stderr) = run(draconic().arg("extract").arg(&path));
        if code != 0 {
            assert!(
                stderr.contains("export") || stderr.contains("type"),
                "parser lacks type-only re-export trees; extract should fail clearly: stderr={stderr}",
            );
            continue;
        }
        let imports = json_array_body(&stdout, "imports").unwrap_or_default();
        assert!(
            imports.trim().is_empty(),
            "draconic extract should skip type-only re-export {src}: {stdout}",
        );
        assert!(
            !stdout.contains("\"reexport\""),
            "draconic extract should not emit a reexport key for {src}: {stdout}",
        );
    }
}
