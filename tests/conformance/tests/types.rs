//! ROADMAP T01–T06: type annotations, object types, unions/intersections/narrowing, generics, native types, dual-worlds boundary.

use draconic_check::{check, BoundProgram, CheckedProgram, NativeType, Type};
use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};
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
fn annotations_erase_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/annotations_erase")
        .expect("types/annotations_erase");
    assert!(!fixture.targets.is_empty());
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
fn object_types_erase_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/object_types_erase")
        .expect("types/object_types_erase");
    assert!(!fixture.targets.is_empty());
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
fn union_erase_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/union_intersection_erase")
        .expect("types/union_intersection_erase");
    assert!(!fixture.targets.is_empty());
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

// --- T04: generics (functions, types) ---

#[test]
fn generic_type_alias_app() {
    let program = parse(
        r#"
        type Box<T> = { value: T };
        let n: Box<number> = { value: 1 };
        let s: Box<string> = { value: "a" };
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert!(matches!(type_of(&checked, "n"), Type::Shape(_)));
    assert!(matches!(type_of(&checked, "s"), Type::Shape(_)));
}

#[test]
fn generic_type_alias_rejects_wrong_arg() {
    let program = parse(
        r#"
        type Box<T> = { value: T };
        let bad: Box<number> = { value: "no" };
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
fn generic_type_alias_arity_error() {
    let program = parse(
        r#"
        type Box<T> = { value: T };
        let bad: Box = { value: 1 };
        "#,
    )
    .unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("type argument") || err.message.contains("generic"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn generic_type_alias_two_params() {
    let program = parse(
        r#"
        type Pair<A, B> = { a: A; b: B };
        let p: Pair<number, string> = { a: 1, b: "x" };
        let n: number = p.a;
        let s: string = p.b;
        "#,
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn generic_function_identity_infers() {
    let program = parse(
        r#"
        function id<T>(x: T): T { return x; }
        let n = id(1);
        let s = id("hi");
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "n"), Type::Number);
    assert_eq!(type_of(&checked, "s"), Type::String);
}

#[test]
fn generic_function_identity_rejects_mismatch() {
    let program = parse(
        r#"
        function id<T>(x: T): T { return x; }
        let bad: string = id(1);
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
fn generic_function_body_type_param() {
    let program = parse(
        r#"
        function id<T>(x: T): T {
          let y: T = x;
          return y;
        }
        "#,
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn generic_function_body_rejects_wrong_concrete() {
    let program = parse(
        r#"
        function id<T>(x: T): T {
          let y: number = x;
          return x;
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
fn generic_function_two_params_same_t_ok() {
    let program = parse(
        r#"
        function both<T>(a: T, b: T): T { return a; }
        let n = both(1, 2);
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "n"), Type::Number);
}

#[test]
fn generic_function_two_params_same_t_conflict() {
    let program = parse(
        r#"
        function both<T>(a: T, b: T): T { return a; }
        let bad = both(1, "x");
        "#,
    )
    .unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable")
            || err.message.contains("inferred")
            || err.message.contains("type parameter"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn generic_box_via_function() {
    let program = parse(
        r#"
        type Box<T> = { value: T };
        function box<T>(v: T): Box<T> {
          return { value: v };
        }
        let b = box(42);
        let n: number = b.value;
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "n"), Type::Number);
    assert!(matches!(type_of(&checked, "b"), Type::Shape(_)));
}

#[test]
fn generics_erase_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/generics_erase"),
        "missing types/generics_erase fixture, got {ids:?}"
    );
}

#[test]
fn generics_erase_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/generics_erase")
        .expect("types/generics_erase");
    assert!(!fixture.targets.is_empty());
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
fn parse_generic_alias_and_fn() {
    let program = parse(
        r#"
        type Id<T> = T;
        function f<T>(x: T): T { return x; }
        let a: Id<number> = 1;
        let b = f(a);
        "#,
    )
    .unwrap();
    assert!(!program.body.is_empty());
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "a"), Type::Number);
    assert_eq!(type_of(&checked, "b"), Type::Number);
}

// --- T05: native types in the type system ---

#[test]
fn native_types_erase_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/native/native_types_erase"),
        "missing types/native/native_types_erase fixture, got {ids:?}"
    );
}

#[test]
fn native_types_erase_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/native/native_types_erase")
        .expect("types/native/native_types_erase");
    assert!(!fixture.targets.is_empty());
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
fn i32_annotation_sets_binding_type() {
    let program = parse("let x: i32 = 1;").unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "x"), Type::Native(NativeType::I32));
}

#[test]
fn native_integer_and_float_names() {
    let program = parse(
        r#"
        let a: i8 = 1;
        let b: i16 = 2;
        let c: i32 = 3;
        let d: i64 = 4;
        let e: u8 = 5;
        let f: u16 = 6;
        let g: u32 = 7;
        let h: u64 = 8;
        let i: f32 = 1.5;
        let j: f64 = 2.5;
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "a"), Type::Native(NativeType::I8));
    assert_eq!(type_of(&checked, "b"), Type::Native(NativeType::I16));
    assert_eq!(type_of(&checked, "c"), Type::Native(NativeType::I32));
    assert_eq!(type_of(&checked, "d"), Type::Native(NativeType::I64));
    assert_eq!(type_of(&checked, "e"), Type::Native(NativeType::U8));
    assert_eq!(type_of(&checked, "f"), Type::Native(NativeType::U16));
    assert_eq!(type_of(&checked, "g"), Type::Native(NativeType::U32));
    assert_eq!(type_of(&checked, "h"), Type::Native(NativeType::U64));
    assert_eq!(type_of(&checked, "i"), Type::Native(NativeType::F32));
    assert_eq!(type_of(&checked, "j"), Type::Native(NativeType::F64));
}

#[test]
fn native_rejects_mismatched_native_width() {
    let program = parse("let x: i32 = 1; let y: i64 = x;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable")
            && err.message.contains("i32")
            && err.message.contains("i64"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn native_rejects_number_binding_to_i32() {
    let program = parse("let n: number = 1; let x: i32 = n;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable")
            && err.message.contains("number")
            && err.message.contains("i32"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn native_rejects_i32_to_number() {
    let program = parse("let x: i32 = 1; let n: number = x;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable")
            && err.message.contains("i32")
            && err.message.contains("number"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn native_param_and_return() {
    let program = parse(
        r#"
        function add(a: i32, b: i32): i32 {
          return a + b;
        }
        let s: i32 = add(1, 2);
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "a"), Type::Native(NativeType::I32));
    assert_eq!(type_of(&checked, "b"), Type::Native(NativeType::I32));
    assert_eq!(type_of(&checked, "s"), Type::Native(NativeType::I32));
}

#[test]
fn native_same_type_assign_and_arith() {
    let program = parse(
        r#"
        let x: i32 = 10;
        x = 20;
        let y: i32 = x + 1;
        let z: i32 = x - y;
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "x"), Type::Native(NativeType::I32));
    assert_eq!(type_of(&checked, "y"), Type::Native(NativeType::I32));
    assert_eq!(type_of(&checked, "z"), Type::Native(NativeType::I32));
}

#[test]
fn native_in_alias_and_object() {
    let program = parse(
        r#"
        type Point = { x: i32; y: i32 };
        let x0: i32 = 1;
        let y0: i32 = 2;
        let p: Point = { x: x0, y: y0 };
        let x: i32 = p.x;
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert!(matches!(type_of(&checked, "p"), Type::Shape(_)));
    assert_eq!(type_of(&checked, "x"), Type::Native(NativeType::I32));
}

#[test]
fn native_unary_minus_literal_ok() {
    let program = parse("let x: i32 = -1;").unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "x"), Type::Native(NativeType::I32));
}

// --- T06: dual-worlds boundary (`as`) ---

#[test]
fn dual_boundary_as_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/dual/boundary_as"),
        "missing types/dual/boundary_as fixture, got {ids:?}"
    );
}

#[test]
fn dual_boundary_as_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/dual/boundary_as")
        .expect("types/dual/boundary_as");
    assert!(!fixture.targets.is_empty());
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
fn as_number_to_i32() {
    let program = parse("let n: number = 1; let x: i32 = n as i32;").unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "x"), Type::Native(NativeType::I32));
}

#[test]
fn as_i32_to_number() {
    let program = parse("let x: i32 = 1; let n: number = x as number;").unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "n"), Type::Number);
}

#[test]
fn as_f64_to_number_and_back() {
    let program = parse(
        r#"
        let f: f64 = 1.5;
        let n: number = f as number;
        let g: f64 = n as f64;
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "n"), Type::Number);
    assert_eq!(type_of(&checked, "g"), Type::Native(NativeType::F64));
}

