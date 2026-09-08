//! IR statement emission to ECMAScript source text.

use std::collections::HashMap;

use draconic_ast::{BinaryOp, BindingKind};
use draconic_ir::{Expr, LocalId, Stmt};

use crate::es_expr::{
    emit_array_pattern, emit_assign_target, emit_expr, emit_object_pattern, emit_params,
    emit_pattern, local_name,
};

pub(crate) fn emit_stmt(out: &mut String, stmt: &Stmt, names: &HashMap<LocalId, &str>) {
    match stmt {
        Stmt::Declare { local, init, kind } => {
            let name = local_name(names, *local);
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Var => out.push_str("var "),
                BindingKind::Function => out.push_str("let "),
                BindingKind::Using => out.push_str("using "),
                BindingKind::AwaitUsing => out.push_str("await using "),
            }
            out.push_str(name);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
            out.push_str(";\n");
        }
        Stmt::DeclareArrayPattern {
            kind,
            elements,
            init,
        } => {
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Var => out.push_str("var "),
                BindingKind::Function => out.push_str("let "),
                BindingKind::Using => out.push_str("using "),
                BindingKind::AwaitUsing => out.push_str("await using "),
            }
            emit_array_pattern(out, elements, names);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
            out.push_str(";\n");
        }
        Stmt::DeclareObjectPattern {
            kind,
            properties,
            init,
        } => {
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Var => out.push_str("var "),
                BindingKind::Function => out.push_str("let "),
                BindingKind::Using => out.push_str("using "),
                BindingKind::AwaitUsing => out.push_str("await using "),
            }
            emit_object_pattern(out, properties, names);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
            out.push_str(";\n");
        }
        Stmt::AssignLeft { target } => {
            emit_assign_target(out, target, names);
            out.push_str(";\n");
        }
        Stmt::Expr { expr } => {
            // Object / function / class at statement start need grouping so they
            // are not parsed as block / declaration (E19.52 await-ident fixtures).
            if expr_needs_stmt_paren(expr) {
                out.push('(');
                emit_expr(out, expr, names);
                out.push(')');
            } else {
                emit_expr(out, expr, names);
            }
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
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
        } => {
            if *is_await {
                out.push_str("for await (");
            } else {
                out.push_str("for (");
            }
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
            is_async,
            is_generator,
        } => {
            if *is_async {
                out.push_str("async ");
            }
            out.push_str("function");
            if *is_generator {
                out.push('*');
            }
            out.push(' ');
            out.push_str(local_name(names, *local));
            out.push('(');
            emit_params(out, params, names);
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
        Stmt::Throw { value } => {
            out.push_str("throw ");
            emit_expr(out, value, names);
            out.push_str(";\n");
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            out.push_str("try {\n");
            for s in block {
                emit_stmt(out, s, names);
            }
            out.push('}');
            if let Some(handler) = handler {
                if let Some(param) = handler_param {
                    out.push_str(" catch (");
                    emit_pattern(out, param, names);
                    out.push_str(") {\n");
                } else {
                    out.push_str(" catch {\n");
                }
                for s in handler {
                    emit_stmt(out, s, names);
                }
                out.push('}');
            }
            if let Some(finalizer) = finalizer {
                out.push_str(" finally {\n");
                for s in finalizer {
                    emit_stmt(out, s, names);
                }
                out.push('}');
            }
            out.push('\n');
        }
        Stmt::With { object, body } => {
            out.push_str("with (");
            emit_expr(out, object, names);
            out.push_str(") {\n");
            for s in body {
                emit_stmt(out, s, names);
            }
            out.push_str("}\n");
        }
        // F06.03 / F08.01: extern is rejected before emit; no JS surface.
        Stmt::ExternFunction { .. } => {}
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
                BindingKind::Var => out.push_str("var "),
                BindingKind::Function => out.push_str("let "),
                BindingKind::Using => out.push_str("using "),
                BindingKind::AwaitUsing => out.push_str("await using "),
            }
            out.push_str(name);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
        }
        Stmt::DeclareArrayPattern {
            kind,
            elements,
            init,
        } => {
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Var => out.push_str("var "),
                BindingKind::Function => out.push_str("let "),
                BindingKind::Using => out.push_str("using "),
                BindingKind::AwaitUsing => out.push_str("await using "),
            }
            emit_array_pattern(out, elements, names);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
        }
        Stmt::DeclareObjectPattern {
            kind,
            properties,
            init,
        } => {
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Var => out.push_str("var "),
                BindingKind::Function => out.push_str("let "),
                BindingKind::Using => out.push_str("using "),
                BindingKind::AwaitUsing => out.push_str("await using "),
            }
            emit_object_pattern(out, properties, names);
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
                BindingKind::Var => out.push_str("var "),
                BindingKind::Function => out.push_str("let "),
                BindingKind::Using => out.push_str("using "),
                BindingKind::AwaitUsing => out.push_str("await using "),
            }
            out.push_str(name);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
        }
        Stmt::DeclareArrayPattern {
            kind,
            elements,
            init,
        } => {
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Var => out.push_str("var "),
                BindingKind::Function => out.push_str("let "),
                BindingKind::Using => out.push_str("using "),
                BindingKind::AwaitUsing => out.push_str("await using "),
            }
            emit_array_pattern(out, elements, names);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
        }
        Stmt::DeclareObjectPattern {
            kind,
            properties,
            init,
        } => {
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Var => out.push_str("var "),
                BindingKind::Function => out.push_str("let "),
                BindingKind::Using => out.push_str("using "),
                BindingKind::AwaitUsing => out.push_str("await using "),
            }
            emit_object_pattern(out, properties, names);
            if let Some(init) = init {
                out.push_str(" = ");
                emit_expr(out, init, names);
            }
        }
        Stmt::AssignLeft { target } => {
            emit_assign_target(out, target, names);
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

/// Expression statement forms that would be mis-parsed without grouping parens.
fn expr_needs_stmt_paren(expr: &Expr) -> bool {
    match expr {
        Expr::Object { .. } => true,
        // Non-arrow function expression at stmt start → FunctionDeclaration.
        Expr::Function {
            is_arrow: false, ..
        } => true,
        Expr::Assign { value, .. } => expr_needs_stmt_paren(value),
        Expr::Binary {
            op: BinaryOp::Comma,
            left,
            ..
        } => expr_needs_stmt_paren(left),
        _ => false,
    }
}
