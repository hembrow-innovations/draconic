//! Deterministic source printer for `draconic fmt` (ROADMAP U05).
//!
//! Style (v1): 2-space indent, stable spacing, no comment preservation.
//! `print_program` is pure AST → text; parse → print → parse → print is
//! idempotent for well-formed programs.

mod print_expr;
mod print_type;

use print_expr::{
    expr_needs_stmt_paren, print_expr, print_expr_inner, print_object_key, print_string_lit,
    PREC_ASSIGN,
};
use print_type::{print_type_ann, print_type_params};
use crate::{
    AccessorKind, ArrayPatternElement, BindingKind, BindingPattern, ClassElement, ImportAttribute,
    ImportAttributeKey, ImportPhase, ObjectPatternProp, Param, Program, Stmt,
};

/// Pretty-print a Program as Draconic source text (stable style, 2-space indent).
pub fn print_program(program: &Program) -> String {
    let mut out = String::new();
    for stmt in &program.body {
        print_stmt(stmt, 0, &mut out);
    }
    out
}

fn indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

fn print_stmt(stmt: &Stmt, level: usize, out: &mut String) {
    match stmt {
        Stmt::Expression { expr, .. } => {
            indent(level, out);
            // Avoid ASI / declaration ambiguity for object/function/class at stmt start.
            if expr_needs_stmt_paren(expr) {
                out.push('(');
                print_expr(expr, 0, out);
                out.push(')');
            } else {
                print_expr(expr, 0, out);
            }
            out.push_str(";\n");
        }
        Stmt::Let {
            kind,
            binding,
            type_ann,
            init,
            ..
        } => {
            indent(level, out);
            print_binding_kind(*kind, out);
            print_binding_pattern(binding, out);
            if let Some(ty) = type_ann {
                out.push_str(": ");
                print_type_ann(ty, out);
            }
            if let Some(init) = init {
                out.push_str(" = ");
                print_expr(init, 0, out);
            }
            out.push_str(";\n");
        }
        Stmt::Empty { .. } => {
            indent(level, out);
            out.push_str(";\n");
        }
        Stmt::Block { body, .. } => {
            indent(level, out);
            print_block_body(body, level, out);
            out.push('\n');
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            indent(level, out);
            out.push_str("if (");
            print_expr(test, 0, out);
            out.push_str(") ");
            print_stmt_body(consequent, level, out);
            if let Some(alt) = alternate {
                // print_stmt_body ends with \n; attach else on same visual flow.
                if out.ends_with('\n') {
                    out.pop();
                }
                out.push_str(" else ");
                // else-if chain: keep `else if` without extra brace when alt is If.
                if matches!(alt.as_ref(), Stmt::If { .. }) {
                    // Strip indent that print_stmt would add — print If without leading indent.
                    print_if_continued(alt, level, out);
                } else {
                    print_stmt_body(alt, level, out);
                }
            }
        }
        Stmt::While { test, body, .. } => {
            indent(level, out);
            out.push_str("while (");
            print_expr(test, 0, out);
            out.push_str(") ");
            print_stmt_body(body, level, out);
        }
        Stmt::DoWhile { body, test, .. } => {
            indent(level, out);
            out.push_str("do ");
            print_stmt_body(body, level, out);
            if out.ends_with('\n') {
                out.pop();
            }
            out.push_str(" while (");
            print_expr(test, 0, out);
            out.push_str(");\n");
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            indent(level, out);
            out.push_str("for (");
            if let Some(init) = init {
                print_for_init(init, out);
            }
            out.push(';');
            if let Some(test) = test {
                out.push(' ');
                print_expr(test, 0, out);
            }
            out.push(';');
            if let Some(update) = update {
                out.push(' ');
                print_expr(update, 0, out);
            }
            out.push_str(") ");
            print_stmt_body(body, level, out);
        }
        Stmt::ForIn {
            left, right, body, ..
        } => {
            indent(level, out);
            out.push_str("for (");
            print_for_in_of_left(left, out);
            out.push_str(" in ");
            print_expr(right, 0, out);
            out.push_str(") ");
            print_stmt_body(body, level, out);
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
            ..
        } => {
            indent(level, out);
            if *is_await {
                out.push_str("for await (");
            } else {
                out.push_str("for (");
            }
            print_for_in_of_left(left, out);
            out.push_str(" of ");
            print_expr(right, 0, out);
            out.push_str(") ");
            print_stmt_body(body, level, out);
        }
        Stmt::Break { label, .. } => {
            indent(level, out);
            out.push_str("break");
            if let Some(lab) = label {
                out.push(' ');
                out.push_str(&lab.name);
            }
            out.push_str(";\n");
        }
        Stmt::Continue { label, .. } => {
            indent(level, out);
            out.push_str("continue");
            if let Some(lab) = label {
                out.push(' ');
                out.push_str(&lab.name);
            }
            out.push_str(";\n");
        }
        Stmt::Labeled { label, body, .. } => {
            indent(level, out);
            out.push_str(&label.name);
            out.push_str(": ");
            // Body on same line start — if block, no extra indent prefix from body.
            match body.as_ref() {
                Stmt::Block { body, .. } => {
                    print_block_body(body, level, out);
                    out.push('\n');
                }
                other => {
                    // Avoid double indent: print without indent by using a temp.
                    let mut tmp = String::new();
                    print_stmt(other, 0, &mut tmp);
                    // Re-indent each line of tmp at `level` (tmp has no leading indent).
                    // Actually print_stmt(other, 0) has no indent — paste after label.
                    // If tmp is multi-line, subsequent lines need indent.
                    let trimmed = tmp.trim_end_matches('\n');
                    let mut lines = trimmed.lines();
                    if let Some(first) = lines.next() {
                        out.push_str(first);
                        out.push('\n');
                        for line in lines {
                            indent(level, out);
                            out.push_str(line);
                            out.push('\n');
                        }
                    }
                }
            }
        }
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            indent(level, out);
            out.push_str("switch (");
            print_expr(discriminant, 0, out);
            out.push_str(") {\n");
            for case in cases {
                indent(level + 1, out);
                match &case.test {
                    Some(t) => {
                        out.push_str("case ");
                        print_expr(t, 0, out);
                        out.push_str(":\n");
                    }
                    None => out.push_str("default:\n"),
                }
                for s in &case.body {
                    print_stmt(s, level + 2, out);
                }
            }
            indent(level, out);
            out.push_str("}\n");
        }
        Stmt::FunctionDeclaration {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            is_generator,
            ..
        } => {
            indent(level, out);
            if *is_async {
                out.push_str("async ");
            }
            out.push_str("function");
            if *is_generator {
                out.push('*');
            }
            out.push(' ');
            out.push_str(&name.name);
            print_type_params(type_params, out);
            print_param_list(params, out);
            if let Some(ret) = return_type {
                out.push_str(": ");
                print_type_ann(ret, out);
            }
            out.push(' ');
            print_function_body(body, level, out);
            out.push('\n');
        }
        Stmt::ClassDeclaration {
            name,
            super_class,
            body,
            ..
        } => {
            indent(level, out);
            out.push_str("class ");
            out.push_str(&name.name);
            if let Some(sup) = super_class {
                out.push_str(" extends ");
                print_expr(sup, 0, out);
            }
            out.push(' ');
            print_class_body(body, level, out);
            out.push('\n');
        }
        Stmt::Return { argument, .. } => {
            indent(level, out);
            out.push_str("return");
            if let Some(arg) = argument {
                out.push(' ');
                print_expr(arg, 0, out);
            }
            out.push_str(";\n");
        }
        Stmt::Throw { argument, .. } => {
            indent(level, out);
            out.push_str("throw ");
            print_expr(argument, 0, out);
            out.push_str(";\n");
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            ..
        } => {
            indent(level, out);
            out.push_str("try ");
            print_try_block(block, level, out);
            if let Some(h) = handler {
                out.push_str(" catch");
                if let Some(p) = handler_param {
                    out.push_str(" (");
                    print_binding_pattern(p, out);
                    out.push(')');
                }
                out.push(' ');
                print_try_block(h, level, out);
            }
            if let Some(f) = finalizer {
                out.push_str(" finally ");
                print_try_block(f, level, out);
            }
            out.push('\n');
        }
        Stmt::With { object, body, .. } => {
            indent(level, out);
            out.push_str("with (");
            print_expr(object, 0, out);
            out.push_str(") ");
            print_stmt_body(body, level, out);
        }
        Stmt::ImportDeclaration {
            specifiers,
            namespace,
            source,
            attributes,
            phase,
            ..
        } => {
            indent(level, out);
            out.push_str("import ");
            if *phase == ImportPhase::Defer {
                out.push_str("defer ");
            }
            let has_default = specifiers.iter().any(|s| s.imported.name == "default");
            let named: Vec<_> = specifiers
                .iter()
                .filter(|s| s.imported.name != "default")
                .collect();
            let mut wrote = false;
            if has_default {
                if let Some(d) = specifiers.iter().find(|s| s.imported.name == "default") {
                    out.push_str(&d.local.name);
                    wrote = true;
                }
            }
            if let Some(ns) = namespace {
                if wrote {
                    out.push_str(", ");
                }
                out.push_str("* as ");
                out.push_str(&ns.name);
                wrote = true;
            }
            if !named.is_empty() {
                if wrote {
                    out.push_str(", ");
                }
                out.push_str("{ ");
                for (i, s) in named.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    if s.imported.name == s.local.name {
                        out.push_str(&s.local.name);
                    } else {
                        out.push_str(&s.imported.name);
                        out.push_str(" as ");
                        out.push_str(&s.local.name);
                    }
                }
                out.push_str(" }");
                wrote = true;
            }
            if wrote {
                out.push_str(" from ");
            }
            print_string_lit(&source.value, out);
            print_import_attributes(attributes, out);
            out.push_str(";\n");
        }
        Stmt::ExportNamedDeclaration {
            declaration,
            specifiers,
            source,
            attributes,
            ..
        } => {
            indent(level, out);
            out.push_str("export ");
            if let Some(decl) = declaration {
                // Print declaration without its own indent (already indented).
                let mut tmp = String::new();
                print_stmt(decl, 0, &mut tmp);
                out.push_str(tmp.trim_start());
            } else {
                out.push_str("{ ");
                for (i, s) in specifiers.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    if s.local.name == s.exported.name {
                        out.push_str(&s.local.name);
                    } else {
                        out.push_str(&s.local.name);
                        out.push_str(" as ");
                        out.push_str(&s.exported.name);
                    }
                }
                out.push_str(" }");
                if let Some(src) = source {
                    out.push_str(" from ");
                    print_string_lit(&src.value, out);
                    print_import_attributes(attributes, out);
                }
                out.push_str(";\n");
            }
        }
        Stmt::ExportDefaultDeclaration { declaration, .. } => {
            indent(level, out);
            out.push_str("export default ");
            match declaration.as_ref() {
                Stmt::FunctionDeclaration { .. } | Stmt::ClassDeclaration { .. } => {
                    let mut tmp = String::new();
                    print_stmt(declaration, 0, &mut tmp);
                    out.push_str(tmp.trim_start());
                }
                Stmt::Let {
                    init: Some(expr), ..
                } => {
                    // Synthetic let for `export default expr`
                    print_expr(expr, 0, out);
                    out.push_str(";\n");
                }
                other => {
                    let mut tmp = String::new();
                    print_stmt(other, 0, &mut tmp);
                    out.push_str(tmp.trim_start());
                }
            }
        }
        Stmt::ExportAllDeclaration {
            exported,
            source,
            attributes,
            ..
        } => {
            indent(level, out);
            out.push_str("export *");
            if let Some(name) = exported {
                out.push_str(" as ");
                out.push_str(&name.name);
            }
            out.push_str(" from ");
            print_string_lit(&source.value, out);
            print_import_attributes(attributes, out);
            out.push_str(";\n");
        }
        Stmt::TypeAlias {
            name,
            type_params,
            ty,
            ..
        } => {
            indent(level, out);
            out.push_str("type ");
            out.push_str(&name.name);
            print_type_params(type_params, out);
            out.push_str(" = ");
            print_type_ann(ty, out);
            out.push_str(";\n");
        }
        Stmt::ExternFunctionDeclaration {
            abi,
            name,
            params,
            return_type,
            ..
        } => {
            indent(level, out);
            out.push_str("extern ");
            print_string_lit(&abi.value, out);
            out.push_str(" function ");
            out.push_str(&name.name);
            print_param_list(params, out);
            if let Some(ret) = return_type {
                out.push_str(": ");
                print_type_ann(ret, out);
            }
            out.push_str(";\n");
        }
    }
}