#[test]
fn as_same_type_identity() {
    let program = parse("let x: i32 = 1; let y: i32 = x as i32;").unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "y"), Type::Native(NativeType::I32));
}

#[test]
fn as_rejects_string_to_i32() {
    let program = parse(r#"let s: string = "1"; let x: i32 = s as i32;"#).unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("dual-worlds")
            && err.message.contains("string")
            && err.message.contains("i32"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn as_rejects_i32_to_string() {
    let program = parse(r#"let x: i32 = 1; let s: string = x as string;"#).unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("dual-worlds")
            && err.message.contains("i32")
            && err.message.contains("string"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn as_rejects_i32_to_i64_without_number_hop() {
    let program = parse("let x: i32 = 1; let y: i64 = x as i64;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("dual-worlds")
            && err.message.contains("i32")
            && err.message.contains("i64"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn as_in_call_arg_and_return() {
    let program = parse(
        r#"
        function f(a: i32): number {
          return a as number;
        }
        let n: number = 3;
        let r: number = f(n as i32);
        "#,
    )
    .unwrap();
    let checked = check(program).unwrap();
    assert_eq!(type_of(&checked, "r"), Type::Number);
}

// --- T07.01: annotated call-site argument checking ---

#[test]
fn call_too_few_required_args_errors() {
    let program = parse("function f(a: number, b: number) {} f(1);").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("at least 2"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn call_too_many_args_errors() {
    let program = parse("function f(a: number) {} f(1, 2);").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("at most 1"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn call_arg_type_mismatch_errors() {
    let program = parse(r#"function f(a: number) {} f("x");"#).unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn call_native_param_mismatch_errors() {
    let program = parse(r#"function f(a: i32) {} f("x");"#).unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn call_shape_param_missing_prop_errors() {
    let program = parse("function f(a: { x: number }) {} f({});").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn call_shape_param_wrong_prop_type_errors() {
    let program = parse(r#"function f(a: { x: number }) {} f({ x: "s" });"#).unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn call_default_params_allow_omission() {
    let program = parse("function f(a: number, b: number = 2) {} f(1);").unwrap();
    check(program).unwrap();
}

#[test]
fn call_rest_allows_extra_args() {
    let program = parse("function f(a: number, ...rest) {} f(1, 2, 3);").unwrap();
    check(program).unwrap();
}

#[test]
fn call_unannotated_function_permissive() {
    let program = parse("function f(a) {} f(); f(1, 2, \"x\");").unwrap();
    check(program).unwrap();
}

#[test]
fn call_arrow_binding_arg_mismatch_errors() {
    let program = parse(r#"let f = (a: number) => a; f("x");"#).unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not assignable"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn call_arrow_binding_arity_errors() {
    let program = parse("let f = (a: number, b: number) => a + b; f(1);").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("at least 2"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn call_type_check_uses_annotated_params_only() {
    let program = parse("function f(a: number, b) { return a; } f(1); f(1, \"x\");").unwrap();
    check(program).unwrap();
}

#[test]
fn call_annotated_call_ok_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/annotated_call_ok"),
        "missing types/annotated_call_ok fixture, got {ids:?}"
    );
}

#[test]
fn call_annotated_call_ok_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/annotated_call_ok")
        .expect("types/annotated_call_ok");
    assert!(!fixture.targets.is_empty());
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
fn call_reject_fixtures_run() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    for fixture in fixtures.iter().filter(|f| f.id.starts_with("types/reject")) {
        assert!(
            !fixture.targets.is_empty(),
            "{}: no targets",
            fixture.id
        );
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
}

// --- T07.04: call/`new` of an annotated non-callable value ---

#[test]
fn call_untyped_non_callable_ok_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/call_untyped_non_callable_ok"),
        "missing types/call_untyped_non_callable_ok fixture, got {ids:?}"
    );
}

#[test]
fn call_untyped_non_callable_ok_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/call_untyped_non_callable_ok")
        .expect("types/call_untyped_non_callable_ok");
    assert!(!fixture.targets.is_empty());
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
fn call_annotated_callable_ok_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/call_annotated_callable_ok"),
        "missing types/call_annotated_callable_ok fixture, got {ids:?}"
    );
}

#[test]
fn call_annotated_callable_ok_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/call_annotated_callable_ok")
        .expect("types/call_annotated_callable_ok");
    assert!(!fixture.targets.is_empty());
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
fn call_annotated_number_errors() {
    let program = parse("let x: number = 1; x();").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not callable") && err.message.contains("number"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn call_annotated_shape_errors() {
    let program = parse("let p: { x: number } = { x: 1 }; p();").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not callable") && err.message.contains("x"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn new_annotated_number_errors() {
    let program = parse("let x: number = 1; new x();").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("not constructable") && err.message.contains("number"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn call_untyped_number_ok() {
    let program = parse("let x = 1; if (false) { x(); }").unwrap();
    check(program).expect("untyped inferred number stays permissive when called");
}

#[test]
fn call_annotated_function_ok() {
    let program = parse("function g(a: number): number { return a * 2; } let m: number = g(21);")
        .unwrap();
    check(program).expect("annotated declared function is callable");
}

#[test]
fn call_annotated_any_ok() {
    let program = parse("let f: any = () => 1; let n: number = f();").unwrap();
    check(program).expect("annotated `any` is callable");
}

// --- T07.03: unknown property on annotated shape ---

#[test]
fn unknown_shape_prop_read_errors() {
    let program = parse("let p: { x: number } = { x: 1 }; p.y;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("unknown property") && err.message.contains("y"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn unknown_shape_prop_write_errors() {
    let program = parse("let p: { x: number } = { x: 1 }; p.y = 1;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("unknown property") && err.message.contains("y"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn unknown_shape_prop_update_errors() {
    let program = parse("let p: { x: number } = { x: 1 }; p.y++;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("unknown property") && err.message.contains("y"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn unknown_shape_prop_compound_assign_errors() {
    let program = parse("let p: { x: number } = { x: 1 }; p.y += 1;").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("unknown property") && err.message.contains("y"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn known_shape_prop_access_ok() {
    let program = parse("let p: { x: number } = { x: 1 }; let n: number = p.x; p.x = 2;").unwrap();
    check(program).unwrap();
}

#[test]
fn untyped_object_unknown_prop_stays_dynamic() {
    let program = parse("let p = { x: 1 }; p.y = 2; let n = p.y;").unwrap();
    check(program).unwrap();
}

#[test]
fn unknown_shape_prop_dynamic_ok_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/unknown_shape_prop_dynamic_ok"),
        "missing types/unknown_shape_prop_dynamic_ok fixture, got {ids:?}"
    );
}

#[test]
fn unknown_shape_prop_dynamic_ok_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/unknown_shape_prop_dynamic_ok")
        .expect("types/unknown_shape_prop_dynamic_ok");
    assert!(!fixture.targets.is_empty());
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

// --- T07.05: object literal excess-property check vs annotated shape ---

#[test]
fn excess_prop_annotated_shape_errors() {
    let program = parse("let p: { a: number } = { a: 1, b: 2 };").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("excess property") && err.message.contains("b"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn excess_prop_exact_match_ok() {
    let program = parse("let p: { a: number } = { a: 1 };").unwrap();
    check(program).unwrap();
}

#[test]
fn excess_prop_via_type_alias_errors() {
    let program = parse("type P = { a: number }; let p: P = { a: 1, b: 2 };").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("excess property") && err.message.contains("b"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn excess_prop_string_key_errors() {
    let program = parse("let p: { a: number } = { a: 1, \"b\": 2 };").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("excess property") && err.message.contains("b"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn excess_prop_call_arg_errors() {
    let program = parse("function f(a: { x: number }) {} f({ x: 1, y: 2 });").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("excess property") && err.message.contains("y"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn excess_prop_return_errors() {
    let program = parse("function f(): { x: number } { return { x: 1, y: 2 }; }").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("excess property") && err.message.contains("y"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn excess_prop_assignment_errors() {
    let program = parse("let p: { x: number } = { x: 1 }; p = { x: 1, y: 2 };").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("excess property") && err.message.contains("y"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn excess_prop_nested_literal_errors() {
    let program = parse("let p: { a: { x: number } } = { a: { x: 1, y: 2 } };").unwrap();
    let err = check(program).unwrap_err();
    assert!(
        err.message.contains("excess property") && err.message.contains("y"),
        "unexpected: {}",
        err.message
    );
}

#[test]
fn excess_prop_via_variable_ok() {
    // Structural assignability from a variable is not a fresh literal; stays permissive.
    let program = parse(
        "let full: { x: number; y: number } = { x: 1, y: 2 }; let part: { x: number } = full;",
    )
    .unwrap();
    check(program).unwrap();
}

#[test]
fn excess_prop_inferred_target_permissive() {
    let program = parse("let a = { x: 1 }; let b = { x: 1, y: 2 };").unwrap();
    check(program).unwrap();
}

#[test]
fn excess_prop_ok_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/object_literal_excess_ok"),
        "missing types/object_literal_excess_ok fixture, got {ids:?}"
    );
}

#[test]
fn excess_prop_ok_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/object_literal_excess_ok")
        .expect("types/object_literal_excess_ok");
    assert!(!fixture.targets.is_empty());
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

// --- F06.02: extern "C" signature checking (native ABI types only) ---

#[test]
fn extern_c_ok_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "types/extern_c_ok"),
        "missing types/extern_c_ok fixture, got {ids:?}"
    );
}

#[test]
fn extern_c_ok_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "types/extern_c_ok")
        .expect("types/extern_c_ok");
    assert!(!fixture.targets.is_empty());
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
fn extern_reject_fixtures_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    for want in [
        "types/reject/extern_string_param",
        "types/reject/extern_number_param",
        "types/reject/extern_any_param",
        "types/reject/extern_shape_param",
        "types/reject/extern_unannotated_param",
    ] {
        assert!(
            ids.iter().any(|id| *id == want),
            "missing {want} fixture, got {ids:?}"
        );
    }
}
