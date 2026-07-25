//! ROADMAP T01: type annotations on bindings and functions (compiler).

use draconic_check::{check, BoundProgram, CheckedProgram, Type};
use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};
use draconic_parser::parse;

fn user_sym<'a>(bound: &'a BoundProgram, name: &str) -> &'a draconic_check::Symbol {
    // Prefer the last declaration (user code; builtins use dummy spans at the front).
    bound
        .symbols()
        .iter()
        .filter(|s| s.name == name)
        .last()
        .unwrap_or_else(|| panic!("no symbol `{name}`"))
}

fn type_of(checked: &CheckedProgram, name: &str) -> Type {
    let s = user_sym(&checked.bound, name);
    checked.type_of_symbol(s.id)
}

#[test]
fn annotations_erase_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/annotations_erase"),
        "missing types/annotations_erase fixture, got {ids:?}"
    );
}

#[test]
fn annotations_erase_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/annotations_erase")
        .expect("types/annotations_erase");
    assert!(fixture.targets.contains(&Target::Js));
    assert!(fixture.targets.contains(&Target::Native));
    for r in run_fixture(fixture) {
        assert!(
            r.ok,
            "{} @ {}: {}",
            r.fixture_id,
            r.target.as_str(),
            r.message
        );
    }
}

#[test]
fn let_annotation_sets_binding_type() {
    let program = parse("let x: number = 1;").unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "x"), Type::Number);
}

#[test]
fn let_annotation_rejects_mismatched_init() {
    let program = parse("let x: number = \"no\";").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable")
            && err.message.contains("string")
            && err.message.contains("number"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn let_annotation_rejects_mismatched_assign() {
    let program = parse("let x: number = 1; x = \"no\";").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("string") && err.message.contains("number"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn const_annotation_ok() {
    let program = parse("const s: string = \"ok\";").unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "s"), Type::String);
}

#[test]
fn param_annotation_sets_param_type() {
    let program = parse("function f(a: number) { let b: number = a; }").unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "a"), Type::Number);
    assert_eq!(type_of(&checked, "b"), Type::Number);
}

#[test]
fn param_default_must_match_annotation() {
    let program = parse("function f(a: number = \"x\") { return a; }").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn return_type_rejects_mismatch() {
    let program = parse("function f(): number { return \"x\"; }").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable")
            && err.message.contains("string")
            && err.message.contains("number"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn return_type_accepts_match() {
    let program = parse("function f(): number { return 1; }").unwrap();
    check(program).unwrap();
}

#[test]
fn arrow_return_type_rejects_mismatch() {
    let program = parse("let f = (x: number): string => x;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn arrow_return_type_accepts_match() {
    let program = parse("let f = (x: number): number => x;").unwrap();
    check(program).unwrap();
}

#[test]
fn unknown_type_name_errors() {
    let program = parse("let x: Widget = 1;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("unknown type") && err.message.contains("Widget"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn bare_return_rejects_concrete_return_type() {
    let program = parse("function f(): number { return; }").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("return"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn parse_rejects_missing_type_name() {
    let err = parse("let x: = 1;").unwrap_err();
    assert!(
        err.message.contains("type name") || err.message.contains("expected"),
        "unexpected: {}",
        err.message
    );
}
