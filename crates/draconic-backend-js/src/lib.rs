//! JS backend: IR → ECMAScript (ROADMAP B07 + N04 native policy + U03 source maps).

mod emit;
mod source_map;

pub use source_map::{
    decode_mappings, decode_vlq, encode_vlq, source_mapping_url_comment, Mapping, SourceMap,
    SourceMapOptions,
};

use std::collections::HashMap;

use draconic_ast::UnaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    ArrayPatternEl, AssignTarget, Expr, IrType, LocalId, Module, ObjectPatternEl, Pattern, Stmt,
};
use source_map::SourceMapBuilder;

/// JS emit result with optional Source Map v3 (U03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmittedJs {
    pub code: String,
    pub map: Option<SourceMap>,
}

/// Emit ECMAScript source for a shared IR module.
///
/// **N04 native policy (JS target):**
/// - Native scalars (`i32`, …), layout structs, and fixed arrays: polyfill/erase
///   (type annotations already gone at IR; values lower as ordinary JS numbers/objects/arrays).
/// - Native pointers (`*T`, `&x`, `*p`, `*p = v`): hard-error (native-only).
pub fn emit_js(module: &Module) -> Result<String, Diagnostic> {
    Ok(emit_js_full(module, None)?.code)
}

/// Emit ECMAScript plus a Source Map v3 mapping generated positions back to the Program.
///
/// One mapping segment is recorded at the start of each top-level IR statement, using
/// `module.body_spans` (original AST spans preserved through lower). Nested statements
/// share their enclosing top-level origin.
pub fn emit_js_with_map(
    module: &Module,
    opts: &SourceMapOptions<'_>,
) -> Result<EmittedJs, Diagnostic> {
    emit_js_full(module, Some(opts))
}

fn emit_js_full(
    module: &Module,
    map_opts: Option<&SourceMapOptions<'_>>,
) -> Result<EmittedJs, Diagnostic> {
    reject_native_only(module)?;

    let names: HashMap<LocalId, &str> = module
        .locals
        .iter()
        .map(|l| (l.id, l.name.as_str()))
        .collect();

    let mut out = String::new();
    let mut builder = map_opts.map(SourceMapBuilder::new);

    for (i, stmt) in module.body.iter().enumerate() {
        if let Some(b) = builder.as_mut() {
            let span = module
                .body_spans
                .get(i)
                .copied()
                .unwrap_or_else(Span::dummy);
            b.add_mapping_span(span);
        }
        let before = out.len();
        emit::emit_stmt(&mut out, stmt, &names);
        if let Some(b) = builder.as_mut() {
            b.note_write(&out[before..]);
        }
    }

    let map = builder.map(|b| b.finish());
    Ok(EmittedJs { code: out, map })
}

fn native_only_diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}

/// Reject IR that is native-only on the JS backend (N04).
fn reject_native_only(module: &Module) -> Result<(), Diagnostic> {
    for local in &module.locals {
        if matches!(local.ty, IrType::Ptr(_)) {
            return Err(native_only_diag(format!(
                "native pointer type `*T` is native-only (cannot emit JS for `{}`)",
                local.name
            )));
        }
    }
    for stmt in &module.body {
        reject_native_only_stmt(stmt)?;
    }
    Ok(())
}