/// Continue an else-if without re-indenting the `if`.
fn print_if_continued(stmt: &Stmt, level: usize, out: &mut String) {
    match stmt {
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            out.push_str("if (");
            print_expr(test, 0, out);
            out.push_str(") ");
            print_stmt_body(consequent, level, out);
            if let Some(alt) = alternate {
                if out.ends_with('\n') {
                    out.pop();
                }
                out.push_str(" else ");
                if matches!(alt.as_ref(), Stmt::If { .. }) {
                    print_if_continued(alt, level, out);
                } else {
                    print_stmt_body(alt, level, out);
                }
            }
        }
        other => print_stmt_body(other, level, out),
    }
}

fn print_stmt_body(stmt: &Stmt, level: usize, out: &mut String) {
    match stmt {
        Stmt::Block { body, .. } => {
            print_block_body(body, level, out);
            out.push('\n');
        }
        other => {
            // Wrap non-block bodies for stable multi-line style.
            out.push_str("{\n");
            print_stmt(other, level + 1, out);
            indent(level, out);
            out.push_str("}\n");
        }
    }
}

fn print_block_body(body: &[Stmt], level: usize, out: &mut String) {
    out.push_str("{\n");
    for s in body {
        print_stmt(s, level + 1, out);
    }
    indent(level, out);
    out.push('}');
}

