use super::*;
use draconic_ast::{
    Arg, ArrayElement, ArrayPatternElement, BindingKind, BindingPattern, ClassElement, Expr,
    ObjectKey, ObjectPatternProp, ObjectProp, Program, Stmt,
};
use draconic_diagnostics::Span;
use draconic_parser::parse;

fn user_symbol<'a>(bound: &'a BoundProgram, name: &str) -> &'a Symbol {
    bound
        .symbols()
        .iter()
        .find(|s| s.name == name && s.span != Span::dummy())
        .unwrap_or_else(|| panic!("no user symbol `{name}`"))
}

#[test]
fn bind_resolves_reference_to_let() {
    let program = parse("let x = 1; x;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "x");
    let id = bound.resolve(use_span).expect("x should resolve");
    assert_eq!(bound.symbol(id).name, "x");
}

#[test]
fn bind_resolves_ident_in_initializer() {
    let program = parse("let x = 1; let y = x + 2;").unwrap();
    let bound = bind(program).unwrap();
    assert!(user_symbol(&bound, "x").name == "x");
    assert!(user_symbol(&bound, "y").name == "y");
    let use_span = find_ident_use(&bound.program, "x");
    let id = bound.resolve(use_span).expect("x in init should resolve");
    assert_eq!(bound.symbol(id).name, "x");
}

#[test]
fn bind_resolves_global_math() {
    let program = parse("Math.abs(-1);").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Math");
    let id = bound.resolve(use_span).expect("Math should resolve");
    assert_eq!(bound.symbol(id).name, "Math");
    assert_eq!(bound.symbol(id).kind, BindingKind::Const);
}

#[test]
fn bind_let_math_shadows_builtin() {
    let program = parse("let Math = 1; Math;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Math");
    let id = bound.resolve(use_span).expect("Math should resolve");
    assert_eq!(bound.symbol(id).name, "Math");
}

#[test]
fn bind_resolves_global_number_nan_infinity() {
    let program = parse("Number.isNaN(NaN); Infinity;").unwrap();
    let bound = bind(program).unwrap();
    for name in ["Number", "NaN", "Infinity"] {
        let use_span = find_ident_use(&bound.program, name);
        let id = bound.resolve(use_span).expect("should resolve");
        assert_eq!(bound.symbol(id).name, name);
    }
}

#[test]
fn bind_let_number_shadows_builtin() {
    let program = parse("let Number = 1; Number;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Number");
    let id = bound.resolve(use_span).expect("Number should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_symbol() {
    let program = parse("Symbol(); Symbol.for(\"x\");").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Symbol");
    let id = bound.resolve(use_span).expect("Symbol should resolve");
    assert_eq!(bound.symbol(id).name, "Symbol");
    assert_eq!(bound.symbol(id).kind, BindingKind::Const);
}

#[test]
fn bind_let_symbol_shadows_builtin() {
    let program = parse("let Symbol = 1; Symbol;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Symbol");
    let id = bound.resolve(use_span).expect("Symbol should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_promise() {
    let program = parse("new Promise(function (r) { r(1); });").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Promise");
    let id = bound.resolve(use_span).expect("Promise should resolve");
    assert_eq!(bound.symbol(id).name, "Promise");
    assert_eq!(bound.symbol(id).kind, BindingKind::Const);
}

#[test]
fn bind_let_promise_shadows_builtin() {
    let program = parse("let Promise = 1; Promise;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Promise");
    let id = bound.resolve(use_span).expect("Promise should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_proxy() {
    let program = parse("new Proxy({}, {});").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Proxy");
    let id = bound.resolve(use_span).expect("Proxy should resolve");
    assert_eq!(bound.symbol(id).name, "Proxy");
    assert_eq!(bound.symbol(id).kind, BindingKind::Const);
}

#[test]
fn bind_let_proxy_shadows_builtin() {
    let program = parse("let Proxy = 1; Proxy;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Proxy");
    let id = bound.resolve(use_span).expect("Proxy should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_reflect() {
    let program = parse("Reflect.get({}, \"a\");").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Reflect");
    let id = bound.resolve(use_span).expect("Reflect should resolve");
    assert_eq!(bound.symbol(id).name, "Reflect");
    assert_eq!(bound.symbol(id).kind, BindingKind::Const);
}

#[test]
fn bind_let_reflect_shadows_builtin() {
    let program = parse("let Reflect = 1; Reflect;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Reflect");
    let id = bound.resolve(use_span).expect("Reflect should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_undefined_and_global_this() {
    let program = parse("undefined; globalThis;").unwrap();
    let bound = bind(program).unwrap();
    for name in ["undefined", "globalThis"] {
        let use_span = find_ident_use(&bound.program, name);
        let id = bound.resolve(use_span).expect("should resolve");
        assert_eq!(bound.symbol(id).name, name);
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }
}

#[test]
fn bind_resolves_fundamental_constructors() {
    let program = parse("Object; Function; Array; String; Boolean;").unwrap();
    let bound = bind(program).unwrap();
    for name in ["Object", "Function", "Array", "String", "Boolean"] {
        let use_span = find_ident_use(&bound.program, name);
        let id = bound.resolve(use_span).expect("should resolve");
        assert_eq!(bound.symbol(id).name, name);
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }
}

#[test]
fn bind_let_object_shadows_builtin() {
    let program = parse("let Object = 1; Object;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Object");
    let id = bound.resolve(use_span).expect("Object should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_error_constructors() {
    let program = parse(
        "Error; TypeError; RangeError; ReferenceError; SyntaxError; URIError; EvalError; AggregateError;",
    )
    .unwrap();
    let bound = bind(program).unwrap();
    for name in [
        "Error",
        "TypeError",
        "RangeError",
        "ReferenceError",
        "SyntaxError",
        "URIError",
        "EvalError",
        "AggregateError",
    ] {
        let use_span = find_ident_use(&bound.program, name);
        let id = bound.resolve(use_span).expect("should resolve");
        assert_eq!(bound.symbol(id).name, name);
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }
}

#[test]
fn bind_let_error_shadows_builtin() {
    let program = parse("let Error = 1; Error;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Error");
    let id = bound.resolve(use_span).expect("Error should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_functions() {
    let program = parse("parseInt; parseFloat; isNaN; isFinite;").unwrap();
    let bound = bind(program).unwrap();
    for name in ["parseInt", "parseFloat", "isNaN", "isFinite"] {
        let use_span = find_ident_use(&bound.program, name);
        let id = bound.resolve(use_span).expect("should resolve");
        assert_eq!(bound.symbol(id).name, name);
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }
}

#[test]
fn bind_let_parse_int_shadows_builtin() {
    let program = parse("let parseInt = 1; parseInt;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "parseInt");
    let id = bound.resolve(use_span).expect("parseInt should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_uri_functions() {
    let program = parse("encodeURI; decodeURI; encodeURIComponent; decodeURIComponent;").unwrap();
    let bound = bind(program).unwrap();
    for name in [
        "encodeURI",
        "decodeURI",
        "encodeURIComponent",
        "decodeURIComponent",
    ] {
        let use_span = find_ident_use(&bound.program, name);
        let id = bound.resolve(use_span).expect("should resolve");
        assert_eq!(bound.symbol(id).name, name);
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }
}

#[test]
fn bind_let_encode_uri_shadows_builtin() {
    let program = parse("let encodeURI = 1; encodeURI;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "encodeURI");
    let id = bound.resolve(use_span).expect("encodeURI should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_escape_unescape() {
    let program = parse("escape; unescape;").unwrap();
    let bound = bind(program).unwrap();
    for name in ["escape", "unescape"] {
        let use_span = find_ident_use(&bound.program, name);
        let id = bound.resolve(use_span).expect("should resolve");
        assert_eq!(bound.symbol(id).name, name);
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }
}

#[test]
fn bind_let_escape_shadows_builtin() {
    let program = parse("let escape = 1; escape;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "escape");
    let id = bound.resolve(use_span).expect("escape should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_json() {
    let program = parse("JSON;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "JSON");
    let id = bound.resolve(use_span).expect("JSON should resolve");
    assert_eq!(bound.symbol(id).name, "JSON");
    assert_eq!(bound.symbol(id).kind, BindingKind::Const);
}

#[test]
fn bind_let_json_shadows_builtin() {
    let program = parse("let JSON = 1; JSON;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "JSON");
    let id = bound.resolve(use_span).expect("JSON should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_date() {
    let program = parse("Date;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Date");
    let id = bound.resolve(use_span).expect("Date should resolve");
    assert_eq!(bound.symbol(id).name, "Date");
    assert_eq!(bound.symbol(id).kind, BindingKind::Const);
}

#[test]
fn bind_let_date_shadows_builtin() {
    let program = parse("let Date = 1; Date;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Date");
    let id = bound.resolve(use_span).expect("Date should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_regexp() {
    let program = parse("RegExp;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "RegExp");
    let id = bound.resolve(use_span).expect("RegExp should resolve");
    assert_eq!(bound.symbol(id).name, "RegExp");
    assert_eq!(bound.symbol(id).kind, BindingKind::Const);
}

#[test]
fn bind_let_regexp_shadows_builtin() {
    let program = parse("let RegExp = 1; RegExp;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "RegExp");
    let id = bound.resolve(use_span).expect("RegExp should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_map_and_set() {
    let program = parse("Map; Set;").unwrap();
    let bound = bind(program).unwrap();
    for name in ["Map", "Set"] {
        let use_span = find_ident_use(&bound.program, name);
        let id = bound.resolve(use_span).expect(name);
        assert_eq!(bound.symbol(id).name, name);
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }
}

#[test]
fn bind_let_map_shadows_builtin() {
    let program = parse("let Map = 1; Map;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "Map");
    let id = bound.resolve(use_span).expect("Map should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_weak_map_and_weak_set() {
    let program = parse("WeakMap; WeakSet;").unwrap();
    let bound = bind(program).unwrap();
    for name in ["WeakMap", "WeakSet"] {
        let use_span = find_ident_use(&bound.program, name);
        let id = bound.resolve(use_span).expect(name);
        assert_eq!(bound.symbol(id).name, name);
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }
}

#[test]
fn bind_let_weak_map_shadows_builtin() {
    let program = parse("let WeakMap = 1; WeakMap;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "WeakMap");
    let id = bound.resolve(use_span).expect("WeakMap should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_global_arraybuffer_dataview_typedarrays() {
    let program = parse("ArrayBuffer; DataView; Uint8Array; Int32Array; Float64Array;").unwrap();
    let bound = bind(program).unwrap();
    for name in [
        "ArrayBuffer",
        "DataView",
        "Uint8Array",
        "Int32Array",
        "Float64Array",
    ] {
        let use_span = find_ident_use(&bound.program, name);
        let id = bound.resolve(use_span).expect(name);
        assert_eq!(bound.symbol(id).name, name);
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }
}

#[test]
fn bind_let_arraybuffer_shadows_builtin() {
    let program = parse("let ArrayBuffer = 1; ArrayBuffer;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "ArrayBuffer");
    let id = bound.resolve(use_span).expect("ArrayBuffer should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_resolves_eval() {
    let program = parse("eval;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "eval");
    let id = bound.resolve(use_span).expect("eval should resolve");
    assert_eq!(bound.symbol(id).name, "eval");
    assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    assert_eq!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_let_eval_shadows_builtin() {
    let program = parse("let eval = 1; eval;").unwrap();
    let bound = bind(program).unwrap();
    let use_span = find_ident_use(&bound.program, "eval");
    let id = bound.resolve(use_span).expect("eval should resolve");
    assert_ne!(bound.symbol(id).span, Span::dummy());
}

#[test]
fn bind_free_identifier_ok_global_object_ref() {
    // E19.05: free idents are runtime global/unresolvable refs, not bind errors.
    let program = parse("y;").unwrap();
    let bound = bind(program).expect("free ident binds");
    let y_span = find_ident_use(&bound.program, "y");
    assert!(
        bound.resolve(y_span).is_none(),
        "free y must stay unresolved for IdentName emit"
    );
}

#[test]
fn bind_free_assign_and_typeof_ok() {
    let src = "x = 1; typeof z;";
    let bound = bind(parse(src).unwrap()).expect("free assign/typeof bind");
    check(parse(src).unwrap()).expect("free assign/typeof check");
    let x_span = find_ident_use(&bound.program, "x");
    let z_span = find_ident_use(&bound.program, "z");
    assert!(bound.resolve(x_span).is_none());
    assert!(bound.resolve(z_span).is_none());
}

/// First non-declaration Ident use of `name` (expression reference).
pub(crate) fn find_ident_use(program: &Program, name: &str) -> Span {
    fn walk_object_key(key: &ObjectKey, name: &str, out: &mut Option<Span>) {
        if let ObjectKey::Computed(expr) = key {
            walk_expr(expr, name, out);
        }
    }
    fn walk_expr(expr: &Expr, name: &str, out: &mut Option<Span>) {
        if out.is_some() {
            return;
        }
        match expr {
            Expr::Ident(id) if id.name == name => *out = Some(id.span),
            Expr::Ident(_)
            | Expr::Number(_)
            | Expr::BigInt(_)
            | Expr::String(_)
            | Expr::RegExp { .. }
            | Expr::Boolean { .. }
            | Expr::Null { .. }
            | Expr::This { .. }
            | Expr::Super { .. }
            | Expr::NewTarget { .. }
            | Expr::ImportMeta { .. } => {}
            Expr::ImportCall {
                source, options, ..
            } => {
                walk_expr(source, name, out);
                if let Some(opts) = options {
                    walk_expr(opts, name, out);
                }
            }
            Expr::TemplateLiteral { expressions, .. } => {
                for e in expressions {
                    walk_expr(e, name, out);
                }
            }
            Expr::TaggedTemplate {
                tag, expressions, ..
            } => {
                walk_expr(tag, name, out);
                for e in expressions {
                    walk_expr(e, name, out);
                }
            }
            Expr::Unary { arg, .. }
            | Expr::Paren { expr: arg, .. }
            | Expr::As { expr: arg, .. } => walk_expr(arg, name, out),
            Expr::Binary { left, right, .. } => {
                walk_expr(left, name, out);
                walk_expr(right, name, out);
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                walk_expr(test, name, out);
                walk_expr(consequent, name, out);
                walk_expr(alternate, name, out);
            }
            Expr::Assign { target, value, .. } => {
                walk_expr(target, name, out);
                walk_expr(value, name, out);
            }
            Expr::Update { arg, .. } => walk_expr(arg, name, out),
            Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
                walk_expr(callee, name, out);
                for a in args {
                    match a {
                        Arg::Expr(expr) | Arg::Spread(expr) => walk_expr(expr, name, out),
                    }
                }
            }
            Expr::ObjectExpression { properties, .. } => {
                for prop in properties {
                    match prop {
                        ObjectProp::Property { key, value, .. } => {
                            if let ObjectKey::Computed(expr) = key {
                                walk_expr(expr, name, out);
                            }
                            walk_expr(value, name, out);
                        }
                        ObjectProp::Accessor { key, body, .. } => {
                            if let ObjectKey::Computed(expr) = key {
                                walk_expr(expr, name, out);
                            }
                            walk_stmt(body, name, out);
                        }
                        ObjectProp::Spread { expr, .. } => walk_expr(expr, name, out),
                    }
                }
            }
            Expr::ArrayExpression { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayElement::Expr(expr) | ArrayElement::Spread(expr) => {
                            walk_expr(expr, name, out);
                        }
                        ArrayElement::Elision => {}
                    }
                }
            }
            Expr::MemberExpression {
                object,
                property,
                computed,
                ..
            } => {
                walk_expr(object, name, out);
                if *computed {
                    walk_expr(property, name, out);
                }
            }
            Expr::PrivateIn { object, .. } => walk_expr(object, name, out),
            // Function/class bodies walked via declaration paths when needed.
            Expr::FunctionExpression { .. }
            | Expr::ClassExpression { .. }
            | Expr::ArrowFunction { .. } => {}
            Expr::ArrayPattern { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Pattern {
                            binding: BindingPattern::Ident(id),
                            default,
                        } if id.name == name => {
                            *out = Some(id.span);
                            if let Some(def) = default {
                                walk_expr(def, name, out);
                            }
                        }
                        ArrayPatternElement::Pattern {
                            binding: BindingPattern::Ident(_),
                            default,
                        } => {
                            if let Some(def) = default {
                                walk_expr(def, name, out);
                            }
                        }
                        ArrayPatternElement::Pattern {
                            binding:
                                BindingPattern::Array {
                                    elements: nested, ..
                                },
                            default,
                        } => {
                            walk_expr(
                                &Expr::ArrayPattern {
                                    elements: nested.clone(),
                                    span: Span::dummy(),
                                },
                                name,
                                out,
                            );
                            if let Some(def) = default {
                                walk_expr(def, name, out);
                            }
                        }
                        ArrayPatternElement::Pattern {
                            binding: BindingPattern::Object { properties, .. },
                            default,
                        } => {
                            walk_expr(
                                &Expr::ObjectPattern {
                                    properties: properties.clone(),
                                    span: Span::dummy(),
                                },
                                name,
                                out,
                            );
                            if let Some(def) = default {
                                walk_expr(def, name, out);
                            }
                        }
                        ArrayPatternElement::Elision => {}
                        ArrayPatternElement::Pattern {
                            binding: BindingPattern::Member(expr),
                            default,
                        } => {
                            walk_expr(expr, name, out);
                            if let Some(def) = default {
                                walk_expr(def, name, out);
                            }
                        }
                        ArrayPatternElement::Rest(BindingPattern::Ident(id)) if id.name == name => {
                            *out = Some(id.span);
                        }
                        ArrayPatternElement::Rest(BindingPattern::Array { elements, .. }) => {
                            walk_expr(
                                &Expr::ArrayPattern {
                                    elements: elements.clone(),
                                    span: Span::dummy(),
                                },
                                name,
                                out,
                            );
                        }
                        ArrayPatternElement::Rest(BindingPattern::Object {
                            properties, ..
                        }) => {
                            walk_expr(
                                &Expr::ObjectPattern {
                                    properties: properties.clone(),
                                    span: Span::dummy(),
                                },
                                name,
                                out,
                            );
                        }
                        ArrayPatternElement::Rest(BindingPattern::Member(expr)) => {
                            walk_expr(expr, name, out);
                        }
                        ArrayPatternElement::Rest(_) => {}
                    }
                }
            }
            Expr::ObjectPattern { properties, .. } => {
                for p in properties {
                    match p {
                        ObjectPatternProp::Prop {
                            key,
                            binding: BindingPattern::Ident(id),
                            default,
                            ..
                        } if id.name == name => {
                            *out = Some(id.span);
                            if let ObjectKey::Computed(e) = key {
                                walk_expr(e, name, out);
                            }
                            if let Some(def) = default {
                                walk_expr(def, name, out);
                            }
                        }
                        ObjectPatternProp::Prop {
                            key,
                            binding: BindingPattern::Ident(_),
                            default,
                            ..
                        } => {
                            if let ObjectKey::Computed(e) = key {
                                walk_expr(e, name, out);
                            }
                            if let Some(def) = default {
                                walk_expr(def, name, out);
                            }
                        }
                        ObjectPatternProp::Prop {
                            key,
                            binding: BindingPattern::Array { elements, .. },
                            default,
                            ..
                        } => {
                            if let ObjectKey::Computed(e) = key {
                                walk_expr(e, name, out);
                            }
                            walk_expr(
                                &Expr::ArrayPattern {
                                    elements: elements.clone(),
                                    span: Span::dummy(),
                                },
                                name,
                                out,
                            );
                            if let Some(def) = default {
                                walk_expr(def, name, out);
                            }
                        }
                        ObjectPatternProp::Prop {
                            key,
                            binding:
                                BindingPattern::Object {
                                    properties: nested, ..
                                },
                            default,
                            ..
                        } => {
                            if let ObjectKey::Computed(e) = key {
                                walk_expr(e, name, out);
                            }
                            walk_expr(
                                &Expr::ObjectPattern {
                                    properties: nested.clone(),
                                    span: Span::dummy(),
                                },
                                name,
                                out,
                            );
                            if let Some(def) = default {
                                walk_expr(def, name, out);
                            }
                        }
                        ObjectPatternProp::Prop {
                            key,
                            binding: BindingPattern::Member(expr),
                            default,
                            ..
                        } => {
                            if let ObjectKey::Computed(e) = key {
                                walk_expr(e, name, out);
                            }
                            walk_expr(expr, name, out);
                            if let Some(def) = default {
                                walk_expr(def, name, out);
                            }
                        }
                        ObjectPatternProp::Rest(BindingPattern::Ident(id)) if id.name == name => {
                            *out = Some(id.span);
                        }
                        ObjectPatternProp::Rest(BindingPattern::Array { elements, .. }) => {
                            walk_expr(
                                &Expr::ArrayPattern {
                                    elements: elements.clone(),
                                    span: Span::dummy(),
                                },
                                name,
                                out,
                            );
                        }
                        ObjectPatternProp::Rest(BindingPattern::Object {
                            properties: nested,
                            ..
                        }) => {
                            walk_expr(
                                &Expr::ObjectPattern {
                                    properties: nested.clone(),
                                    span: Span::dummy(),
                                },
                                name,
                                out,
                            );
                        }
                        ObjectPatternProp::Rest(BindingPattern::Member(expr)) => {
                            walk_expr(expr, name, out);
                        }
                        ObjectPatternProp::Rest(_) => {}
                    }
                }
            }
        }
    }

    fn walk_stmt(stmt: &Stmt, name: &str, out: &mut Option<Span>) {
        if out.is_some() {
            return;
        }
        match stmt {
            Stmt::Expression { expr, .. } => walk_expr(expr, name, out),
            Stmt::Let {
                init: Some(init), ..
            } => walk_expr(init, name, out),
            Stmt::Let { init: None, .. }
            | Stmt::Empty { .. }
            | Stmt::TypeAlias { .. }
            | Stmt::ExternFunctionDeclaration { .. } => {}
            Stmt::Block { body, .. } => {
                for s in body {
                    walk_stmt(s, name, out);
                }
            }
            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => {
                walk_expr(test, name, out);
                walk_stmt(consequent, name, out);
                if let Some(alt) = alternate {
                    walk_stmt(alt, name, out);
                }
            }
            Stmt::While { test, body, .. } => {
                walk_expr(test, name, out);
                walk_stmt(body, name, out);
            }
            Stmt::DoWhile { body, test, .. } => {
                walk_stmt(body, name, out);
                walk_expr(test, name, out);
            }
            Stmt::For {
                init,
                test,
                update,
                body,
                ..
            } => {
                if let Some(init) = init {
                    walk_stmt(init, name, out);
                }
                if let Some(t) = test {
                    walk_expr(t, name, out);
                }
                if let Some(u) = update {
                    walk_expr(u, name, out);
                }
                walk_stmt(body, name, out);
            }
            Stmt::ForIn {
                left, right, body, ..
            }
            | Stmt::ForOf {
                left, right, body, ..
            } => {
                walk_stmt(left, name, out);
                walk_expr(right, name, out);
                walk_stmt(body, name, out);
            }
            Stmt::Break { .. } | Stmt::Continue { .. } => {}
            Stmt::Labeled { body, .. } => walk_stmt(body, name, out),
            Stmt::Switch {
                discriminant,
                cases,
                ..
            } => {
                walk_expr(discriminant, name, out);
                for case in cases {
                    if let Some(test) = &case.test {
                        walk_expr(test, name, out);
                    }
                    for s in &case.body {
                        walk_stmt(s, name, out);
                    }
                }
            }
            Stmt::FunctionDeclaration { params, body, .. } => {
                let _ = params;
                walk_stmt(body, name, out);
            }
            Stmt::ClassDeclaration {
                super_class, body, ..
            } => {
                if let Some(sc) = super_class {
                    walk_expr(sc, name, out);
                }
                for el in body {
                    match el {
                        ClassElement::Constructor { body, .. }
                        | ClassElement::StaticBlock { body, .. } => {
                            walk_stmt(body, name, out);
                        }
                        ClassElement::Method { key, body, .. }
                        | ClassElement::Accessor { key, body, .. } => {
                            walk_object_key(key, name, out);
                            walk_stmt(body, name, out);
                        }
                        ClassElement::Field { key, value, .. } => {
                            walk_object_key(key, name, out);
                            if let Some(v) = value {
                                walk_expr(v, name, out);
                            }
                        }
                    }
                }
            }
            Stmt::Return {
                argument: Some(arg),
                ..
            } => walk_expr(arg, name, out),
            Stmt::Return { argument: None, .. } => {}
            Stmt::Throw { argument, .. } => walk_expr(argument, name, out),
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                walk_stmt(block, name, out);
                if let Some(handler) = handler {
                    walk_stmt(handler, name, out);
                }
                if let Some(finalizer) = finalizer {
                    walk_stmt(finalizer, name, out);
                }
            }
            Stmt::With { object, body, .. } => {
                walk_expr(object, name, out);
                walk_stmt(body, name, out);
            }
            Stmt::ImportDeclaration { .. }
            | Stmt::ExportNamedDeclaration { .. }
            | Stmt::ExportDefaultDeclaration { .. }
            | Stmt::ExportAllDeclaration { .. } => {}
        }
    }

    let mut found = None;
    for stmt in &program.body {
        walk_stmt(stmt, name, &mut found);
        if found.is_some() {
            break;
        }
    }
    found.unwrap_or_else(|| panic!("no ident use of `{name}` found"))
}
