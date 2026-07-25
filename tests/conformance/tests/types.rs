//! ROADMAP T01–T03: type annotations, object types, unions/intersections/narrowing.

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

// --- T02: structural object types + type aliases ---

#[test]
fn object_type_ann_accepts_matching_literal() {
    let program = parse("let p: { x: number; y: string } = { x: 1, y: \"a\" };").unwrap();
    let checked = check(program).unwrap();
    assert!(matches!(type_of(&checked, "p"), Type::Shape(_)));
}

#[test]
fn object_type_ann_rejects_missing_prop() {
    let program = parse("let p: { x: number; y: number } = { x: 1 };").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn object_type_ann_rejects_wrong_prop_type() {
    let program = parse("let p: { x: number } = { x: \"no\" };").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn structural_assign_extra_props_ok() {
    let program = parse(
        r#"
        let full: { x: number; y: number } = { x: 1, y: 2 };
        let part: { x: number } = full;
        "#,
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn structural_assign_missing_prop_errors() {
    let program = parse(
        r#"
        let part: { x: number } = { x: 1 };
        let full: { x: number; y: number } = part;
        "#,
    )
    .unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn type_alias_object_ok() {
    let program = parse(
        r#"
        type Point = { x: number; y: number };
        let p: Point = { x: 1, y: 2 };
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert!(matches!(type_of(&checked, "p"), Type::Shape(_)));
}

#[test]
fn type_alias_rejects_mismatch() {
    let program = parse(
        r#"
        type Point = { x: number; y: number };
        let p: Point = { x: 1 };
        "#,
    )
    .unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn type_alias_named_primitive() {
    let program = parse(
        r#"
        type Num = number;
        let n: Num = 1;
        let bad: Num = "x";
        "#,
    )
    .unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn member_access_uses_shape_prop_type() {
    let program = parse(
        r#"
        let p: { x: number } = { x: 1 };
        let n: number = p.x;
        let bad: string = p.x;
        "#,
    )
    .unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable")
            && err.message.contains("number")
            && err.message.contains("string"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn object_types_erase_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/object_types_erase"),
        "missing types/object_types_erase fixture, got {ids:?}"
    );
}

#[test]
fn object_types_erase_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/object_types_erase")
        .expect("types/object_types_erase");
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
fn duplicate_type_alias_errors() {
    let program = parse("type A = number; type A = string;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("duplicate type alias"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn parse_object_type_and_alias() {
    let program = parse("type P = { x: number, y: string }; let p: P = { x: 1, y: \"a\" };")
        .unwrap();
    assert!(!program.body.is_empty());
    check(program).unwrap();
}

// --- T03: unions, intersections, typeof narrowing ---

#[test]
fn union_accepts_each_member() {
    let program = parse(
        r#"
        let a: string | number = 1;
        let b: string | number = "hi";
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert!(matches!(type_of(&checked, "a"), Type::Union(_)));
    assert!(matches!(type_of(&checked, "b"), Type::Union(_)));
}

#[test]
fn union_rejects_outside_member() {
    let program = parse(r#"let a: string | number = true;"#).unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn union_member_not_assignable_to_single() {
    let program = parse(
        r#"
        let u: string | number = 1;
        let n: number = u;
        "#,
    )
    .unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn type_alias_union() {
    let program = parse(
        r#"
        type StrOrNum = string | number;
        let x: StrOrNum = "a";
        let y: StrOrNum = false;
        "#,
    )
    .unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn intersection_object_merge() {
    let program = parse(
        r#"
        type A = { x: number };
        type B = { y: string };
        let o: A & B = { x: 1, y: "a" };
        let n: number = o.x;
        let s: string = o.y;
        "#,
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn intersection_requires_all_props() {
    let program = parse(
        r#"
        type A = { x: number };
        type B = { y: string };
        let o: A & B = { x: 1 };
        "#,
    )
    .unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn intersection_assignable_to_each_part() {
    let program = parse(
        r#"
        type A = { x: number };
        type B = { y: string };
        let o: A & B = { x: 1, y: "a" };
        let a: A = o;
        let b: B = o;
        "#,
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn typeof_narrow_string_branch() {
    let program = parse(
        r#"
        function f(x: string | number): string {
          if (typeof x === "string") {
            let s: string = x;
            return s;
          } else {
            let n: number = x;
            return "n";
          }
        }
        "#,
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn typeof_narrow_rejects_wrong_branch() {
    let program = parse(
        r#"
        function f(x: string | number): number {
          if (typeof x === "string") {
            let n: number = x;
            return n;
          }
          return 0;
        }
        "#,
    )
    .unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn typeof_narrow_else_branch() {
    let program = parse(
        r#"
        function f(x: string | number): number {
          if (typeof x === "string") {
            return 0;
          } else {
            let n: number = x;
            return n;
          }
        }
        "#,
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn typeof_narrow_flipped_operands() {
    let program = parse(
        r#"
        function f(x: string | number): string {
          if ("string" === typeof x) {
            return x;
          }
          return "n";
        }
        "#,
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn union_erase_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/union_intersection_erase"),
        "missing types/union_intersection_erase fixture, got {ids:?}"
    );
}

#[test]
fn union_erase_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/union_intersection_erase")
        .expect("types/union_intersection_erase");
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
fn parse_union_and_intersection() {
    let program = parse("type T = string | number; type U = { a: number } & { b: string };")
        .unwrap();
    assert!(!program.body.is_empty());
    check(program).unwrap();
}