pub(crate) fn print_function_body(body: &Stmt, level: usize, out: &mut String) {
    match body {
        Stmt::Block { body, .. } => {
            print_block_body(body, level, out);
        }
        other => {
            out.push_str("{\n");
            print_stmt(other, level + 1, out);
            indent(level, out);
            out.push('}');
        }
    }
}

fn print_try_block(stmt: &Stmt, level: usize, out: &mut String) {
    match stmt {
        Stmt::Block { body, .. } => print_block_body(body, level, out),
        other => {
            out.push_str("{\n");
            print_stmt(other, level + 1, out);
            indent(level, out);
            out.push('}');
        }
    }
}

fn print_for_init(stmt: &Stmt, out: &mut String) {
    match stmt {
        Stmt::Let {
            kind,
            binding,
            type_ann,
            init,
            ..
        } => {
            print_binding_kind(*kind, out);
            print_binding_pattern(binding, out);
            if let Some(ty) = type_ann {
                out.push_str(": ");
                print_type_ann(ty, out);
            }
            if let Some(init) = init {
                out.push_str(" = ");
                print_expr(init, 0, out);
            }
        }
        Stmt::Expression { expr, .. } => {
            print_expr(expr, 0, out);
        }
        other => {
            // Fallback: strip trailing semicolon/newline from full stmt print.
            let mut tmp = String::new();
            print_stmt(other, 0, &mut tmp);
            let t = tmp.trim_end_matches('\n').trim_end_matches(';');
            out.push_str(t);
        }
    }
}

