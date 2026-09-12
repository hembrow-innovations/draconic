use std::collections::HashSet;

use draconic_ast::{AssignOp, BinaryOp, BindingKind};
use draconic_diagnostics::Diagnostic;
use draconic_ir::{
    Arg, ArrayElement, ArrayPatternEl, AssignTarget, Expr, Module, ObjectPatternEl, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt, UpdateTarget,
};

pub(super) fn emit_walk(module: &Module) -> Result<String, Diagnostic> {
    let seen = Seen::walk(module);
    seen.emit(module)
}

#[derive(Default)]
pub(super) struct Seen {
    pub(super) host: HashSet<&'static str>,
    pub(super) idents: HashSet<String>,
    pub(super) has_script: bool,
    pub(super) has_async_fn: bool,
    pub(super) has_generator: bool,
    pub(super) has_function: bool,
    pub(super) has_try: bool,
    pub(super) has_with: bool,
    pub(super) has_array: bool,
    pub(super) has_tagged: bool,
    pub(super) has_new_target: bool,
    pub(super) has_optional: bool,
    pub(super) has_nullish: bool,
    pub(super) has_spread_arg: bool,
    pub(super) has_instanceof: bool,
    pub(super) has_obj_pattern: bool,
    pub(super) has_arr_pattern: bool,
    pub(super) has_var: bool,
    pub(super) has_for_in_of: bool,
    pub(super) has_for_await: bool,
    pub(super) has_regexp: bool,
    pub(super) has_date_now: bool,
    pub(super) has_value_of: bool,
}

impl Seen {
    fn walk(module: &Module) -> Self {
        let mut seen = Self::default();
        for stmt in &module.body {
            walk_stmt(stmt, module, &mut seen);
        }
        seen
    }

    pub(super) fn ident(&self, name: &str) -> bool {
        self.idents.contains(name)
    }

    pub(super) fn host_note_prefix(&self, prefix: &str) -> bool {
        self.host_entries().any(|e| e.note.starts_with(prefix))
    }

    pub(super) fn host_note(&self, pred: impl Fn(&str) -> bool) -> bool {
        self.host_entries().any(|e| pred(e.note))
    }

    pub(super) fn host_only_note_prefix(&self, prefixes: &[&str]) -> bool {
        let mut any = false;
        for e in self.host_entries() {
            any = true;
            if !prefixes.iter().any(|p| e.note.starts_with(p)) {
                return false;
            }
        }
        any
    }

    fn host_entries(&self) -> impl Iterator<Item = &'static draconic_check::HostApiEntry> + '_ {
        self.host
            .iter()
            .copied()
            .filter_map(draconic_check::lookup_host_api)
    }

    fn emit(self, module: &Module) -> Result<String, Diagnostic> {
        if !self.host.is_empty() || self.has_date_now {
            return super::host_dispatch::emit_host(module, &self);
        }
        super::es_kind::emit_es(module, &self)
    }
}

fn walk_stmt(stmt: &Stmt, module: &Module, seen: &mut Seen) {
    match stmt {
        Stmt::Declare { init, kind, .. } => {
            if *kind == BindingKind::Var {
                seen.has_var = true;
            }
            if let Some(expr) = init {
                walk_expr(expr, module, seen);
            }
        }
        Stmt::DeclareArrayPattern {
            init,
            kind,
            elements,
            ..
        } => {
            if *kind == BindingKind::Var {
                seen.has_var = true;
            }
            seen.has_arr_pattern = true;
            walk_array_pattern(elements, module, seen);
            if let Some(expr) = init {
                walk_expr(expr, module, seen);
            }
        }
        Stmt::DeclareObjectPattern {
            init,
            kind,
            properties,
            ..
        } => {
            if *kind == BindingKind::Var {
                seen.has_var = true;
            }
            seen.has_obj_pattern = true;
            walk_object_pattern(properties, module, seen);
            if let Some(expr) = init {
                walk_expr(expr, module, seen);
            }
        }
        Stmt::AssignLeft { target } => walk_assign_target(target, module, seen),
        Stmt::Expr { expr } => walk_expr(expr, module, seen),
        Stmt::Block { body } => {
            seen.has_script = true;
            for s in body {
                walk_stmt(s, module, seen);
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            seen.has_script = true;
            walk_expr(test, module, seen);
            walk_stmt(consequent, module, seen);
            if let Some(alt) = alternate {
                walk_stmt(alt, module, seen);
            }
        }
        Stmt::While { test, body } | Stmt::DoWhile { body, test } => {
            seen.has_script = true;
            walk_expr(test, module, seen);
            walk_stmt(body, module, seen);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            seen.has_script = true;
            if let Some(i) = init {
                walk_stmt(i, module, seen);
            }
            if let Some(t) = test {
                walk_expr(t, module, seen);
            }
            if let Some(u) = update {
                walk_expr(u, module, seen);
            }
            walk_stmt(body, module, seen);
        }
        Stmt::ForIn { left, right, body } => {
            seen.has_script = true;
            seen.has_for_in_of = true;
            walk_stmt(left, module, seen);
            walk_expr(right, module, seen);
            walk_stmt(body, module, seen);
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
        } => {
            seen.has_script = true;
            seen.has_for_in_of = true;
            if *is_await {
                seen.has_for_await = true;
            }
            walk_stmt(left, module, seen);
            walk_expr(right, module, seen);
            walk_stmt(body, module, seen);
        }
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
        Stmt::Labeled { body, .. } => walk_stmt(body, module, seen),
        Stmt::Switch {
            discriminant,
            cases,
        } => {
            walk_expr(discriminant, module, seen);
            for c in cases {
                if let Some(t) = &c.test {
                    walk_expr(t, module, seen);
                }
                for s in &c.body {
                    walk_stmt(s, module, seen);
                }
            }
        }
        Stmt::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            seen.has_function = true;
            if *is_async {
                seen.has_async_fn = true;
            }
            if *is_generator {
                seen.has_generator = true;
            }
            walk_params(params, module, seen);
            for s in body {
                walk_stmt(s, module, seen);
            }
        }
        Stmt::ExternFunction { .. } => {}
        Stmt::Return { value } => {
            if let Some(v) = value {
                walk_expr(v, module, seen);
            }
        }
        Stmt::Throw { value } => {
            seen.has_try = true;
            walk_expr(value, module, seen);
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            seen.has_try = true;
            for s in block {
                walk_stmt(s, module, seen);
            }
            if let Some(p) = handler_param {
                walk_pattern(p, module, seen);
            }
            if let Some(h) = handler {
                for s in h {
                    walk_stmt(s, module, seen);
                }
            }
            if let Some(f) = finalizer {
                for s in f {
                    walk_stmt(s, module, seen);
                }
            }
        }
        Stmt::With { object, body } => {
            seen.has_with = true;
            walk_expr(object, module, seen);
            for s in body {
                walk_stmt(s, module, seen);
            }
        }
    }
}

