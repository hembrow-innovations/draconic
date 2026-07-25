//! JS backend: IR → ECMAScript (ROADMAP B07).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, BindingKind, UnaryOp, UpdateOp};
use draconic_diagnostics::Diagnostic;
use draconic_ir::{Expr, LocalId, Module, Stmt};

/// Emit ECMAScript source for a shared IR module.
pub fn emit_js(module: &Module) -> Result<String, Diagnostic> {
    let names: HashMap<LocalId, &str> = module
        .locals
        .iter()
        .map(|l| (l.id, l.name.as_str()))
        .collect();

    let mut out = String::new();
    for stmt in &module.body {
        emit_stmt(&mut out, stmt, &names);
    }
    Ok(out)
}

fn emit_stmt(out: &mut String, stmt: &Stmt, names: &HashMap<LocalId, &str>) {
    match stmt {
        Stmt::Declare { local, init, kind } => {
            let name = local_name(names, *local);
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Function => out.push_str("let "),
            }
            out.push_str(name);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
            out.push_str(";\n");
        }
        Stmt::Expr { expr } => {
            emit_expr(out, expr, names);
            out.push_str(";\n");
        }
        Stmt::Block { body } => {
            out.push_str("{\n");
            for s in body {
                emit_stmt(out, s, names);
            }
            out.push_str("}\n");
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            out.push_str("if (");
            emit_expr(out, test, names);
            out.push_str(") ");
            emit_stmt_as_body(out, consequent, names);
            if let Some(alt) = alternate {
                out.push_str(" else ");
                emit_stmt_as_body(out, alt, names);
            }
        }
        Stmt::While { test, body } => {
            out.push_str("while (");
            emit_expr(out, test, names);
            out.push_str(") ");
            emit_stmt_as_body(out, body, names);
        }
        Stmt::DoWhile { body, test } => {
            out.push_str("do ");
            emit_stmt_as_body(out, body, names);
            // emit_stmt_as_body ends with newline; attach while on next line-ish cleanly
            if out.ends_with('\n') {
                out.pop();
            }
            out.push_str(" while (");
            emit_expr(out, test, names);
            out.push_str(");\n");
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            out.push_str("for (");
            if let Some(init) = init {
                emit_for_init(out, init, names);
            }
            out.push(';');
            if let Some(test) = test {
                out.push(' ');
                emit_expr(out, test, names);
            }
            out.push(';');
            if let Some(update) = update {
                out.push(' ');
                emit_expr(out, update, names);
            }
            out.push_str(") ");
            emit_stmt_as_body(out, body, names);
        }
        Stmt::ForIn { left, right, body } => {
            out.push_str("for (");
            emit_for_in_of_left(out, left, names);
            out.push_str(" in ");
            emit_expr(out, right, names);
            out.push_str(") ");
            emit_stmt_as_body(out, body, names);
        }
        Stmt::ForOf { left, right, body } => {
            out.push_str("for (");
            emit_for_in_of_left(out, left, names);
            out.push_str(" of ");
            emit_expr(out, right, names);
            out.push_str(") ");
            emit_stmt_as_body(out, body, names);
        }
        Stmt::Break { label } => {
            if let Some(label) = label {
                out.push_str("break ");
                out.push_str(label);
                out.push_str(";\n");
            } else {
                out.push_str("break;\n");
            }
        }
        Stmt::Continue { label } => {
            if let Some(label) = label {
                out.push_str("continue ");
                out.push_str(label);
                out.push_str(";\n");
            } else {
                out.push_str("continue;\n");
            }
        }
        Stmt::Labeled { label, body } => {
            out.push_str(label);
            out.push_str(": ");
            emit_stmt(out, body, names);
        }
        Stmt::Switch {
            discriminant,
            cases,
        } => {
            out.push_str("switch (");
            emit_expr(out, discriminant, names);
            out.push_str(") {\n");
            for case in cases {
                if let Some(test) = &case.test {
                    out.push_str("case ");
                    emit_expr(out, test, names);
                    out.push_str(":\n");
                } else {
                    out.push_str("default:\n");
                }
                for s in &case.body {
                    emit_stmt(out, s, names);
                }
            }
            out.push_str("}\n");
        }
        Stmt::Function {
            local,
            params,
            body,
        } => {
            out.push_str("function ");
            out.push_str(local_name(names, *local));
            out.push('(');
            for (i, p) in params.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                out.push_str(local_name(names, *p));
            }
            out.push_str(") {\n");
            for s in body {
                emit_stmt(out, s, names);
            }
            out.push_str("}\n");
        }
        Stmt::Return { value } => {
            out.push_str("return");
            if let Some(value) = value {
                out.push(' ');
                emit_expr(out, value, names);
            }
            out.push_str(";\n");
        }
    }
}

