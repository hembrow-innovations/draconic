//! Member calls and instance tracking for `draconic extract`.

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