fn walk_expr(expr: &Expr, module: &Module, seen: &mut Seen) {
    if let Some(entry) = crate::host_catalog::catalog_callee_in_module(expr, module) {
        seen.host.insert(entry.name);
    }
    match expr {
        Expr::Local { id, .. } => {
            if let Some(local) = module.locals.iter().find(|l| l.id == *id) {
                seen.idents.insert(local.name.clone());
            }
        }
        Expr::IdentName { name, .. } => {
            seen.idents.insert(name.clone());
        }
        Expr::Number { .. }
        | Expr::BigInt { .. }
        | Expr::String { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::Super { .. }
        | Expr::ImportMeta { .. } => {}
        Expr::RegExp { .. } => seen.has_regexp = true,
        Expr::NewTarget { .. } => seen.has_new_target = true,
        Expr::Template { expressions, .. } => {
            for e in expressions {
                walk_expr(e, module, seen);
            }
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            seen.has_tagged = true;
            walk_expr(tag, module, seen);
            for e in expressions {
                walk_expr(e, module, seen);
            }
        }
        Expr::ImportCall {
            source, options, ..
        } => {
            walk_expr(source, module, seen);
            if let Some(o) = options {
                walk_expr(o, module, seen);
            }
        }
        Expr::Unary { arg, .. } => walk_expr(arg, module, seen),
        Expr::Binary {
            left, op, right, ..
        } => {
            if matches!(*op, BinaryOp::Nullish) {
                seen.has_nullish = true;
            }
            if matches!(*op, BinaryOp::InstanceOf) {
                seen.has_instanceof = true;
            }
            walk_expr(left, module, seen);
            walk_expr(right, module, seen);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            walk_expr(test, module, seen);
            walk_expr(consequent, module, seen);
            walk_expr(alternate, module, seen);
        }
        Expr::Assign {
            target, op, value, ..
        } => {
            if matches!(
                *op,
                AssignOp::NullishEq | AssignOp::AndAndEq | AssignOp::OrOrEq
            ) {
                seen.has_nullish = true;
            }
            walk_assign_target(target, module, seen);
            walk_expr(value, module, seen);
        }
        Expr::Update { target, .. } => walk_update_target(target, module, seen),
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                seen.has_optional = true;
            }
            note_date_now(callee, args, module, seen);
            walk_expr(callee, module, seen);
            walk_args(args, module, seen);
        }
        Expr::New { callee, args, .. } => {
            walk_expr(callee, module, seen);
            walk_args(args, module, seen);
        }
        Expr::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            seen.has_function = true;
            if *is_async {
                seen.has_async_fn = true;
            }
            if *is_generator {
                seen.has_generator = true;
            }
            walk_params(params, module, seen);
            for s in body {
                walk_stmt(s, module, seen);
            }
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                walk_object_prop(p, module, seen);
            }
        }
        Expr::Array { elements, .. } => {
            seen.has_array = true;
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        walk_expr(e, module, seen);
                    }
                    ArrayElement::Elision => {}
                }
            }
        }
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            if *optional {
                seen.has_optional = true;
            }
            if let Expr::String { value, .. } = property.as_ref() {
                let key = value.to_string_lossy();
                if key == "valueOf" || key == "toString" {
                    seen.has_value_of = true;
                }
            }
            walk_expr(object, module, seen);
            walk_expr(property, module, seen);
        }
    }
}