fn print_for_in_of_left(stmt: &Stmt, out: &mut String) {
    match stmt {
        Stmt::Let {
            kind,
            binding,
            type_ann,
            init,
            ..
        } => {
            print_binding_kind(*kind, out);
            print_binding_pattern(binding, out);
            if let Some(ty) = type_ann {
                out.push_str(": ");
                print_type_ann(ty, out);
            }
            // for-in/of left should not have init in valid code; print if present.
            if let Some(init) = init {
                out.push_str(" = ");
                print_expr(init, 0, out);
            }
        }
        Stmt::Expression { expr, .. } => print_expr(expr, 0, out),
        other => {
            let mut tmp = String::new();
            print_stmt(other, 0, &mut tmp);
            let t = tmp.trim_end_matches('\n').trim_end_matches(';');
            out.push_str(t);
        }
    }
}

fn print_binding_kind(kind: BindingKind, out: &mut String) {
    match kind {
        BindingKind::Let => out.push_str("let "),
        BindingKind::Const => out.push_str("const "),
        BindingKind::Var => out.push_str("var "),
        BindingKind::Function => out.push_str("let "),
        BindingKind::Using => out.push_str("using "),
        BindingKind::AwaitUsing => out.push_str("await using "),
    }
}

pub(crate) fn print_binding_pattern(pat: &BindingPattern, out: &mut String) {
    match pat {
        BindingPattern::Ident(id) => out.push_str(&id.name),
        BindingPattern::Array { elements, .. } => {
            out.push('[');
            for (i, el) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        print_binding_pattern(binding, out);
                        if let Some(d) = default {
                            out.push_str(" = ");
                            print_expr(d, 0, out);
                        }
                    }
                    ArrayPatternElement::Rest(b) => {
                        out.push_str("...");
                        print_binding_pattern(b, out);
                    }
                }
            }
            out.push(']');
        }
        BindingPattern::Object { properties, .. } => {
            out.push_str("{ ");
            for (i, p) in properties.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        shorthand,
                        default,
                        ..
                    } => {
                        if *shorthand {
                            print_binding_pattern(binding, out);
                            if let Some(d) = default {
                                out.push_str(" = ");
                                print_expr(d, 0, out);
                            }
                        } else {
                            print_object_key(key, out);
                            out.push_str(": ");
                            print_binding_pattern(binding, out);
                            if let Some(d) = default {
                                out.push_str(" = ");
                                print_expr(d, 0, out);
                            }
                        }
                    }
                    ObjectPatternProp::Rest(b) => {
                        out.push_str("...");
                        print_binding_pattern(b, out);
                    }
                }
            }
            out.push_str(" }");
        }
        BindingPattern::Member(expr) => print_expr_inner(expr, PREC_ASSIGN, out),
    }
}