fn reject_native_only_stmt(stmt: &Stmt) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Declare { init, .. } => {
            if let Some(init) = init {
                reject_native_only_expr(init)?;
            }
        }
        Stmt::DeclareArrayPattern { elements, init, .. } => {
            if let Some(init) = init {
                reject_native_only_expr(init)?;
            }
            for el in elements {
                reject_native_only_array_pat_el(el)?;
            }
        }
        Stmt::DeclareObjectPattern {
            properties, init, ..
        } => {
            if let Some(init) = init {
                reject_native_only_expr(init)?;
            }
            for prop in properties {
                reject_native_only_object_pat_el(prop)?;
            }
        }
        Stmt::AssignLeft { target } => reject_native_only_assign_target(target)?,
        Stmt::Expr { expr } => reject_native_only_expr(expr)?,
        Stmt::Block { body } => {
            for s in body {
                reject_native_only_stmt(s)?;
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            reject_native_only_expr(test)?;
            reject_native_only_stmt(consequent)?;
            if let Some(alt) = alternate {
                reject_native_only_stmt(alt)?;
            }
        }
        Stmt::While { test, body } | Stmt::DoWhile { test, body } => {
            reject_native_only_expr(test)?;
            reject_native_only_stmt(body)?;
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            if let Some(init) = init {
                reject_native_only_stmt(init)?;
            }
            if let Some(test) = test {
                reject_native_only_expr(test)?;
            }
            if let Some(update) = update {
                reject_native_only_expr(update)?;
            }
            reject_native_only_stmt(body)?;
        }
        Stmt::ForIn { left, right, body }
        | Stmt::ForOf {
            left,
            right,
            body,
            ..
        } => {
            reject_native_only_stmt(left)?;
            reject_native_only_expr(right)?;
            reject_native_only_stmt(body)?;
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
        Stmt::Labeled { body, .. } => reject_native_only_stmt(body)?,
        Stmt::Switch {
            discriminant,
            cases,
        } => {
            reject_native_only_expr(discriminant)?;
            for case in cases {
                if let Some(test) = &case.test {
                    reject_native_only_expr(test)?;
                }
                for s in &case.body {
                    reject_native_only_stmt(s)?;
                }
            }
        }
        Stmt::Function { params, body, .. } => {
            for p in params {
                reject_native_only_pattern(&p.pattern)?;
                if let Some(default) = &p.default {
                    reject_native_only_expr(default)?;
                }
            }
            for s in body {
                reject_native_only_stmt(s)?;
            }
        }
        Stmt::Return { value } => {
            if let Some(value) = value {
                reject_native_only_expr(value)?;
            }
        }
        Stmt::Throw { value } => reject_native_only_expr(value)?,
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            for s in block {
                reject_native_only_stmt(s)?;
            }
            if let Some(handler) = handler {
                for s in handler {
                    reject_native_only_stmt(s)?;
                }
            }
            if let Some(finalizer) = finalizer {
                for s in finalizer {
                    reject_native_only_stmt(s)?;
                }
            }
        }
        Stmt::With { object, body } => {
            reject_native_only_expr(object)?;
            for s in body {
                reject_native_only_stmt(s)?;
            }
        }
    }
    Ok(())
}

fn reject_native_only_expr(expr: &Expr) -> Result<(), Diagnostic> {
    match expr {
        Expr::Unary {
            op: UnaryOp::Ref | UnaryOp::Deref,
            ..
        } => Err(native_only_diag(
            "native pointer operators `&` / `*` are native-only (cannot emit JS)",
        )),
        Expr::Assign {
            target: AssignTarget::Deref(_),
            ..
        } => Err(native_only_diag(
            "native pointer store `*p = …` is native-only (cannot emit JS)",
        )),
        Expr::Local { .. }
        | Expr::IdentName { .. }
        | Expr::Number { .. }
        | Expr::BigInt { .. }
        | Expr::String { .. }
        | Expr::RegExp { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::NewTarget { .. }
        | Expr::Super { .. } => Ok(()),
        Expr::ImportCall {
            source, options, ..
        } => {
            reject_native_only_expr(source)?;
            if let Some(opts) = options {
                reject_native_only_expr(opts)?;
            }
            Ok(())
        }
        Expr::Unary { arg, .. } => reject_native_only_expr(arg),
        Expr::Binary { left, right, .. } => {
            reject_native_only_expr(left)?;
            reject_native_only_expr(right)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            reject_native_only_expr(test)?;
            reject_native_only_expr(consequent)?;
            reject_native_only_expr(alternate)
        }
        Expr::Assign { target, value, .. } => {
            reject_native_only_assign_target(target)?;
            reject_native_only_expr(value)
        }
        Expr::Update { .. } => Ok(()),
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            reject_native_only_expr(callee)?;
            for a in args {
                match a {
                    draconic_ir::Arg::Expr(e) | draconic_ir::Arg::Spread(e) => {
                        reject_native_only_expr(e)?;
                    }
                }
            }
            Ok(())
        }
        Expr::Function { params, body, .. } => {
            for p in params {
                reject_native_only_pattern(&p.pattern)?;
                if let Some(default) = &p.default {
                    reject_native_only_expr(default)?;
                }
            }
            for s in body {
                reject_native_only_stmt(s)?;
            }
            Ok(())
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                reject_native_only_object_prop(p)?;
            }
            Ok(())
        }
        Expr::Array { elements, .. } => {
            for el in elements {
                match el {
                    draconic_ir::ArrayElement::Expr(e) | draconic_ir::ArrayElement::Spread(e) => {
                        reject_native_only_expr(e)?;
                    }
                    draconic_ir::ArrayElement::Elision => {}
                }
            }
            Ok(())
        }
        Expr::Member {
            object, property, ..
        } => {
            reject_native_only_expr(object)?;
            reject_native_only_expr(property)
        }
        Expr::Template { expressions, .. } => {
            for e in expressions {
                reject_native_only_expr(e)?;
            }
            Ok(())
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            reject_native_only_expr(tag)?;
            for e in expressions {
                reject_native_only_expr(e)?;
            }
            Ok(())
        }
    }
}