fn note_date_now(callee: &Expr, args: &[Arg], module: &Module, seen: &mut Seen) {
    if !args.is_empty() {
        return;
    }
    let Expr::Member {
        object, property, ..
    } = callee
    else {
        return;
    };
    let Expr::String { value, .. } = property.as_ref() else {
        return;
    };
    if value.to_string_lossy() != "now" {
        return;
    }
    if crate::host_catalog::is_named_callee_in(object, "Date", Some(module))
        || ident_is(object, module, "Date")
    {
        seen.has_date_now = true;
    }
}

fn ident_is(expr: &Expr, module: &Module, want: &str) -> bool {
    match expr {
        Expr::IdentName { name, .. } => name == want,
        Expr::Local { id, .. } => module
            .locals
            .iter()
            .find(|l| l.id == *id)
            .is_some_and(|l| l.name == want),
        _ => false,
    }
}

fn walk_args(args: &[Arg], module: &Module, seen: &mut Seen) {
    for a in args {
        match a {
            Arg::Expr(e) => walk_expr(e, module, seen),
            Arg::Spread(e) => {
                seen.has_spread_arg = true;
                walk_expr(e, module, seen);
            }
        }
    }
}

fn walk_object_prop(prop: &ObjectProp, module: &Module, seen: &mut Seen) {
    match prop {
        ObjectProp::Property { key, value } | ObjectProp::Accessor { key, value, .. } => {
            if let ObjectPropKey::Static(name) = key {
                let n = name.to_string_lossy();
                if n == "valueOf" || n == "toString" {
                    seen.has_value_of = true;
                }
            }
            walk_prop_key(key, module, seen);
            walk_expr(value, module, seen);
        }
        ObjectProp::Spread(e) => walk_expr(e, module, seen),
    }
}

fn walk_prop_key(key: &ObjectPropKey, module: &Module, seen: &mut Seen) {
    if let ObjectPropKey::Computed(e) = key {
        walk_expr(e, module, seen);
    }
}

fn walk_assign_target(target: &AssignTarget, module: &Module, seen: &mut Seen) {
    match target {
        AssignTarget::Local(_) | AssignTarget::Name(_) => {}
        AssignTarget::Member {
            object, property, ..
        } => {
            walk_expr(object, module, seen);
            walk_expr(property, module, seen);
        }
        AssignTarget::Deref(e) => walk_expr(e, module, seen),
        AssignTarget::ArrayPattern { elements } => {
            seen.has_arr_pattern = true;
            walk_array_pattern(elements, module, seen);
        }
        AssignTarget::ObjectPattern { properties } => {
            seen.has_obj_pattern = true;
            walk_object_pattern(properties, module, seen);
        }
    }
}

fn walk_update_target(target: &UpdateTarget, module: &Module, seen: &mut Seen) {
    match target {
        UpdateTarget::Local(_) | UpdateTarget::Name(_) => {}
        UpdateTarget::Member {
            object, property, ..
        } => {
            walk_expr(object, module, seen);
            walk_expr(property, module, seen);
        }
    }
}

fn walk_params(params: &[Param], module: &Module, seen: &mut Seen) {
    for p in params {
        walk_pattern(&p.pattern, module, seen);
        if let Some(d) = &p.default {
            walk_expr(d, module, seen);
        }
    }
}

fn walk_pattern(pat: &Pattern, module: &Module, seen: &mut Seen) {
    match pat {
        Pattern::Local(_) | Pattern::Name(_) => {}
        Pattern::Member {
            object, property, ..
        } => {
            walk_expr(object, module, seen);
            walk_expr(property, module, seen);
        }
        Pattern::Array(els) => {
            seen.has_arr_pattern = true;
            walk_array_pattern(els, module, seen);
        }
        Pattern::Object(els) => {
            seen.has_obj_pattern = true;
            walk_object_pattern(els, module, seen);
        }
    }
}

fn walk_array_pattern(els: &[ArrayPatternEl], module: &Module, seen: &mut Seen) {
    for el in els {
        match el {
            ArrayPatternEl::Elision => {}
            ArrayPatternEl::Pattern { binding, default } => {
                walk_pattern(binding, module, seen);
                if let Some(d) = default {
                    walk_expr(d, module, seen);
                }
            }
            ArrayPatternEl::Rest(p) => walk_pattern(p, module, seen),
        }
    }
}

fn walk_object_pattern(els: &[ObjectPatternEl], module: &Module, seen: &mut Seen) {
    for el in els {
        match el {
            ObjectPatternEl::Prop {
                key,
                binding,
                default,
                ..
            } => {
                walk_prop_key(key, module, seen);
                walk_pattern(binding, module, seen);
                if let Some(d) = default {
                    walk_expr(d, module, seen);
                }
            }
            ObjectPatternEl::Rest(p) => walk_pattern(p, module, seen),
        }
    }
}
