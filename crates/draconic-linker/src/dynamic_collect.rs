use std::path::{Path, PathBuf};

use draconic_ast::{
    Arg, ArrayElement, ArrowBody, ClassElement, Expr, ImportPhase, ObjectKey, ObjectProp, Stmt,
};
use draconic_diagnostics::Diagnostic;

use crate::path::resolve_specifier;

/// E19.84.06: collect resolved paths of string-literal `import.defer("…")` calls.
pub(crate) fn collect_dynamic_defer_targets(
    body: &[Stmt],
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    for stmt in body {
        collect_dynamic_defer_in_stmt(stmt, parent, out)?;
    }
    Ok(())
}

/// E19.84.08: collect resolved paths of string-literal evaluation-phase `import("…")`.
pub(crate) fn collect_dynamic_eval_import_targets(
    body: &[Stmt],
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    for stmt in body {
        collect_dynamic_eval_import_in_stmt(stmt, parent, out)?;
    }
    Ok(())
}

pub(crate) fn collect_dynamic_eval_import_in_stmt(
    stmt: &Stmt,
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    // Reuse the defer walker; only the ImportCall phase filter differs.
    collect_dynamic_import_phase_in_stmt(stmt, parent, out, ImportPhase::Evaluation)
}

pub(crate) fn collect_dynamic_import_phase_in_stmt(
    stmt: &Stmt,
    parent: &Path,
    out: &mut Vec<PathBuf>,
    phase_filter: ImportPhase,
) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Expression { expr, .. } => {
            collect_dynamic_import_phase_in_expr(expr, parent, out, phase_filter)?
        }
        Stmt::Let {
            init: Some(init), ..
        } => collect_dynamic_import_phase_in_expr(init, parent, out, phase_filter)?,
        Stmt::Block { body, .. } => {
            for s in body {
                collect_dynamic_import_phase_in_stmt(s, parent, out, phase_filter)?;
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            collect_dynamic_import_phase_in_expr(test, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_stmt(consequent, parent, out, phase_filter)?;
            if let Some(alt) = alternate {
                collect_dynamic_import_phase_in_stmt(alt, parent, out, phase_filter)?;
            }
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            collect_dynamic_import_phase_in_expr(test, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            if let Some(init) = init {
                collect_dynamic_import_phase_in_stmt(init, parent, out, phase_filter)?;
            }
            if let Some(test) = test {
                collect_dynamic_import_phase_in_expr(test, parent, out, phase_filter)?;
            }
            if let Some(update) = update {
                collect_dynamic_import_phase_in_expr(update, parent, out, phase_filter)?;
            }
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Stmt::ForIn {
            left, right, body, ..
        }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            collect_dynamic_import_phase_in_stmt(left, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_expr(right, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Stmt::Labeled { body, .. } => {
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?
        }
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            collect_dynamic_import_phase_in_expr(discriminant, parent, out, phase_filter)?;
            for c in cases {
                if let Some(test) = &c.test {
                    collect_dynamic_import_phase_in_expr(test, parent, out, phase_filter)?;
                }
                for s in &c.body {
                    collect_dynamic_import_phase_in_stmt(s, parent, out, phase_filter)?;
                }
            }
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            collect_dynamic_import_phase_in_stmt(block, parent, out, phase_filter)?;
            if let Some(handler) = handler {
                collect_dynamic_import_phase_in_stmt(handler, parent, out, phase_filter)?;
            }
            if let Some(finalizer) = finalizer {
                collect_dynamic_import_phase_in_stmt(finalizer, parent, out, phase_filter)?;
            }
        }
        Stmt::With { object, body, .. } => {
            collect_dynamic_import_phase_in_expr(object, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Stmt::Return {
            argument: Some(arg),
            ..
        }
        | Stmt::Throw { argument: arg, .. } => {
            collect_dynamic_import_phase_in_expr(arg, parent, out, phase_filter)?
        }
        Stmt::FunctionDeclaration { body, params, .. } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_import_phase_in_expr(default, parent, out, phase_filter)?;
                }
            }
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Stmt::ClassDeclaration {
            super_class, body, ..
        } => {
            if let Some(sc) = super_class {
                collect_dynamic_import_phase_in_expr(sc, parent, out, phase_filter)?;
            }
            for el in body {
                match el {
                    ClassElement::Constructor { body, params, .. }
                    | ClassElement::Method { body, params, .. }
                    | ClassElement::Accessor { body, params, .. } => {
                        for p in params {
                            if let Some(default) = &p.default {
                                collect_dynamic_import_phase_in_expr(
                                    default,
                                    parent,
                                    out,
                                    phase_filter,
                                )?;
                            }
                        }
                        collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
                    }
                    ClassElement::Field { key, value, .. } => {
                        if let ObjectKey::Computed(key) = key {
                            collect_dynamic_import_phase_in_expr(key, parent, out, phase_filter)?;
                        }
                        if let Some(value) = value {
                            collect_dynamic_import_phase_in_expr(value, parent, out, phase_filter)?;
                        }
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn collect_dynamic_import_phase_in_expr(
    expr: &Expr,
    parent: &Path,
    out: &mut Vec<PathBuf>,
    phase_filter: ImportPhase,
) -> Result<(), Diagnostic> {
    match expr {
        Expr::ImportCall {
            phase,
            source,
            options,
            ..
        } => {
            if *phase == phase_filter {
                if let Expr::String(lit) = source.as_ref() {
                    if let Some(spec) = lit.value.to_string_strict() {
                        let dep = resolve_specifier(parent, &spec, lit.span)?;
                        if !out.iter().any(|p| p == &dep) {
                            out.push(dep);
                        }
                    }
                }
            }
            collect_dynamic_import_phase_in_expr(source, parent, out, phase_filter)?;
            if let Some(options) = options {
                collect_dynamic_import_phase_in_expr(options, parent, out, phase_filter)?;
            }
        }
        Expr::Unary { arg, .. }
        | Expr::Update { arg, .. }
        | Expr::Paren { expr: arg, .. }
        | Expr::As { expr: arg, .. } => {
            collect_dynamic_import_phase_in_expr(arg, parent, out, phase_filter)?;
        }
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => {
            collect_dynamic_import_phase_in_expr(left, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_expr(right, parent, out, phase_filter)?;
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            collect_dynamic_import_phase_in_expr(test, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_expr(consequent, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_expr(alternate, parent, out, phase_filter)?;
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            collect_dynamic_import_phase_in_expr(callee, parent, out, phase_filter)?;
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => {
                        collect_dynamic_import_phase_in_expr(e, parent, out, phase_filter)?
                    }
                }
            }
        }
        Expr::MemberExpression {
            object, property, ..
        } => {
            collect_dynamic_import_phase_in_expr(object, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_expr(property, parent, out, phase_filter)?;
        }
        Expr::PrivateIn { object, .. } => {
            collect_dynamic_import_phase_in_expr(object, parent, out, phase_filter)?
        }
        Expr::ArrayExpression { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        collect_dynamic_import_phase_in_expr(e, parent, out, phase_filter)?
                    }
                    ArrayElement::Elision => {}
                }
            }
        }
        Expr::ObjectExpression { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { key, value, .. } => {
                        if let ObjectKey::Computed(key) = key {
                            collect_dynamic_import_phase_in_expr(key, parent, out, phase_filter)?;
                        }
                        collect_dynamic_import_phase_in_expr(value, parent, out, phase_filter)?;
                    }
                    ObjectProp::Accessor {
                        key, params, body, ..
                    } => {
                        if let ObjectKey::Computed(key) = key {
                            collect_dynamic_import_phase_in_expr(key, parent, out, phase_filter)?;
                        }
                        for p in params {
                            if let Some(default) = &p.default {
                                collect_dynamic_import_phase_in_expr(
                                    default,
                                    parent,
                                    out,
                                    phase_filter,
                                )?;
                            }
                        }
                        collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
                    }
                    ObjectProp::Spread { expr, .. } => {
                        collect_dynamic_import_phase_in_expr(expr, parent, out, phase_filter)?
                    }
                }
            }
        }
        Expr::TemplateLiteral { expressions, .. } => {
            for e in expressions {
                collect_dynamic_import_phase_in_expr(e, parent, out, phase_filter)?;
            }
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            collect_dynamic_import_phase_in_expr(tag, parent, out, phase_filter)?;
            for e in expressions {
                collect_dynamic_import_phase_in_expr(e, parent, out, phase_filter)?;
            }
        }
        Expr::FunctionExpression { params, body, .. } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_import_phase_in_expr(default, parent, out, phase_filter)?;
                }
            }
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Expr::ClassExpression {
            super_class, body, ..
        } => {
            if let Some(sc) = super_class {
                collect_dynamic_import_phase_in_expr(sc, parent, out, phase_filter)?;
            }
            for el in body {
                match el {
                    ClassElement::Constructor { body, params, .. }
                    | ClassElement::Method { body, params, .. }
                    | ClassElement::Accessor { body, params, .. } => {
                        for p in params {
                            if let Some(default) = &p.default {
                                collect_dynamic_import_phase_in_expr(
                                    default,
                                    parent,
                                    out,
                                    phase_filter,
                                )?;
                            }
                        }
                        collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
                    }
                    ClassElement::Field { key, value, .. } => {
                        if let ObjectKey::Computed(key) = key {
                            collect_dynamic_import_phase_in_expr(key, parent, out, phase_filter)?;
                        }
                        if let Some(value) = value {
                            collect_dynamic_import_phase_in_expr(value, parent, out, phase_filter)?;
                        }
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
                    }
                }
            }
        }
        Expr::ArrowFunction { params, body, .. } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_import_phase_in_expr(default, parent, out, phase_filter)?;
                }
            }
            match body {
                ArrowBody::Expr(e) => {
                    collect_dynamic_import_phase_in_expr(e, parent, out, phase_filter)?
                }
                ArrowBody::Block(b) => {
                    collect_dynamic_import_phase_in_stmt(b, parent, out, phase_filter)?
                }
            }
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn collect_dynamic_defer_in_stmt(
    stmt: &Stmt,
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Expression { expr, .. } => collect_dynamic_defer_in_expr(expr, parent, out)?,
        Stmt::Let {
            init: Some(init), ..
        } => collect_dynamic_defer_in_expr(init, parent, out)?,
        Stmt::Block { body, .. } => {
            for s in body {
                collect_dynamic_defer_in_stmt(s, parent, out)?;
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            collect_dynamic_defer_in_expr(test, parent, out)?;
            collect_dynamic_defer_in_stmt(consequent, parent, out)?;
            if let Some(alt) = alternate {
                collect_dynamic_defer_in_stmt(alt, parent, out)?;
            }
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            collect_dynamic_defer_in_expr(test, parent, out)?;
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            if let Some(init) = init {
                collect_dynamic_defer_in_stmt(init, parent, out)?;
            }
            if let Some(test) = test {
                collect_dynamic_defer_in_expr(test, parent, out)?;
            }
            if let Some(update) = update {
                collect_dynamic_defer_in_expr(update, parent, out)?;
            }
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Stmt::ForIn {
            left, right, body, ..
        }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            collect_dynamic_defer_in_stmt(left, parent, out)?;
            collect_dynamic_defer_in_expr(right, parent, out)?;
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Stmt::Labeled { body, .. } => collect_dynamic_defer_in_stmt(body, parent, out)?,
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            collect_dynamic_defer_in_expr(discriminant, parent, out)?;
            for c in cases {
                if let Some(test) = &c.test {
                    collect_dynamic_defer_in_expr(test, parent, out)?;
                }
                for s in &c.body {
                    collect_dynamic_defer_in_stmt(s, parent, out)?;
                }
            }
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            collect_dynamic_defer_in_stmt(block, parent, out)?;
            if let Some(handler) = handler {
                collect_dynamic_defer_in_stmt(handler, parent, out)?;
            }
            if let Some(finalizer) = finalizer {
                collect_dynamic_defer_in_stmt(finalizer, parent, out)?;
            }
        }
        Stmt::With { object, body, .. } => {
            collect_dynamic_defer_in_expr(object, parent, out)?;
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Stmt::Return {
            argument: Some(arg),
            ..
        }
        | Stmt::Throw { argument: arg, .. } => collect_dynamic_defer_in_expr(arg, parent, out)?,
        Stmt::FunctionDeclaration { body, params, .. } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_defer_in_expr(default, parent, out)?;
                }
            }
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Stmt::ClassDeclaration {
            super_class, body, ..
        } => {
            if let Some(sc) = super_class {
                collect_dynamic_defer_in_expr(sc, parent, out)?;
            }
            collect_dynamic_defer_in_class_els(body, parent, out)?;
        }
        _ => {}
    }
    Ok(())
}

pub(crate) fn collect_dynamic_defer_in_class_els(
    body: &[ClassElement],
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    for el in body {
        match el {
            ClassElement::Constructor { body, params, .. }
            | ClassElement::Method { body, params, .. }
            | ClassElement::Accessor { body, params, .. } => {
                for p in params {
                    if let Some(default) = &p.default {
                        collect_dynamic_defer_in_expr(default, parent, out)?;
                    }
                }
                collect_dynamic_defer_in_stmt(body, parent, out)?;
            }
            ClassElement::Field { key, value, .. } => {
                if let ObjectKey::Computed(key) = key {
                    collect_dynamic_defer_in_expr(key, parent, out)?;
                }
                if let Some(value) = value {
                    collect_dynamic_defer_in_expr(value, parent, out)?;
                }
            }
            ClassElement::StaticBlock { body, .. } => {
                collect_dynamic_defer_in_stmt(body, parent, out)?;
            }
        }
    }
    Ok(())
}

pub(crate) fn collect_dynamic_defer_in_expr(
    expr: &Expr,
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    match expr {
        Expr::ImportCall {
            phase,
            source,
            options,
            ..
        } => {
            if *phase == ImportPhase::Defer {
                if let Expr::String(lit) = source.as_ref() {
                    if let Some(spec) = lit.value.to_string_strict() {
                        let dep = resolve_specifier(parent, &spec, lit.span)?;
                        if !out.iter().any(|p| p == &dep) {
                            out.push(dep);
                        }
                    }
                }
            }
            collect_dynamic_defer_in_expr(source, parent, out)?;
            if let Some(options) = options {
                collect_dynamic_defer_in_expr(options, parent, out)?;
            }
        }
        Expr::Unary { arg, .. }
        | Expr::Update { arg, .. }
        | Expr::Paren { expr: arg, .. }
        | Expr::As { expr: arg, .. } => {
            collect_dynamic_defer_in_expr(arg, parent, out)?;
        }
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => {
            collect_dynamic_defer_in_expr(left, parent, out)?;
            collect_dynamic_defer_in_expr(right, parent, out)?;
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            collect_dynamic_defer_in_expr(test, parent, out)?;
            collect_dynamic_defer_in_expr(consequent, parent, out)?;
            collect_dynamic_defer_in_expr(alternate, parent, out)?;
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            collect_dynamic_defer_in_expr(callee, parent, out)?;
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => collect_dynamic_defer_in_expr(e, parent, out)?,
                }
            }
        }
        Expr::MemberExpression {
            object, property, ..
        } => {
            collect_dynamic_defer_in_expr(object, parent, out)?;
            collect_dynamic_defer_in_expr(property, parent, out)?;
        }
        Expr::PrivateIn { object, .. } => collect_dynamic_defer_in_expr(object, parent, out)?,
        Expr::ArrayExpression { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        collect_dynamic_defer_in_expr(e, parent, out)?
                    }
                    ArrayElement::Elision => {}
                }
            }
        }
        Expr::ObjectExpression { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { key, value, .. } => {
                        if let ObjectKey::Computed(key) = key {
                            collect_dynamic_defer_in_expr(key, parent, out)?;
                        }
                        collect_dynamic_defer_in_expr(value, parent, out)?;
                    }
                    ObjectProp::Accessor {
                        key, params, body, ..
                    } => {
                        if let ObjectKey::Computed(key) = key {
                            collect_dynamic_defer_in_expr(key, parent, out)?;
                        }
                        for p in params {
                            if let Some(default) = &p.default {
                                collect_dynamic_defer_in_expr(default, parent, out)?;
                            }
                        }
                        collect_dynamic_defer_in_stmt(body, parent, out)?;
                    }
                    ObjectProp::Spread { expr, .. } => {
                        collect_dynamic_defer_in_expr(expr, parent, out)?
                    }
                }
            }
        }
        Expr::TemplateLiteral { expressions, .. } => {
            for e in expressions {
                collect_dynamic_defer_in_expr(e, parent, out)?;
            }
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            collect_dynamic_defer_in_expr(tag, parent, out)?;
            for e in expressions {
                collect_dynamic_defer_in_expr(e, parent, out)?;
            }
        }
        Expr::FunctionExpression { params, body, .. } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_defer_in_expr(default, parent, out)?;
                }
            }
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Expr::ClassExpression {
            super_class, body, ..
        } => {
            if let Some(sc) = super_class {
                collect_dynamic_defer_in_expr(sc, parent, out)?;
            }
            collect_dynamic_defer_in_class_els(body, parent, out)?;
        }
        Expr::ArrowFunction { params, body, .. } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_defer_in_expr(default, parent, out)?;
                }
            }
            match body {
                ArrowBody::Expr(e) => collect_dynamic_defer_in_expr(e, parent, out)?,
                ArrowBody::Block(b) => collect_dynamic_defer_in_stmt(b, parent, out)?,
            }
        }
        _ => {}
    }
    Ok(())
}