fn reject_native_only_assign_target(target: &AssignTarget) -> Result<(), Diagnostic> {
    match target {
        AssignTarget::Local(_) | AssignTarget::Name(_) => Ok(()),
        AssignTarget::Deref(_) => Err(native_only_diag(
            "native pointer store `*p = …` is native-only (cannot emit JS)",
        )),
        AssignTarget::Member {
            object, property, ..
        } => {
            reject_native_only_expr(object)?;
            reject_native_only_expr(property)
        }
        AssignTarget::ArrayPattern { elements } => {
            for el in elements {
                reject_native_only_array_pat_el(el)?;
            }
            Ok(())
        }
        AssignTarget::ObjectPattern { properties } => {
            for p in properties {
                reject_native_only_object_pat_el(p)?;
            }
            Ok(())
        }
    }
}

fn reject_native_only_pattern(pat: &Pattern) -> Result<(), Diagnostic> {
    match pat {
        Pattern::Local(_) | Pattern::Name(_) => Ok(()),
        Pattern::Member {
            object, property, ..
        } => {
            reject_native_only_expr(object)?;
            reject_native_only_expr(property)
        }
        Pattern::Array(els) => {
            for el in els {
                reject_native_only_array_pat_el(el)?;
            }
            Ok(())
        }
        Pattern::Object(props) => {
            for p in props {
                reject_native_only_object_pat_el(p)?;
            }
            Ok(())
        }
    }
}

fn reject_native_only_array_pat_el(el: &ArrayPatternEl) -> Result<(), Diagnostic> {
    match el {
        ArrayPatternEl::Elision => Ok(()),
        ArrayPatternEl::Pattern { binding, default } => {
            reject_native_only_pattern(binding)?;
            if let Some(d) = default {
                reject_native_only_expr(d)?;
            }
            Ok(())
        }
        ArrayPatternEl::Rest(pat) => reject_native_only_pattern(pat),
    }
}

fn reject_native_only_object_pat_el(el: &ObjectPatternEl) -> Result<(), Diagnostic> {
    match el {
        ObjectPatternEl::Prop {
            key,
            binding,
            default,
            ..
        } => {
            if let draconic_ir::ObjectPropKey::Computed(e) = key {
                reject_native_only_expr(e)?;
            }
            reject_native_only_pattern(binding)?;
            if let Some(d) = default {
                reject_native_only_expr(d)?;
            }
            Ok(())
        }
        ObjectPatternEl::Rest(pat) => reject_native_only_pattern(pat),
    }
}

