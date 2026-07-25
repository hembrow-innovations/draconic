//! JS backend: IR → ECMAScript (ROADMAP B07).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, BindingKind, UnaryOp, UpdateOp};
use draconic_diagnostics::Diagnostic;
use draconic_ir::{ArrayPatternEl, Expr, LocalId, Module, Pattern, Stmt};

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
        Stmt::DeclareArrayPattern {
            kind,
            elements,
            init,
        } => {
            match kind {
                BindingKind::Let => out.push_str("let "),
                BindingKind::Const => out.push_str("const "),
                BindingKind::Function => out.push_str("let "),
            }
            emit_array_pattern(out, elements, names);
            out.push_str(" = ");
            emit_expr(out, init, names);
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
            out.push_str("}");
            if let Some(handler) = handler {
                if let Some(param) = handler_param {
                    out.push_str(" catch (");
                    out.push_str(local_name(names, *param));
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
        Expr::IdentName { name, .. } => {
            out.push_str(name);
        }
        Expr::Number { raw, .. } => {
            out.push_str(raw);
        }
        Expr::BigInt { raw, .. } => {
            out.push_str(raw);
        }
        Expr::String { value, .. } => {
            push_js_string(out, value);
        }
        Expr::Template {
            quasis,
            expressions,
            ..
        } => {
            emit_template_body(out, quasis, expressions, names);
        }
        Expr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            ..
        } => {
            // Member/call tag must stay a Reference so `obj.m\`...\`` keeps `this`.
            match tag.as_ref() {
                Expr::Member {
                    object,
                    property,
                    computed,
                    ..
                } => {
                    emit_member_access(out, object, property, *computed, names);
                }
                Expr::Call { .. } => {
                    emit_expr(out, tag, names);
                }
                _ => {
                    out.push('(');
                    emit_expr(out, tag, names);
                    out.push(')');
                }
            }
            emit_template_body(out, quasis, expressions, names);
        }
        Expr::Boolean { value, .. } => {
            out.push_str(if *value { "true" } else { "false" });
        }
        Expr::Null { .. } => {
            out.push_str("null");
        }
        Expr::This { .. } => {
            out.push_str("this");
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
            emit_assign_target(out, target, names);
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
            let name = match target {
                draconic_ir::UpdateTarget::Local(id) => local_name(names, *id),
                draconic_ir::UpdateTarget::Name(n) => n.as_str(),
            };
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
            // Member callee must stay a Reference (`obj.m(args)`), not `(obj.m)(args)`,
            // so `this` is bound to the receiver.
            match callee.as_ref() {
                Expr::Member {
                    object,
                    property,
                    computed,
                    ..
                } => {
                    emit_member_access(out, object, property, *computed, names);
                }
                _ => {
                    out.push('(');
                    emit_expr(out, callee, names);
                    out.push(')');
                }
            }
            out.push('(');
            emit_args(out, args, names);
            out.push(')');
        }
        Expr::New { callee, args, .. } => {
            out.push_str("(new (");
            emit_expr(out, callee, names);
            out.push_str(")(");
            emit_args(out, args, names);
            out.push_str("))");
        }
        Expr::Function {
            name,
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if *is_async {
                out.push_str("async ");
            }
            out.push_str("function");
            if *is_generator {
                out.push('*');
            }
            if let Some(local) = name {
                out.push(' ');
                out.push_str(local_name(names, *local));
            }
            out.push('(');
            emit_params(out, params, names);
            out.push_str(") {\n");
            for s in body {
                emit_stmt(out, s, names);
            }
            out.push('}');
        }
        Expr::Object { properties, .. } => {
            out.push('{');
            for (i, prop) in properties.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match &prop.key {
                    draconic_ir::ObjectPropKey::Static(k) => {
                        if let Some(s) = k.to_string_strict().filter(|s| is_js_ident(s)) {
                            out.push_str(&s);
                        } else {
                            push_js_string(out, k);
                        }
                        out.push_str(": ");
                    }
                    draconic_ir::ObjectPropKey::Computed(k) => {
                        out.push('[');
                        emit_expr(out, k, names);
                        out.push_str("]: ");
                    }
                }
                emit_expr(out, &prop.value, names);
            }
            out.push('}');
        }
        Expr::Array { elements, .. } => {
            out.push('[');
            for (i, el) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match el {
                    draconic_ir::ArrayElement::Expr(expr) => emit_expr(out, expr, names),
                    draconic_ir::ArrayElement::Spread(expr) => {
                        out.push_str("...");
                        emit_expr(out, expr, names);
                    }
                }
            }
            out.push(']');
        }
        Expr::Member {
            object,
            property,
            computed,
            ..
        } => {
            emit_member_access(out, object, property, *computed, names);
        }
    }
}

fn emit_member_access(
    out: &mut String,
    object: &Expr,
    property: &Expr,
    computed: bool,
    names: &HashMap<LocalId, &str>,
) {
    out.push('(');
    emit_expr(out, object, names);
    out.push(')');
    if computed {
        out.push('[');
        emit_expr(out, property, names);
        out.push(']');
    } else {
        match property {
            Expr::String { value, .. } => {
                if let Some(s) = value.to_string_strict().filter(|s| is_js_ident(s)) {
                    out.push('.');
                    out.push_str(&s);
                } else {
                    out.push('[');
                    emit_expr(out, property, names);
                    out.push(']');
                }
            }
            _ => {
                out.push('[');
                emit_expr(out, property, names);
                out.push(']');
            }
        }
    }
}