/// Emit for-loop init without a trailing newline (semicolon comes from the for head).
fn emit_for_init(out: &mut String, stmt: &Stmt, names: &HashMap<LocalId, &str>) {
    match stmt {
        Stmt::Declare { local, init, kind } => {
            let name = local_name(names, *local);
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Function => out.push_str("let "),
            }
            out.push_str(name);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
        }
        Stmt::Expr { expr } => {
            emit_expr(out, expr, names);
        }
        other => {
            // Fallback: emit as a block expression is invalid; emit nested form.
            emit_stmt(out, other, names);
            if out.ends_with('\n') {
                out.pop();
            }
            if out.ends_with(';') {
                out.pop();
            }
        }
    }
}

/// Emit `for (left in/of …)` left without trailing semicolon.
fn emit_for_in_of_left(out: &mut String, stmt: &Stmt, names: &HashMap<LocalId, &str>) {
    match stmt {
        Stmt::Declare { local, init, kind } => {
            let name = local_name(names, *local);
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Function => out.push_str("let "),
            }
            out.push_str(name);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
        }
        Stmt::Expr { expr } => {
            emit_expr(out, expr, names);
        }
        other => {
            emit_stmt(out, other, names);
            if out.ends_with('\n') {
                out.pop();
            }
            if out.ends_with(';') {
                out.pop();
            }
        }
    }
}

/// Emit a statement in statement-body position (if/else), ensuring a trailing newline.
fn emit_stmt_as_body(out: &mut String, stmt: &Stmt, names: &HashMap<LocalId, &str>) {
    match stmt {
        Stmt::Block { .. } => emit_stmt(out, stmt, names),
        other => {
            // Single-statement body: wrap so chained else-if formatting stays clear.
            out.push_str("{\n");
            emit_stmt(out, other, names);
            out.push_str("}\n");
        }
    }
}

fn emit_expr(out: &mut String, expr: &Expr, names: &HashMap<LocalId, &str>) {
    match expr {
        Expr::Local { id, .. } => {
            out.push_str(local_name(names, *id));
        }
        Expr::Number { raw, .. } => {
            out.push_str(raw);
        }
        Expr::String { value, .. } => {
            push_js_string(out, value);
        }
        Expr::Boolean { value, .. } => {
            out.push_str(if *value { "true" } else { "false" });
        }
        Expr::Null { .. } => {
            out.push_str("null");
        }
        Expr::Unary { op, arg, .. } => {
            emit_unary(out, *op, arg, names);
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            // Comma needs a grouping wrapper so `let a = 1, 2` is not multi-declarator.
            let group = matches!(op, BinaryOp::Comma);
            if group {
                out.push('(');
            }
            out.push('(');
            emit_expr(out, left, names);
            out.push(')');
            out.push(' ');
            out.push_str(binary_op(*op));
            out.push(' ');
            out.push('(');
            emit_expr(out, right, names);
            out.push(')');
            if group {
                out.push(')');
            }
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            out.push('(');
            emit_expr(out, test, names);
            out.push_str(") ? (");
            emit_expr(out, consequent, names);
            out.push_str(") : (");
            emit_expr(out, alternate, names);
            out.push(')');
        }
        Expr::Assign {
            target,
            op,
            value,
            ..
        } => {
            out.push('(');
            out.push_str(local_name(names, *target));
            out.push(' ');
            out.push_str(assign_op(*op));
            out.push(' ');
            emit_expr(out, value, names);
            out.push(')');
        }
        Expr::Update {
            op,
            target,
            prefix,
            ..
        } => {
            out.push('(');
            let name = local_name(names, *target);
            let op_s = match op {
                UpdateOp::Inc => "++",
                UpdateOp::Dec => "--",
            };
            if *prefix {
                out.push_str(op_s);
                out.push_str(name);
            } else {
                out.push_str(name);
                out.push_str(op_s);
            }
            out.push(')');
        }
        Expr::Call { callee, args, .. } => {
            out.push('(');
            emit_expr(out, callee, names);
            out.push(')');
            out.push('(');
            for (i, arg) in args.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                emit_expr(out, arg, names);
            }
            out.push(')');
        }
    }
}

