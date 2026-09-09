use std::fmt;

use draconic_ast::TypeAnn;

/// Unboxed native / systems types (T05). Outside the JS value heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    /// Unboxed native boolean (N02); distinct from JS `boolean`.
    Bool,
}

impl NativeType {
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "f32" => Self::F32,
            "f64" => Self::F64,
            "bool" => Self::Bool,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::F32 => "f32",
            Self::F64 => "f64",
            Self::Bool => "bool",
        }
    }

    pub fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    pub fn is_bool(self) -> bool {
        matches!(self, Self::Bool)
    }

    /// Integer native types only (`i8`–`i64`, `u8`–`u64`).
    pub fn is_int(self) -> bool {
        !self.is_float() && !self.is_bool()
    }

    pub fn is_signed(self) -> bool {
        matches!(self, Self::I8 | Self::I16 | Self::I32 | Self::I64)
    }

    pub fn bit_width(self) -> u32 {
        match self {
            Self::I8 | Self::U8 | Self::Bool => 8,
            Self::I16 | Self::U16 => 16,
            Self::I32 | Self::U32 | Self::F32 => 32,
            Self::I64 | Self::U64 | Self::F64 => 64,
        }
    }
}

/// TypeScript-inspired types for the minimal Program surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    Number,
    BigInt,
    String,
    Boolean,
    Null,
    /// Callable function value (declaration or expression).
    Function,
    /// Ordinary object value without a known shape.
    Object,
    /// Structural object type; index into the shape table on `CheckedProgram`.
    Shape(u32),
    /// Union type; index into the unions table on `CheckedProgram`.
    Union(u32),
    /// Intersection type; index into the intersections table on `CheckedProgram`.
    Intersection(u32),
    /// Open type parameter while checking a generic body (T04); unique id.
    TypeParam(u32),
    /// Generic function signature; index into the generic_fns table (T04).
    GenericFn(u32),
    /// Unboxed native type (`i32`, `f64`, …); T05.
    Native(NativeType),
    /// Pointer to a native scalar (`*i32`, …); N03.03.
    Ptr(NativeType),
    /// Flexible / unannotated (e.g. `let x;` with no initializer).
    Any,
}

/// Generic function signature stored for call-site instantiation (T04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericFnSig {
    pub type_params: Vec<String>,
    pub param_types: Vec<Option<TypeAnn>>,
    pub return_type: Option<TypeAnn>,
}

/// Property list for a structural object type (`Type::Shape`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectShape {
    pub props: Vec<(String, Type)>,
    /// True when the shape came from an explicit type annotation (`{ x: number }`).
    /// Only strict shapes reject access to unknown properties (T07.03); inferred
    /// object-literal and tuple shapes stay permissive so untyped JS is dynamic.
    pub strict: bool,
}

/// Members of a union type (`Type::Union`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionType {
    pub members: Vec<Type>,
}

/// Members of an intersection type (`Type::Intersection`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntersectionType {
    pub members: Vec<Type>,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Type::Number => "number",
            Type::BigInt => "bigint",
            Type::String => "string",
            Type::Boolean => "boolean",
            Type::Null => "null",
            Type::Function => "function",
            Type::Object => "object",
            Type::Shape(_) => "object",
            Type::Union(_) => "union",
            Type::Intersection(_) => "intersection",
            Type::TypeParam(_) => "type parameter",
            Type::GenericFn(_) => "function",
            Type::Native(n) => n.as_str(),
            Type::Ptr(n) => {
                return write!(f, "*{}", n.as_str());
            }
            Type::Any => "any",
        };
        write!(f, "{s}")
    }
}

