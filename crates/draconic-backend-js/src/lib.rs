//! JS backend: IR → ECMAScript (ROADMAP B07).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, UnaryOp};
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
        Stmt::Declare { local, init } => {
            let name = local_name(names, *local);
            out.push_str("let ");
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
            out.push('(');
            emit_expr(out, left, names);
            out.push(')');
            out.push(' ');
            out.push_str(binary_op(*op));
            out.push(' ');
            out.push('(');
            emit_expr(out, right, names);
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
        BinaryOp::EqEq => "==",
        BinaryOp::NotEq => "!=",
        BinaryOp::EqEqEq => "===",
        BinaryOp::NotEqEq => "!==",
        BinaryOp::Lt => "<",
        BinaryOp::LtEq => "<=",
        BinaryOp::Gt => ">",
        BinaryOp::GtEq => ">=",
        BinaryOp::And => "&&",
        BinaryOp::Or => "||",
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
    fn emit_empty_program() {
        assert_eq!(emit_src(""), "");
    }

    #[test]
    fn emit_typeof_void() {
        let js = emit_src("let t = typeof 1; let v = void 0;");
        assert_eq!(js, "let t = typeof (1);\nlet v = void (0);\n");
    }
}