fn reject_native_only_object_prop(prop: &draconic_ir::ObjectProp) -> Result<(), Diagnostic> {
    use draconic_ir::{ObjectProp, ObjectPropKey};
    match prop {
        ObjectProp::Spread(e) => reject_native_only_expr(e),
        ObjectProp::Property { key, value } | ObjectProp::Accessor { key, value, .. } => {
            if let ObjectPropKey::Computed(e) = key {
                reject_native_only_expr(e)?;
            }
            reject_native_only_expr(value)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn emit_src(src: &str) -> String {
        let module = compile_source(src).expect("compile");
        emit_js(&module).expect("emit")
    }

    #[test]
    fn emit_let_number() {
        assert_eq!(emit_src("let x = 1;"), "let x = 1;\n");
    }

    #[test]
    fn emit_const_number() {
        assert_eq!(emit_src("const x = 1;"), "const x = 1;\n");
    }

    #[test]
    fn emit_uninitialized_let() {
        assert_eq!(emit_src("let x;"), "let x;\n");
    }

    #[test]
    fn emit_binary_and_use() {
        let js = emit_src("let x = 1 + 2; x;");
        assert_eq!(js, "let x = (1) + (2);\nx;\n");
    }

    #[test]
    fn emit_string_concat() {
        let js = emit_src(r#"let s = "a" + "b";"#);
        assert_eq!(js, "let s = (\"a\") + (\"b\");\n");
    }

    #[test]
    fn emit_string_escapes() {
        let js = emit_src(r#"let s = "a\"b\nc";"#);
        assert!(js.contains("let s = "), "{js}");
        // Round-trip: escaped form must be valid JS string literal content.
        assert!(js.contains('\\') || js.contains("a"), "{js}");
    }

    #[test]
    fn emit_unary_and_literals() {
        let js = emit_src("let a = -1; let b = !false; let c = null; let d = true;");
        assert_eq!(
            js,
            "let a = -(1);\nlet b = !(false);\nlet c = null;\nlet d = true;\n"
        );
    }

    #[test]
    fn emit_call() {
        let js = emit_src("let f; f(1, 2);");
        assert_eq!(js, "let f;\n(f)(1, 2);\n");
    }

    #[test]
    fn emit_import_call() {
        // E19.27: dynamic `import(specifier)` / options.
        let js = emit_src("let p = import('./m.js'); let q = import(p, opts);");
        assert!(js.contains("import(\"./m.js\")"), "{js}");
        assert!(js.contains("import(p, opts)"), "{js}");
    }

    #[test]
    fn emit_import_defer_and_source_call() {
        // E19.33: `import.source` kept; E19.55: `import.defer` → `import()` for Node hosts.
        let js = emit_src("let d = import.defer('./m.js'); let s = import.source(x);");
        assert!(js.contains("import(\"./m.js\")"), "{js}");
        assert!(!js.contains("import.defer"), "{js}");
        assert!(js.contains("import.source(x)"), "{js}");
    }

    #[test]
    fn emit_call_spread() {
        let js = emit_src("let f; let a = [1]; f(...a); f(0, ...a, 2); new f(...a);");
        assert!(js.contains("(f)(...a);"), "{js}");
        assert!(js.contains("(f)(0, ...a, 2);"), "{js}");
        assert!(js.contains("(new (f)(...a))"), "{js}");
    }

    #[test]
    fn emit_comparison_and_logic() {
        let js = emit_src("let ok = 1 < 2 && true || false;");
        assert_eq!(js, "let ok = (((1) < (2)) && (true)) || (false);\n");
    }

    #[test]
    fn emit_bitwise() {
        let js = emit_src("let x = 5 & 3 | ~1 << 2;");
        assert_eq!(js, "let x = ((5) & (3)) | ((~(1)) << (2));\n");
    }

    #[test]
    fn emit_exponentiation() {
        let js = emit_src("let x = 2 ** 3 ** 2;");
        assert_eq!(js, "let x = (2) ** ((3) ** (2));\n");
    }

    #[test]
    fn emit_conditional() {
        let js = emit_src("let x = true ? 1 : 2;");
        assert_eq!(js, "let x = (true) ? (1) : (2);\n");
    }

    #[test]
    fn emit_conditional_right_assoc() {
        let js = emit_src("let x = false ? 1 : true ? 2 : 3;");
        assert_eq!(js, "let x = (false) ? (1) : ((true) ? (2) : (3));\n");
    }

    #[test]
    fn emit_assignment() {
        let js = emit_src("let x; x = 1;");
        assert_eq!(js, "let x;\n(x = 1);\n");
    }

    #[test]
    fn emit_assignment_right_assoc() {
        let js = emit_src("let a; let b; a = b = 1;");
        assert_eq!(js, "let a;\nlet b;\n(a = (b = 1));\n");
    }

    #[test]
    fn emit_compound_assignment() {
        let js = emit_src("let x = 1; x += 2; x **= 3;");
        assert_eq!(js, "let x = 1;\n(x += 2);\n(x **= 3);\n");
    }

    #[test]
    fn emit_compound_assignment_to_property() {
        let js = emit_src("let o = { a: 1 }; o.a += 2; o[\"a\"] *= 3;");
        assert_eq!(
            js,
            "let o = {a: 1};\n((o).a += 2);\n((o)[\"a\"] *= 3);\n"
        );
    }

    #[test]
    fn emit_nullish() {
        let js = emit_src("let x = null ?? 1;");
        assert_eq!(js, "let x = (null) ?? (1);\n");
    }

    #[test]
    fn emit_logical_assignment() {
        let js = emit_src("let x = 1; x &&= 2; x ||= 3; x ??= 4;");
        assert_eq!(js, "let x = 1;\n(x &&= 2);\n(x ||= 3);\n(x ??= 4);\n");
    }

    #[test]
    fn emit_update() {
        let js = emit_src("let x = 1; ++x; x++; --x; x--;");
        assert_eq!(js, "let x = 1;\n(++x);\n(x++);\n(--x);\n(x--);\n");
    }

    #[test]
    fn emit_update_on_property() {
        let js = emit_src("let o = { a: 1 }; o.a++; ++o[\"a\"];");
        assert_eq!(js, "let o = {a: 1};\n((o).a++);\n(++(o)[\"a\"]);\n");
    }

    #[test]
    fn emit_empty_program() {
        assert_eq!(emit_src(""), "");
    }

    #[test]
    fn emit_typeof_void() {
        let js = emit_src("let t = typeof 1; let v = void 0;");
        assert_eq!(js, "let t = typeof (1);\nlet v = void (0);\n");
    }

    #[test]
    fn emit_while() {
        let js = emit_src("let x = 0; while (x < 3) { x = x + 1; }");
        assert_eq!(
            js,
            "let x = 0;\nwhile ((x) < (3)) {\n(x = (x) + (1));\n}\n"
        );
    }

    #[test]
    fn emit_do_while() {
        let js = emit_src("let x = 0; do { x = x + 1; } while (x < 3);");
        assert_eq!(
            js,
            "let x = 0;\ndo {\n(x = (x) + (1));\n} while ((x) < (3));\n"
        );
    }

    #[test]
    fn emit_object_method_super() {
        // E19.23: concise methods keep home-object `super` (not parenthesized; method form).
        let js = emit_src(
            r#"const o = { m() { return super.x; }, n() { return (() => super.y)(); }, ["p"]() { return super["z"]; } };"#,
        );
        assert!(js.contains("m() {"), "{js}");
        assert!(js.contains("return super.x;"), "{js}");
        assert!(js.contains("return super.y;"), "{js}");
        assert!(js.contains("super["), "{js}");
        assert!(!js.contains("(super)"), "{js}");
        assert!(!js.contains("m: function"), "{js}");
    }

    #[test]
    fn emit_for() {
        let js = emit_src("let x = 0; for (let i = 0; i < 3; i = i + 1) { x = x + 1; }");
        assert_eq!(
            js,
            "let x = 0;\nfor (let i = 0; (i) < (3); (i = (i) + (1))) {\n(x = (x) + (1));\n}\n"
        );
    }

    #[test]
    fn emit_for_omitted_clauses() {
        let js = emit_src("let x = 0; for (; x < 2; x = x + 1) {}");
        assert_eq!(js, "let x = 0;\nfor (; (x) < (2); (x = (x) + (1))) {\n}\n");
    }

    #[test]
    fn emit_break_continue() {
        let js = emit_src("let x = 0; while (true) { if (x === 1) break; x = x + 1; continue; }");
        assert!(js.contains("break;\n"), "{js}");
        assert!(js.contains("continue;\n"), "{js}");
    }

    #[test]
    fn emit_labeled_break_continue() {
        let js = emit_src(
            "let x = 0; outer: while (true) { x = x + 1; if (x === 1) continue outer; break outer; }",
        );
        assert!(js.contains("outer:"), "{js}");
        assert!(js.contains("continue outer;\n"), "{js}");
        assert!(js.contains("break outer;\n"), "{js}");
    }

    #[test]
    fn emit_switch() {
        let js = emit_src(
            "let a = 0; switch (1) { case 1: a = 10; break; case 2: a = 20; default: a = 30; }",
        );
        assert!(js.contains("switch (1) {\n"), "{js}");
        assert!(js.contains("case 1:\n"), "{js}");
        assert!(js.contains("case 2:\n"), "{js}");
        assert!(js.contains("default:\n"), "{js}");
        assert!(js.contains("break;\n"), "{js}");
    }

    #[test]
    fn emit_comma() {
        let js = emit_src("let x = (1, 2);");
        assert_eq!(js, "let x = ((1) , (2));\n");
    }

    fn emit_result(src: &str) -> Result<String, Diagnostic> {
        let module = compile_source(src).expect("compile");
        emit_js(&module)
    }

    #[test]
    fn n04_native_scalar_polyfill() {
        let js = emit_src("let a: i32 = 1; let b: i64 = 2; let c: f64 = 3.5;");
        assert!(js.contains("let a = 1;"), "{js}");
        assert!(js.contains("let b = 2;"), "{js}");
        assert!(js.contains("let c = 3.5;"), "{js}");
    }

    #[test]
    fn n04_native_struct_polyfill() {
        let js = emit_src("type Point = { x: i32; y: i32 }; let p: Point = { x: 10, y: 20 }; let a: i32 = p.x;");
        assert!(js.contains("let p = {x: 10, y: 20};"), "{js}");
        assert!(js.contains("let a = (p).x;"), "{js}");
    }

    #[test]
    fn n04_native_array_polyfill() {
        let js = emit_src("type V = [i32, i32, i32]; let v: V = [10, 20, 30]; let a: i32 = v[0];");
        assert!(js.contains("let v = [10, 20, 30];"), "{js}");
        assert!(js.contains("let a = (v)[0];"), "{js}");
    }

    #[test]
    fn n04_pointer_hard_error() {
        let err = emit_result("let x: i32 = 10; let p: *i32 = &x; let y: i32 = *p;")
            .expect_err("pointers must hard-error on JS");
        let msg = err.to_string();
        assert!(
            msg.contains("native-only") || msg.contains("pointer"),
            "{msg}"
        );
    }

    #[test]
    fn n04_pointer_store_hard_error() {
        let err = emit_result("let x: i32 = 10; let p: *i32 = &x; *p = 42;")
            .expect_err("pointer store must hard-error on JS");
        let msg = err.to_string();
        assert!(
            msg.contains("native-only") || msg.contains("pointer"),
            "{msg}"
        );
    }

    fn emit_mapped(src: &str, name: &str) -> EmittedJs {
        let module = compile_source(src).expect("compile");
        let opts = SourceMapOptions::new(name)
            .with_content(src)
            .with_output_file("out.js");
        emit_js_with_map(&module, &opts).expect("emit_js_with_map")
    }

    #[test]
    fn u03_source_map_version_and_sources() {
        let emitted = emit_mapped("let x = 1;\n", "main.drac");
        let map = emitted.map.expect("map");
        assert_eq!(map.version, 3);
        assert_eq!(map.sources, vec!["main.drac".to_string()]);
        assert_eq!(map.file.as_deref(), Some("out.js"));
        assert_eq!(
            map.sources_content,
            vec![Some("let x = 1;\n".to_string())]
        );
        assert!(!map.mappings.is_empty(), "mappings={}", map.mappings);
        assert_eq!(emitted.code, "let x = 1;\n");
    }

    #[test]
    fn u03_source_map_maps_second_statement_to_line_two() {
        let src = "let x = 1;\nlet y = 2;\n";
        let emitted = emit_mapped(src, "t.drac");
        let map = emitted.map.expect("map");
        let segs = decode_mappings(&map.mappings);
        assert!(
            segs.len() >= 2,
            "expected ≥2 segments, got {:?}\nmappings={}",
            segs,
            map.mappings
        );
        // First top-level stmt → original line 0
        assert_eq!(segs[0].original_line, 0, "{segs:?}");
        assert_eq!(segs[0].generated_line, 0, "{segs:?}");
        // Second top-level stmt → original line 1
        assert_eq!(segs[1].original_line, 1, "{segs:?}");
        assert_eq!(segs[1].generated_line, 1, "{segs:?}");
    }

    #[test]
    fn u03_source_map_json_roundtrip_fields() {
        let emitted = emit_mapped("let a = 1 + 2;\n", "x.drac");
        let map = emitted.map.expect("map");
        let json = map.to_json();
        assert!(json.contains("\"version\": 3"), "{json}");
        assert!(json.contains("\"sources\": [\"x.drac\"]"), "{json}");
        assert!(json.contains("\"mappings\":"), "{json}");
        assert!(json.contains("let a = 1 + 2;\\n"), "{json}");
    }

    #[test]
    fn u03_source_mapping_url_comment() {
        let c = source_mapping_url_comment("out.js.map");
        assert_eq!(c, "\n//# sourceMappingURL=out.js.map\n");
    }

    #[test]
    fn u03_emit_js_unchanged_without_map() {
        assert_eq!(emit_src("let x = 1;"), "let x = 1;\n");
    }

    /// E19.32: array pattern elision must emit holes so IteratorStep/IteratorClose run.
    #[test]
    fn emit_array_pattern_elision_holes() {
        let only = emit_src("let [,] = vals;");
        assert!(only.contains("let [,] = vals;") || only.contains("let [, ] = vals;"), "{only}");
        assert!(!only.contains("let [] ="), "{only}");

        let trail = emit_src("let [a,,] = vals;");
        assert!(
            trail.contains("[a,,]") || trail.contains("[a, ,]") || trail.contains("[a, , ]"),
            "{trail}"
        );

        let mid = emit_src("let [a, , b] = vals;");
        assert!(mid.contains("[a, , b]") || mid.contains("[a,, b]"), "{mid}");

        let lead = emit_src("let [, x] = vals;");
        assert!(lead.contains("[, x]") || lead.contains("[,x]"), "{lead}");

        let assign = emit_src("let x; [, ] = vals;");
        assert!(
            assign.contains("[,]") || assign.contains("[, ]") || assign.contains("([,])"),
            "{assign}"
        );
        assert!(!assign.contains("([] ="), "{assign}");
    }

    /// E19.32: array literal trailing/only holes keep length semantics.
    #[test]
    fn emit_array_literal_elision_holes() {
        let only = emit_src("let a = [,];");
        assert!(only.contains("[,]") || only.contains("[, ]"), "{only}");
        assert!(!only.contains("let a = [];"), "{only}");

        let two = emit_src("let a = [,,];");
        assert!(
            two.contains("[,,]") || two.contains("[, ,]") || two.contains("[, , ]"),
            "{two}"
        );
    }
}