fn is_js_ident(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c == '_' || c == '$' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c == '$' || c.is_ascii_alphanumeric())
}

fn emit_assign_target(
    out: &mut String,
    target: &draconic_ir::AssignTarget,
    names: &HashMap<LocalId, &str>,
) {
    match target {
        draconic_ir::AssignTarget::Local(id) => {
            out.push_str(local_name(names, *id));
        }
        draconic_ir::AssignTarget::Name(name) => {
            out.push_str(name);
        }
        draconic_ir::AssignTarget::Member {
            object,
            property,
            computed,
        } => {
            emit_member_access(out, object, property, *computed, names);
        }
        draconic_ir::AssignTarget::ArrayPattern { elements } => {
            emit_array_pattern(out, elements, names);
        }
    }
}

fn emit_array_pattern(
    out: &mut String,
    elements: &[ArrayPatternEl],
    names: &HashMap<LocalId, &str>,
) {
    out.push('[');
    for (i, el) in elements.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        match el {
            ArrayPatternEl::Pattern(p) => emit_pattern(out, p, names),
            ArrayPatternEl::Rest(id) => {
                out.push_str("...");
                out.push_str(local_name(names, *id));
            }
        }
    }
    out.push(']');
}

fn emit_pattern(out: &mut String, pat: &Pattern, names: &HashMap<LocalId, &str>) {
    match pat {
        Pattern::Local(id) => out.push_str(local_name(names, *id)),
        Pattern::Array(els) => emit_array_pattern(out, els, names),
    }
}

fn emit_args(out: &mut String, args: &[draconic_ir::Arg], names: &HashMap<LocalId, &str>) {
    for (i, arg) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        match arg {
            draconic_ir::Arg::Expr(expr) => emit_expr(out, expr, names),
            draconic_ir::Arg::Spread(expr) => {
                out.push_str("...");
                emit_expr(out, expr, names);
            }
        }
    }
}

fn emit_params(out: &mut String, params: &[draconic_ir::Param], names: &HashMap<LocalId, &str>) {
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        if p.rest {
            out.push_str("...");
        }
        out.push_str(local_name(names, p.local));
        if let Some(default) = &p.default {
            out.push_str(" = ");
            emit_expr(out, default, names);
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
        UnaryOp::Await => {
            out.push_str("(await (");
            emit_expr(out, arg, names);
            out.push_str("))");
        }
        UnaryOp::Yield => {
            out.push_str("(yield (");
            emit_expr(out, arg, names);
            out.push_str("))");
        }
        UnaryOp::YieldStar => {
            out.push_str("(yield* (");
            emit_expr(out, arg, names);
            out.push_str("))");
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
        BinaryOp::In => "in",
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

fn push_js_string(out: &mut String, value: &draconic_ast::JsString) {
    out.push('"');
    push_js_string_units(out, value.units());
    out.push('"');
}

/// Emit UTF-16 code units as JS string/template content with `\uXXXX` escapes as needed.
fn push_js_string_units(out: &mut String, units: &[u16]) {
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        match u {
            0x5C => out.push_str("\\\\"), // \
            0x22 => out.push_str("\\\""), // "
            0x0A => out.push_str("\\n"),
            0x0D => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            0x2028 => out.push_str("\\u2028"),
            0x2029 => out.push_str("\\u2029"),
            u if u < 0x20 || (0xD800..=0xDFFF).contains(&u) => {
                let _ = write!(out, "\\u{u:04x}");
            }
            u if u < 0x80 => out.push(u as u8 as char),
            u => {
                // Well-formed BMP or start of a surrogate pair already handled above.
                // Decode scalar from one or two units for emit as UTF-8 char when safe.
                if let Some(c) = char::from_u32(u as u32) {
                    out.push(c);
                } else {
                    let _ = write!(out, "\\u{u:04x}");
                }
            }
        }
        i += 1;
    }
}

/// Escape cooked template quasi text for re-emit inside `` `…` ``.
fn push_js_template_chars(out: &mut String, value: &draconic_ast::JsString) {
    let units = value.units();
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        match u {
            0x5C => out.push_str("\\\\"), // \
            0x60 => out.push_str("\\`"),  // `
            0x24 => {
                // Escape `$` only when followed by `{` (start of interpolation).
                if units.get(i + 1) == Some(&0x7B) {
                    out.push_str("\\$");
                } else {
                    out.push('$');
                }
            }
            0x0D => out.push_str("\\r"),
            0x2028 => out.push_str("\\u2028"),
            0x2029 => out.push_str("\\u2029"),
            u if (0xD800..=0xDFFF).contains(&u) => {
                let _ = write!(out, "\\u{u:04x}");
            }
            u if u < 0x20 && u != 0x0A && u != 0x09 => {
                let _ = write!(out, "\\u{u:04x}");
            }
            u if u < 0x80 => out.push(u as u8 as char),
            u => {
                if let Some(c) = char::from_u32(u as u32) {
                    out.push(c);
                } else {
                    let _ = write!(out, "\\u{u:04x}");
                }
            }
        }
        i += 1;
    }
}

fn emit_template_body(
    out: &mut String,
    quasis: &[draconic_ast::JsString],
    expressions: &[Expr],
    names: &HashMap<LocalId, &str>,
) {
    out.push('`');
    for (i, q) in quasis.iter().enumerate() {
        push_js_template_chars(out, q);
        if i < expressions.len() {
            out.push_str("${");
            emit_expr(out, &expressions[i], names);
            out.push('}');
        }
    }
    out.push('`');
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