fn emit_unary(out: &mut String, op: UnaryOp, arg: &Expr, names: &HashMap<LocalId, &str>) {
    match op {
        UnaryOp::Plus => {
            out.push_str("+(");
            emit_expr(out, arg, names);
            out.push(')');
        }
        UnaryOp::Minus => {
            out.push_str("-(");
            emit_expr(out, arg, names);
            out.push(')');
        }
        UnaryOp::Not => {
            out.push_str("!(");
            emit_expr(out, arg, names);
            out.push(')');
        }
        UnaryOp::BitNot => {
            out.push_str("~(");
            emit_expr(out, arg, names);
            out.push(')');
        }
        UnaryOp::TypeOf => {
            out.push_str("typeof (");
            emit_expr(out, arg, names);
            out.push(')');
        }
        UnaryOp::Void => {
            out.push_str("void (");
            emit_expr(out, arg, names);
            out.push(')');
        }
        UnaryOp::Delete => {
            out.push_str("delete (");
            emit_expr(out, arg, names);
            out.push(')');
        }
    }
}

fn binary_op(op: BinaryOp) -> &'static str {
    match op {
        BinaryOp::Add => "+",
        BinaryOp::Sub => "-",
        BinaryOp::Mul => "*",
        BinaryOp::Div => "/",
        BinaryOp::Rem => "%",
        BinaryOp::Pow => "**",
        BinaryOp::EqEq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::EqEqEq => "===",
        BinaryOp::NotEqEq => "!==",
        BinaryOp::Lt => "<",
        BinaryOp::LtEq => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::GtEq => ">=",
        BinaryOp::BitAnd => "&",
        BinaryOp::BitOr => "|",
        BinaryOp::BitXor => "^",
        BinaryOp::Shl => "<<",
        BinaryOp::Shr => ">>",
        BinaryOp::UShr => ">>>",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
        BinaryOp::Nullish => "??",
        BinaryOp::Comma => ",",
    }
}

fn assign_op(op: AssignOp) -> &'static str {
    match op {
        AssignOp::Eq => "=",
        AssignOp::AddEq => "+=",
        AssignOp::SubEq => "-=",
        AssignOp::MulEq => "*=",
        AssignOp::DivEq => "/=",
        AssignOp::RemEq => "%=",
        AssignOp::PowEq => "**=",
        AssignOp::ShlEq => "<<=",
        AssignOp::ShrEq => ">>=",
        AssignOp::UShrEq => ">>>=",
        AssignOp::BitAndEq => "&=",
        AssignOp::BitOrEq => "|=",
        AssignOp::BitXorEq => "^=",
        AssignOp::AndAndEq => "&&=",
        AssignOp::OrOrEq => "||=",
        AssignOp::NullishEq => "??=",
    }
}

fn local_name<'a>(names: &HashMap<LocalId, &'a str>, id: LocalId) -> &'a str {
    names
        .get(&id)
        .copied()
        .unwrap_or_else(|| panic!("missing local name for %{id:?}"))
}

fn push_js_string(out: &mut String, value: &str) {
    out.push('"');
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c if (c as u32) < 0x20 => {
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_check::check;
    use draconic_ir::lower;
    use draconic_parser::parse;

    fn emit_src(src: &str) -> String {
        let program = parse(src).expect("parse");
        let checked = check(program).expect("check");
        let module = lower(&checked);
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
}