pub(crate) fn print_param_list(params: &[Param], out: &mut String) {
    out.push('(');
    for (i, p) in params.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        if p.rest {
            out.push_str("...");
        }
        print_binding_pattern(&p.binding, out);
        if let Some(ty) = &p.type_ann {
            out.push_str(": ");
            print_type_ann(ty, out);
        }
        if let Some(d) = &p.default {
            out.push_str(" = ");
            print_expr(d, 0, out);
        }
    }
    out.push(')');
}

fn print_import_attributes(attrs: &[ImportAttribute], out: &mut String) {
    if attrs.is_empty() {
        return;
    }
    out.push_str(" with { ");
    for (i, a) in attrs.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        match &a.key {
            ImportAttributeKey::Ident(id) => out.push_str(&id.name),
            ImportAttributeKey::String(s) => print_string_lit(&s.value, out),
        }
        out.push_str(": ");
        print_string_lit(&a.value.value, out);
    }
    out.push_str(" }");
}

pub(crate) fn print_class_body(elements: &[ClassElement], level: usize, out: &mut String) {
    out.push_str("{\n");
    for el in elements {
        match el {
            ClassElement::Constructor { params, body, .. } => {
                indent(level + 1, out);
                out.push_str("constructor");
                print_param_list(params, out);
                out.push(' ');
                print_function_body(body, level + 1, out);
                out.push('\n');
            }
            ClassElement::Method {
                key,
                params,
                body,
                is_static,
                is_async,
                is_generator,
                is_private,
                ..
            } => {
                indent(level + 1, out);
                if *is_static {
                    out.push_str("static ");
                }
                if *is_async {
                    out.push_str("async ");
                }
                if *is_generator {
                    out.push('*');
                }
                if *is_private {
                    out.push('#');
                }
                print_object_key(key, out);
                print_param_list(params, out);
                out.push(' ');
                print_function_body(body, level + 1, out);
                out.push('\n');
            }
            ClassElement::Accessor {
                kind,
                key,
                params,
                body,
                is_static,
                is_private,
                ..
            } => {
                indent(level + 1, out);
                if *is_static {
                    out.push_str("static ");
                }
                match kind {
                    AccessorKind::Get => out.push_str("get "),
                    AccessorKind::Set => out.push_str("set "),
                }
                if *is_private {
                    out.push('#');
                }
                print_object_key(key, out);
                print_param_list(params, out);
                out.push(' ');
                print_function_body(body, level + 1, out);
                out.push('\n');
            }
            ClassElement::Field {
                key,
                value,
                is_static,
                is_private,
                ..
            } => {
                indent(level + 1, out);
                if *is_static {
                    out.push_str("static ");
                }
                if *is_private {
                    out.push('#');
                }
                print_object_key(key, out);
                if let Some(v) = value {
                    out.push_str(" = ");
                    print_expr(v, 0, out);
                }
                out.push_str(";\n");
            }
            ClassElement::StaticBlock { body, .. } => {
                indent(level + 1, out);
                out.push_str("static ");
                print_function_body(body, level + 1, out);
                out.push('\n');
            }
        }
    }
    indent(level, out);
    out.push('}');
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Expr, Ident, NumberLit};
    use draconic_diagnostics::Span;

    fn ident(name: &str) -> Ident {
        Ident {
            name: name.into(),
            span: Span::dummy(),
        }
    }

    #[test]
    fn print_simple_let() {
        let program = Program {
            body: vec![Stmt::Let {
                kind: BindingKind::Let,
                binding: BindingPattern::Ident(ident("x")),
                type_ann: None,
                init: Some(Expr::Number(NumberLit {
                    raw: "1".into(),
                    span: Span::dummy(),
                })),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        assert_eq!(print_program(&program), "let x = 1;\n");
    }

    #[test]
    fn print_block_indent() {
        let program = Program {
            body: vec![Stmt::Block {
                body: vec![Stmt::Let {
                    kind: BindingKind::Const,
                    binding: BindingPattern::Ident(ident("y")),
                    type_ann: None,
                    init: Some(Expr::Number(NumberLit {
                        raw: "2".into(),
                        span: Span::dummy(),
                    })),
                    span: Span::dummy(),
                }],
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        assert_eq!(print_program(&program), "{\n  const y = 2;\n}\n");
    }
}
