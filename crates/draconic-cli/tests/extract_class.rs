//! Class members, tagged templates, and re-exports for `draconic extract`.

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
