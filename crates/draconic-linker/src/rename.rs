use std::collections::{HashMap, HashSet};

use draconic_ast::{
    Arg, ArrayElement, ArrayPatternElement, ArrowBody, BindingPattern, ClassElement, Expr, Ident,
    ObjectKey, ObjectPatternProp, ObjectProp, Param, Stmt,
};

/// Nested scopes for rename: frame 0 is module top-level; deeper frames shadow.
pub(crate) struct ScopeStack {
    frames: Vec<HashSet<String>>,
}

impl ScopeStack {
    pub(crate) fn new() -> Self {
        Self {
            frames: vec![HashSet::new()],
        }
    }

    fn push(&mut self) {
        self.frames.push(HashSet::new());
    }

    fn pop(&mut self) {
        self.frames.pop();
    }

    fn declare_nested(&mut self, name: &str) {
        self.frames
            .last_mut()
            .expect("scope")
            .insert(name.to_string());
    }

    fn depth(&self) -> usize {
        self.frames.len()
    }

    fn is_shadowed(&self, name: &str) -> bool {
        for frame in self.frames.iter().skip(1).rev() {
            if frame.contains(name) {
                return true;
            }
        }
        false
    }
}

pub(crate) fn rename_object_key(
    key: &mut ObjectKey,
    renames: &HashMap<String, String>,
    scopes: &mut ScopeStack,
) {
    match key {
        ObjectKey::Computed(e) => rename_expr(e, renames, scopes),
        ObjectKey::Ident(_) | ObjectKey::String(_) => {}
    }
}

pub(crate) fn rename_ident(id: &mut Ident, renames: &HashMap<String, String>, scopes: &ScopeStack) {
    if scopes.is_shadowed(&id.name) {
        return;
    }
    if let Some(new_name) = renames.get(&id.name) {
        id.name = new_name.clone();
    }
}

pub(crate) fn rename_binding_decl(
    pat: &mut BindingPattern,
    renames: &HashMap<String, String>,
    scopes: &mut ScopeStack,
) {
    match pat {
        BindingPattern::Ident(id) => {
            if scopes.depth() == 1 {
                rename_ident(id, renames, scopes);
            } else {
                scopes.declare_nested(&id.name);
            }
        }
        BindingPattern::Member(expr) => rename_expr(expr, renames, scopes),
        BindingPattern::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        default,
                        ..
                    } => {
                        rename_object_key(key, renames, scopes);
                        rename_binding_decl(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ObjectPatternProp::Rest(binding) => {
                        rename_binding_decl(binding, renames, scopes);
                    }
                }
            }
        }
        BindingPattern::Array { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        rename_binding_decl(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        rename_binding_decl(binding, renames, scopes);
                    }
                }
            }
        }
    }
}