pub(crate) fn format_type_full(
    ty: Type,
    shapes: &[ObjectShape],
    unions: &[UnionType],
    intersections: &[IntersectionType],
) -> String {
    match ty {
        Type::Shape(id) => {
            let Some(shape) = shapes.get(id as usize) else {
                return "object".to_string();
            };
            let props: Vec<String> = shape
                .props
                .iter()
                .map(|(n, t)| {
                    format!(
                        "{n}: {}",
                        format_type_full(*t, shapes, unions, intersections)
                    )
                })
                .collect();
            format!("{{ {} }}", props.join("; "))
        }
        Type::Union(id) => {
            let Some(u) = unions.get(id as usize) else {
                return "union".to_string();
            };
            u.members
                .iter()
                .map(|t| format_type_full(*t, shapes, unions, intersections))
                .collect::<Vec<_>>()
                .join(" | ")
        }
        Type::Intersection(id) => {
            let Some(i) = intersections.get(id as usize) else {
                return "intersection".to_string();
            };
            i.members
                .iter()
                .map(|t| format_type_full(*t, shapes, unions, intersections))
                .collect::<Vec<_>>()
                .join(" & ")
        }
        Type::TypeParam(_) => "type parameter".to_string(),
        Type::GenericFn(_) => "function".to_string(),
        Type::Native(n) => n.as_str().to_string(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::Type;
    use crate::{check, CheckedProgram};
    use draconic_ast::{Arg, BinaryOp, Expr, Program, Stmt};
    use draconic_diagnostics::Span;
    use draconic_parser::parse;

    #[test]
    fn check_infers_literal_and_let_types() {
        let program = parse(r#"let n = 1; let s = "hi"; let b = true; let z = null;"#).unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "n"), Type::Number);
        assert_eq!(sym_type(&checked, "s"), Type::String);
        assert_eq!(sym_type(&checked, "b"), Type::Boolean);
        assert_eq!(sym_type(&checked, "z"), Type::Null);
    }

    #[test]
    fn check_infers_binary_number_and_string() {
        let program = parse("let a = 1 + 2; let b = \"a\" + \"b\"; let c = 1 + \"x\";").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Number);
        assert_eq!(sym_type(&checked, "b"), Type::String);
        assert_eq!(sym_type(&checked, "c"), Type::String);
    }

    #[test]
    fn check_propagates_binding_types() {
        let program = parse("let x = 1; let y = x; let z = y + 2;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "x"), Type::Number);
        assert_eq!(sym_type(&checked, "y"), Type::Number);
        assert_eq!(sym_type(&checked, "z"), Type::Number);
    }

    #[test]
    fn check_comparison_is_boolean() {
        let program = parse("let ok = 1 < 2;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "ok"), Type::Boolean);
    }

    #[test]
    fn check_unary_ops() {
        let program = parse("let a = -1; let b = !false; let c = typeof 1;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Number);
        assert_eq!(sym_type(&checked, "b"), Type::Boolean);
        assert_eq!(sym_type(&checked, "c"), Type::String);
    }

    #[test]
    fn check_unary_plus_coerces_to_number() {
        let program =
            parse(r#"let a = +"42"; let b = +true; let c = +null; let d = +"";"#).unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Number);
        assert_eq!(sym_type(&checked, "b"), Type::Number);
        assert_eq!(sym_type(&checked, "c"), Type::Number);
        assert_eq!(sym_type(&checked, "d"), Type::Number);
    }

    #[test]
    fn check_unary_plus_rejects_bigint() {
        let program = parse("let a = +1n;").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("unary") && err.message.contains("bigint"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn check_add_coercion_types() {
        let program = parse(
            r#"let a = "a" + true; let b = true + 1; let c = null + 1; let d = false + true;"#,
        )
        .unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::String);
        assert_eq!(sym_type(&checked, "b"), Type::Number);
        assert_eq!(sym_type(&checked, "c"), Type::Number);
        assert_eq!(sym_type(&checked, "d"), Type::Number);
    }

    #[test]
    fn check_abstract_eq_mixed_types() {
        let program =
            parse(r#"let a = 1 == "1"; let b = null == 0; let c = true != "1";"#).unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Boolean);
        assert_eq!(sym_type(&checked, "b"), Type::Boolean);
        assert_eq!(sym_type(&checked, "c"), Type::Boolean);
    }

    #[test]
    fn check_to_primitive_object_ops() {
        // valueOf/toString run at runtime; static type of object + primitive is Any.
        let program = parse(
            r#"
            let o = { valueOf: function () { return 1; } };
            let a = o + 2;
            let b = "x" + o;
            let c = o == 1;
            let d = o != "1";
            "#,
        )
        .unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Any);
        assert_eq!(sym_type(&checked, "b"), Type::String);
        assert_eq!(sym_type(&checked, "c"), Type::Boolean);
        assert_eq!(sym_type(&checked, "d"), Type::Boolean);
    }

    #[test]
    fn check_uninitialized_let_is_any() {
        let program = parse("let x;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "x"), Type::Any);
    }

    // E19.04: untyped JS operator applicability — ECMA-262 ToNumber/ToPrimitive, not TS-strict.
    #[test]
    fn check_arithmetic_on_string_coerces() {
        let program = parse(r#"let x = "a" - 1; let y = "2" * 3; let z = "8" / "2";"#).unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "x"), Type::Number);
        assert_eq!(sym_type(&checked, "y"), Type::Number);
        assert_eq!(sym_type(&checked, "z"), Type::Number);
    }

    #[test]
    fn check_unary_minus_on_string_coerces() {
        let program = parse(r#"let x = -"a"; let y = ~"1"; let z = -true;"#).unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "x"), Type::Number);
        assert_eq!(sym_type(&checked, "y"), Type::Number);
        assert_eq!(sym_type(&checked, "z"), Type::Number);
    }

    #[test]
    fn check_relational_mixed_primitives() {
        let program =
            parse(r#"let a = "2" < 10; let b = true > 0; let c = null <= 1; let d = "a" < "b";"#)
                .unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Boolean);
        assert_eq!(sym_type(&checked, "b"), Type::Boolean);
        assert_eq!(sym_type(&checked, "c"), Type::Boolean);
        assert_eq!(sym_type(&checked, "d"), Type::Boolean);
    }

    #[test]
    fn check_arithmetic_object_to_primitive() {
        let program = parse(
            r#"
            let o = { valueOf: function () { return 3; } };
            let a = o - 1;
            let b = o * 2;
            let c = o < 10;
            "#,
        )
        .unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Number);
        assert_eq!(sym_type(&checked, "b"), Type::Number);
        assert_eq!(sym_type(&checked, "c"), Type::Boolean);
    }

    // E19.07: mixed BigInt×Number/object/any is ECMA-262-valid; TypeError is runtime.
    #[test]
    fn check_arithmetic_allows_bigint_mixed() {
        let program = parse(
            r#"
            let a = 1n - 1;
            let b = 1 + 1n;
            let c = 1n * true;
            let d = null / 1n;
            let e = 1n + "x";
            let o = { valueOf: function () { return 1n; } };
            let f = o + 1n;
            let g = 1n & 1;
            let h = 1n >>> 1;
            "#,
        )
        .unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Any);
        assert_eq!(sym_type(&checked, "b"), Type::Any);
        assert_eq!(sym_type(&checked, "c"), Type::Any);
        assert_eq!(sym_type(&checked, "d"), Type::Any);
        assert_eq!(sym_type(&checked, "e"), Type::String);
        assert_eq!(sym_type(&checked, "f"), Type::Any);
        assert_eq!(sym_type(&checked, "g"), Type::Any);
        assert_eq!(sym_type(&checked, "h"), Type::Any);
    }

    #[test]
    fn check_arithmetic_same_type_bigint_still_bigint() {
        let program = parse("let a = 1n + 2n; let b = 3n * 4n; let c = 5n << 1n;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::BigInt);
        assert_eq!(sym_type(&checked, "b"), Type::BigInt);
        assert_eq!(sym_type(&checked, "c"), Type::BigInt);
    }

    // E19.59: call/`new` on boolean/number/string/null — TypeError is runtime, not compile.
    #[test]
    fn check_call_on_primitives_typechecks() {
        let program = parse(
            r#"
            let n = 1; try { n(); } catch (e) {}
            let b = true; try { b(); } catch (e) {}
            let s = "x"; try { s(); } catch (e) {}
            let z = null; try { z(); } catch (e) {}
            try { (1)(); } catch (e) {}
            try { (true)(); } catch (e) {}
            try { ("x")(); } catch (e) {}
            try { (null)(); } catch (e) {}
            "#,
        )
        .unwrap();
        check(program).expect("call on primitives should typecheck; [[Call]] is runtime");
    }

    #[test]
    fn check_new_on_primitives_typechecks() {
        let program = parse(
            r#"
            let n = 1; try { new n(); } catch (e) {}
            let b = true; try { new b(); } catch (e) {}
            let s = "x"; try { new s(); } catch (e) {}
            let z = null; try { new z(); } catch (e) {}
            try { new (1)(); } catch (e) {}
            try { new (true)(); } catch (e) {}
            try { new ("x")(); } catch (e) {}
            try { new (null)(); } catch (e) {}
            "#,
        )
        .unwrap();
        check(program).expect("new on primitives should typecheck; [[Construct]] is runtime");
    }

    // E19.13: ++/-- and call on ToPrimitive / object values — runtime ToNumber/[[Call]], not compile reject.
    #[test]
    fn check_update_on_object_to_primitive() {
        let program = parse(
            r#"
            let o = { valueOf: function () { return 1; } };
            o++;
            ++o;
            let f = function () { return 1; };
            f++;
            "#,
        )
        .unwrap();
        check(program).expect("update on object/function should typecheck (ToNumber)");
    }

    #[test]
    fn check_update_on_member_to_primitive() {
        let program = parse(
            r#"
            let o = { x: 1, y: true };
            o.x++;
            ++o["y"];
            let a = [0];
            a[0]++;
            "#,
        )
        .unwrap();
        check(program).expect("update on property should typecheck");
    }

    #[test]
    fn check_call_on_object_typechecks() {
        let program = parse(
            r#"
            let o = {};
            try { o(); } catch (e) {}
            try { Math(); } catch (e) {}
            try { new Boolean(true)(); } catch (e) {}
            let b = new Boolean(true);
            try { b(); } catch (e) {}
            "#,
        )
        .unwrap();
        check(program).expect("calling object should typecheck; [[Call]] is runtime");
    }

    #[test]
    fn check_records_expr_types() {
        let program = parse("let x = 1 + 2;").unwrap();
        let checked = check(program).unwrap();
        let add_span = find_binary_span(&checked.bound.program, BinaryOp::Add);
        assert_eq!(checked.type_of_expr(add_span), Some(Type::Number));
    }

    fn sym_type(checked: &CheckedProgram, name: &str) -> Type {
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("no symbol `{name}`"));
        checked.type_of_symbol(sym.id)
    }

    fn find_binary_span(program: &Program, op: BinaryOp) -> Span {
        fn walk(expr: &Expr, op: BinaryOp, out: &mut Option<Span>) {
            if out.is_some() {
                return;
            }
            match expr {
                Expr::Binary {
                    left,
                    op: bop,
                    right,
                    span,
                } => {
                    if *bop == op {
                        *out = Some(*span);
                        return;
                    }
                    walk(left, op, out);
                    walk(right, op, out);
                }
                Expr::Unary { arg, .. }
                | Expr::Paren { expr: arg, .. }
                | Expr::Update { arg, .. }
                | Expr::As { expr: arg, .. } => walk(arg, op, out),
                Expr::Conditional {
                    test,
                    consequent,
                    alternate,
                    ..
                } => {
                    walk(test, op, out);
                    walk(consequent, op, out);
                    walk(alternate, op, out);
                }
                Expr::Assign { target, value, .. } => {
                    walk(target, op, out);
                    walk(value, op, out);
                }
                Expr::Call { callee, args, .. } => {
                    walk(callee, op, out);
                    for a in args {
                        match a {
                            Arg::Expr(expr) | Arg::Spread(expr) => walk(expr, op, out),
                        }
                    }
                }
                _ => {}
            }
        }
        fn walk_stmt(stmt: &Stmt, op: BinaryOp, out: &mut Option<Span>) {
            if out.is_some() {
                return;
            }
            match stmt {
                Stmt::Expression { expr, .. } => walk(expr, op, out),
                Stmt::Let {
                    init: Some(init), ..
                } => walk(init, op, out),
                Stmt::Block { body, .. } => {
                    for s in body {
                        walk_stmt(s, op, out);
                    }
                }
                Stmt::If {
                    test,
                    consequent,
                    alternate,
                    ..
                } => {
                    walk(test, op, out);
                    walk_stmt(consequent, op, out);
                    if let Some(alt) = alternate {
                        walk_stmt(alt, op, out);
                    }
                }
                Stmt::While { test, body, .. } => {
                    walk(test, op, out);
                    walk_stmt(body, op, out);
                }
                Stmt::DoWhile { body, test, .. } => {
                    walk_stmt(body, op, out);
                    walk(test, op, out);
                }
                Stmt::For {
                    init,
                    test,
                    update,
                    body,
                    ..
                } => {
                    if let Some(init) = init {
                        walk_stmt(init, op, out);
                    }
                    if let Some(t) = test {
                        walk(t, op, out);
                    }
                    if let Some(u) = update {
                        walk(u, op, out);
                    }
                    walk_stmt(body, op, out);
                }
                Stmt::ForIn {
                    left, right, body, ..
                }
                | Stmt::ForOf {
                    left, right, body, ..
                } => {
                    walk_stmt(left, op, out);
                    walk(right, op, out);
                    walk_stmt(body, op, out);
                }
                _ => {}
            }
        }

        let mut found = None;
        for stmt in &program.body {
            walk_stmt(stmt, op, &mut found);
        }
        found.expect("binary op not found")
    }

    // --- T07.04: call/`new` of an annotated non-callable value ---

    #[test]
    fn check_annotated_number_call_errors() {
        let program = parse("let x: number = 1; x();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("number"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_string_call_errors() {
        let program = parse(r#"let s: string = "a"; s();"#).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("string"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_boolean_call_errors() {
        let program = parse("let b: boolean = true; b();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("boolean"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_bigint_call_errors() {
        let program = parse("let x: bigint = 1n; x();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("bigint"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_shape_call_errors() {
        let program = parse("let p: { x: number } = { x: 1 }; p();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("{ x: number }"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_param_call_errors() {
        let program = parse("function g(x: number) { x(); }").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("number"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_alias_call_errors() {
        let program = parse("type Num = number; let x: Num = 1; x();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("number"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_number_new_errors() {
        let program = parse("let x: number = 1; new x();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not constructable") && err.message.contains("number"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_shape_new_errors() {
        let program = parse("let p: { x: number } = { x: 1 }; new p();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not constructable") && err.message.contains("{ x: number }"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_parenthesized_annotated_call_errors() {
        let program = parse("let x: number = 1; (x)();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_any_call_ok() {
        let program = parse("let x: any = 1; x();").unwrap();
        check(program).expect("`any` stays permissive when called");
    }

    #[test]
    fn check_annotated_callable_declared_fn_ok() {
        let program =
            parse("function g(a: number): number { return a * 2; } let m: number = g(21);")
                .unwrap();
        check(program).expect("annotated declared function is callable");
    }

    #[test]
    fn check_untyped_non_callable_call_ok() {
        let program = parse("let x = 1; x(); let p = { a: 1 }; p();").unwrap();
        check(program).expect("untyped JS stays permissive when calling non-callables");
    }

    #[test]
    fn check_inferred_shape_call_ok() {
        let program = parse("let p = { a: 1 }; p();").unwrap();
        check(program).expect("inferred object-literal shape stays permissive when called");
    }
}
