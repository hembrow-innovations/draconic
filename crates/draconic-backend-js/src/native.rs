//! N04 / F08 JS policy: native-only IR hard-errors instead of silent JS.

use draconic_ast::UnaryOp;
use draconic_check::{
    extern_unsupported_on_js_diagnostic, host_api_unsupported_diagnostic, CompileTarget,
};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    ArrayPatternEl, AssignTarget, Expr, IrType, Module, ObjectPatternEl, Pattern, Stmt,
    UpdateTarget,
};

pub(crate) fn native_only_diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}

/// Hard-error free host API names that the H00.01 registry marks unavailable on js.
fn reject_host_api_name(name: &str) -> Result<(), Diagnostic> {
    if let Some(d) = host_api_unsupported_diagnostic(name, CompileTarget::Js, Span::dummy()) {
        return Err(d);
    }
    Ok(())
}

/// F08.01: `extern "C"` / FFI is native-only — hard-error on the js backend.
pub(crate) fn reject_extern_ffi(module: &Module) -> Result<(), Diagnostic> {
    if !module.has_extern_ffi {
        return Ok(());
    }
    Err(extern_unsupported_on_js_diagnostic("extern", Span::dummy()))
}

/// Reject IR that is native-only on the JS backend (N04).
pub(crate) fn reject_native_only(module: &Module) -> Result<(), Diagnostic> {
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
            left, right, body, ..
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
        Stmt::ExternFunction { .. } => {}
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
        Expr::IdentName { name, .. } => reject_host_api_name(name),
        Expr::Local { .. }
        | Expr::Number { .. }
        | Expr::BigInt { .. }
        | Expr::String { .. }
        | Expr::RegExp { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::NewTarget { .. }
        | Expr::ImportMeta { .. }
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
        Expr::Update { target, .. } => match target {
            UpdateTarget::Local(_) => Ok(()),
            UpdateTarget::Name(name) => reject_host_api_name(name),
            UpdateTarget::Member {
                object, property, ..
            } => {
                reject_native_only_expr(object)?;
                reject_native_only_expr(property)
            }
        },
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
        AssignTarget::Local(_) => Ok(()),
        AssignTarget::Name(name) => reject_host_api_name(name),
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
        Pattern::Local(_) => Ok(()),
        Pattern::Name(name) => reject_host_api_name(name),
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