pub(crate) fn rename_stmt(
    stmt: &mut Stmt,
    renames: &HashMap<String, String>,
    scopes: &mut ScopeStack,
) {
    match stmt {
        Stmt::Expression { expr, .. } => rename_expr(expr, renames, scopes),
        Stmt::Let { binding, init, .. } => {
            if let Some(init) = init {
                rename_expr(init, renames, scopes);
            }
            rename_binding_decl(binding, renames, scopes);
        }
        Stmt::Empty { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::ImportDeclaration { .. }
        | Stmt::ExportNamedDeclaration { .. }
        | Stmt::ExportDefaultDeclaration { .. }
        | Stmt::ExportAllDeclaration { .. }
        | Stmt::TypeAlias { .. }
        | Stmt::ExternFunctionDeclaration { .. } => {}
        Stmt::Block { body, .. } => {
            scopes.push();
            // declare nested first for TDZ-ish list, then rename bodies
            for s in body.iter_mut() {
                predeclare_nested(s, scopes);
            }
            for s in body.iter_mut() {
                rename_stmt(s, renames, scopes);
            }
            scopes.pop();
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            rename_expr(test, renames, scopes);
            rename_stmt(consequent, renames, scopes);
            if let Some(alt) = alternate {
                rename_stmt(alt, renames, scopes);
            }
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { body, test, .. } => {
            rename_expr(test, renames, scopes);
            rename_stmt(body, renames, scopes);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            scopes.push();
            if let Some(init) = init {
                rename_stmt(init, renames, scopes);
            }
            if let Some(test) = test {
                rename_expr(test, renames, scopes);
            }
            if let Some(update) = update {
                rename_expr(update, renames, scopes);
            }
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Stmt::ForIn {
            left, right, body, ..
        }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            scopes.push();
            rename_stmt(left, renames, scopes);
            rename_expr(right, renames, scopes);
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Stmt::Labeled { body, .. } => rename_stmt(body, renames, scopes),
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            rename_expr(discriminant, renames, scopes);
            scopes.push();
            for case in cases.iter_mut() {
                if let Some(test) = &mut case.test {
                    rename_expr(test, renames, scopes);
                }
                for s in case.body.iter_mut() {
                    predeclare_nested(s, scopes);
                }
            }
            for case in cases.iter_mut() {
                for s in case.body.iter_mut() {
                    rename_stmt(s, renames, scopes);
                }
            }
            scopes.pop();
        }
        Stmt::FunctionDeclaration {
            name, params, body, ..
        } => {
            if scopes.depth() == 1 {
                rename_ident(name, renames, scopes);
            } else {
                scopes.declare_nested(&name.name);
            }
            scopes.push();
            rename_params(params, renames, scopes);
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Stmt::ClassDeclaration {
            name,
            super_class,
            body,
            ..
        } => {
            if scopes.depth() == 1 {
                rename_ident(name, renames, scopes);
            } else {
                scopes.declare_nested(&name.name);
            }
            if let Some(sc) = super_class {
                rename_expr(sc, renames, scopes);
            }
            for el in body.iter_mut() {
                match el {
                    ClassElement::Constructor { params, body, .. } => {
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                    ClassElement::Method {
                        key, params, body, ..
                    }
                    | ClassElement::Accessor {
                        key, params, body, ..
                    } => {
                        rename_object_key(key, renames, scopes);
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                    ClassElement::Field { key, value, .. } => {
                        rename_object_key(key, renames, scopes);
                        if let Some(v) = value {
                            rename_expr(v, renames, scopes);
                        }
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        rename_stmt(body, renames, scopes);
                    }
                }
            }
        }
        Stmt::Return { argument, .. } => {
            if let Some(arg) = argument {
                rename_expr(arg, renames, scopes);
            }
        }
        Stmt::Throw { argument, .. } => rename_expr(argument, renames, scopes),
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            ..
        } => {
            rename_stmt(block, renames, scopes);
            if let Some(handler) = handler {
                scopes.push();
                if let Some(param) = handler_param {
                    rename_binding_decl(param, renames, scopes);
                }
                rename_stmt(handler, renames, scopes);
                scopes.pop();
            }
            if let Some(finalizer) = finalizer {
                rename_stmt(finalizer, renames, scopes);
            }
        }
        Stmt::With { object, body, .. } => {
            rename_expr(object, renames, scopes);
            rename_stmt(body, renames, scopes);
        }
    }
}

pub(crate) fn predeclare_nested(stmt: &Stmt, scopes: &mut ScopeStack) {
    if scopes.depth() == 1 {
        return;
    }
    match stmt {
        Stmt::Let { binding, .. } => {
            binding.for_each_ident(&mut |id| scopes.declare_nested(&id.name));
        }
        Stmt::FunctionDeclaration { name, .. } | Stmt::ClassDeclaration { name, .. } => {
            scopes.declare_nested(&name.name);
        }
        _ => {}
    }
}

pub(crate) fn rename_params(
    params: &mut [Param],
    renames: &HashMap<String, String>,
    scopes: &mut ScopeStack,
) {
    for p in params.iter_mut() {
        rename_binding_decl(&mut p.binding, renames, scopes);
        if let Some(default) = &mut p.default {
            rename_expr(default, renames, scopes);
        }
    }
}

pub(crate) fn rename_expr(
    expr: &mut Expr,
    renames: &HashMap<String, String>,
    scopes: &mut ScopeStack,
) {
    match expr {
        Expr::Ident(id) => rename_ident(id, renames, scopes),
        Expr::Number(_)
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
            rename_expr(source, renames, scopes);
            if let Some(opts) = options {
                rename_expr(opts, renames, scopes);
            }
        }
        Expr::TemplateLiteral { expressions, .. } => {
            for e in expressions {
                rename_expr(e, renames, scopes);
            }
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            rename_expr(tag, renames, scopes);
            for e in expressions {
                rename_expr(e, renames, scopes);
            }
        }
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } | Expr::Paren { expr: arg, .. } => {
            rename_expr(arg, renames, scopes)
        }
        Expr::As { expr, .. } => rename_expr(expr, renames, scopes),
        Expr::Binary { left, right, .. } => {
            rename_expr(left, renames, scopes);
            rename_expr(right, renames, scopes);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            rename_expr(test, renames, scopes);
            rename_expr(consequent, renames, scopes);
            rename_expr(alternate, renames, scopes);
        }
        Expr::Assign { target, value, .. } => {
            rename_expr(target, renames, scopes);
            rename_expr(value, renames, scopes);
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            rename_expr(callee, renames, scopes);
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => rename_expr(e, renames, scopes),
                }
            }
        }
        Expr::FunctionExpression {
            name, params, body, ..
        } => {
            scopes.push();
            if let Some(name) = name {
                scopes.declare_nested(&name.name);
            }
            rename_params(params, renames, scopes);
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Expr::ClassExpression {
            name,
            super_class,
            body,
            ..
        } => {
            scopes.push();
            if let Some(name) = name {
                scopes.declare_nested(&name.name);
            }
            if let Some(sc) = super_class {
                rename_expr(sc, renames, scopes);
            }
            for el in body.iter_mut() {
                match el {
                    ClassElement::Constructor { params, body, .. } => {
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                    ClassElement::Method {
                        key, params, body, ..
                    }
                    | ClassElement::Accessor {
                        key, params, body, ..
                    } => {
                        rename_object_key(key, renames, scopes);
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                    ClassElement::Field { key, value, .. } => {
                        rename_object_key(key, renames, scopes);
                        if let Some(v) = value {
                            rename_expr(v, renames, scopes);
                        }
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        rename_stmt(body, renames, scopes);
                    }
                }
            }
            scopes.pop();
        }
        Expr::ArrowFunction { params, body, .. } => {
            scopes.push();
            rename_params(params, renames, scopes);
            match body {
                ArrowBody::Expr(e) => rename_expr(e, renames, scopes),
                ArrowBody::Block(b) => rename_stmt(b, renames, scopes),
            }
            scopes.pop();
        }
        Expr::ObjectExpression { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key,
                        value,
                        shorthand,
                        ..
                    } => {
                        match key {
                            ObjectKey::Computed(e) => rename_expr(e, renames, scopes),
                            ObjectKey::Ident(id) if *shorthand => rename_ident(id, renames, scopes),
                            ObjectKey::Ident(_) | ObjectKey::String(_) => {}
                        }
                        rename_expr(value, renames, scopes);
                    }
                    ObjectProp::Accessor {
                        key, params, body, ..
                    } => {
                        match key {
                            ObjectKey::Computed(e) => rename_expr(e, renames, scopes),
                            ObjectKey::Ident(_) | ObjectKey::String(_) => {}
                        }
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                    ObjectProp::Spread { expr, .. } => rename_expr(expr, renames, scopes),
                }
            }
        }
        Expr::ArrayExpression { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        rename_expr(e, renames, scopes)
                    }
                    ArrayElement::Elision => {}
                }
            }
        }
        Expr::ArrayPattern { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        rename_binding_pattern_use(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        rename_binding_pattern_use(binding, renames, scopes)
                    }
                }
            }
        }
        Expr::ObjectPattern { properties, .. } => {
            for p in properties {
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        default,
                        ..
                    } => {
                        rename_object_key(key, renames, scopes);
                        rename_binding_pattern_use(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ObjectPatternProp::Rest(binding) => {
                        rename_binding_pattern_use(binding, renames, scopes)
                    }
                }
            }
        }
        Expr::MemberExpression {
            object,
            property,
            computed,
            ..
        } => {
            rename_expr(object, renames, scopes);
            if *computed {
                rename_expr(property, renames, scopes);
            }
            // Non-computed property name is not a variable reference.
        }
        Expr::PrivateIn { object, .. } => rename_expr(object, renames, scopes),
    }
}

pub(crate) fn rename_binding_pattern_use(
    pat: &mut BindingPattern,
    renames: &HashMap<String, String>,
    scopes: &mut ScopeStack,
) {
    match pat {
        BindingPattern::Ident(id) => rename_ident(id, renames, scopes),
        BindingPattern::Member(expr) => rename_expr(expr, renames, scopes),
        BindingPattern::Array { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        rename_binding_pattern_use(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        rename_binding_pattern_use(binding, renames, scopes)
                    }
                }
            }
        }
        BindingPattern::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        default,
                        ..
                    } => {
                        rename_object_key(key, renames, scopes);
                        rename_binding_pattern_use(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ObjectPatternProp::Rest(binding) => {
                        rename_binding_pattern_use(binding, renames, scopes)
                    }
                }
            }
        }
    }
}
