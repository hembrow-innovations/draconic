use draconic_ast::{BindingKind, BindingPattern, Stmt};
use draconic_diagnostics::Diagnostic;

use super::checker::Checker;
use super::{
    body_has_use_strict, catch_lexical_conflict, check_statement_list_early_errors,
    collect_var_declared_names_stmt, fn_params_of_expr, is_iteration_labelled_item,
    is_simple_parameter_list, params_contain_super, stmt_contains_super, GenericFnSig, Type,
};

impl Checker {
    pub(crate) fn check_stmt(
        &mut self,
        stmt: &Stmt,
        loop_depth: u32,
        switch_depth: u32,
        fn_depth: u32,
        labels: &mut Vec<(String, bool)>,
    ) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Expression { expr, .. } => {
                self.check_expr(expr)?;
                Ok(())
            }
            Stmt::TypeAlias { .. } => Ok(()),
            // F06.02: bind as function; params/return must be native ABI types.
            Stmt::ExternFunctionDeclaration {
                name,
                params,
                return_type,
                span,
                ..
            } => {
                if self.typecheck {
                    self.check_extern_function_declaration(name, params, return_type, *span)
                } else {
                    Ok(())
                }
            }
            Stmt::Let {
                kind,
                binding,
                type_ann,
                init,
                span,
                ..
            } => {
                // Bare `const` without init is rejected in the parser; for-in/of
                // left may be `const name` with no initializer.
                if self.typecheck && *kind == BindingKind::AwaitUsing && !self.in_async {
                    return Err(Diagnostic::new(
                        "await using is only valid in async functions and modules".to_string(),
                        *span,
                    ));
                }
                let ann_ty = match type_ann {
                    Some(ann) if self.typecheck => Some(self.resolve_type_ann(ann)?),
                    _ => None,
                };
                let init_ty = if let Some(init) = init {
                    self.check_expr(init)?
                } else {
                    Type::Any
                };
                let (ty, annotated) = if let Some(ann_ty) = ann_ty {
                    if let Some(init) = init {
                        self.require_assignable_expr(init_ty, ann_ty, init)?;
                    }
                    (ann_ty, true)
                } else {
                    (init_ty, false)
                };
                self.check_binding_pattern_annotated(binding, ty, annotated)?;
                // T07.01: record a call signature when a simple ident binding is
                // initialized with a function value (`let f = (a: number) => a;`).
                if let (BindingPattern::Ident(name), Some(init)) = (binding, init) {
                    if let Some(params) = fn_params_of_expr(init) {
                        if let Some(sig) = self.fn_sig_from_params(params) {
                            if let Some(id) = self
                                .symbols()
                                .iter()
                                .find(|s| s.span == name.span)
                                .map(|s| s.id)
                            {
                                self.fn_sigs[id.0 as usize] = Some(sig);
                            }
                        }
                    }
                }
                Ok(())
            }
            Stmt::Empty { .. } => Ok(()),
            Stmt::Block { body, .. } => {
                self.binder.push_scope();
                let result =
                    self.walk_stmt_list(body, false, loop_depth, switch_depth, fn_depth, labels);
                self.binder.pop_scope();
                result
            }
            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.check_expr(test)?;
                let (then_n, else_n) = self.typeof_narrow_facts(test);
                self.with_narrows(&then_n, |this| {
                    this.check_stmt(consequent, loop_depth, switch_depth, fn_depth, labels)
                })?;
                if let Some(alt) = alternate {
                    self.with_narrows(&else_n, |this| {
                        this.check_stmt(alt, loop_depth, switch_depth, fn_depth, labels)
                    })?;
                }
                Ok(())
            }
            Stmt::While { test, body, .. } => {
                self.check_expr(test)?;
                self.check_stmt(body, loop_depth + 1, switch_depth, fn_depth, labels)
            }
            Stmt::DoWhile { body, test, .. } => {
                self.check_stmt(body, loop_depth + 1, switch_depth, fn_depth, labels)?;
                self.check_expr(test)?;
                Ok(())
            }
            Stmt::For {
                init,
                test,
                update,
                body,
                ..
            } => {
                if let Some(Stmt::Let {
                    kind,
                    binding,
                    type_ann,
                    init: let_init,
                    span: let_span,
                    ..
                }) = init.as_deref()
                {
                    if matches!(
                        kind,
                        BindingKind::Let
                            | BindingKind::Const
                            | BindingKind::Using
                            | BindingKind::AwaitUsing
                    ) {
                        let mut bound = Vec::new();
                        binding.for_each_ident(&mut |id| {
                            bound.push((id.name.clone(), id.span));
                        });
                        let mut body_vars = Vec::new();
                        collect_var_declared_names_stmt(body, &mut body_vars);
                        for (name, span) in &bound {
                            if body_vars.iter().any(|(n, _)| n == name) {
                                return Err(Diagnostic::new(
                                    format!("duplicate declaration of `{name}`"),
                                    *span,
                                ));
                            }
                        }
                        self.binder.push_scope();
                        let result = (|| {
                            self.binder.declare_binding(binding, *kind)?;
                            self.sync_symbols();
                            self.check_stmt(
                                &Stmt::Let {
                                    kind: *kind,
                                    binding: binding.clone(),
                                    type_ann: type_ann.clone(),
                                    init: let_init.clone(),
                                    span: *let_span,
                                },
                                loop_depth,
                                switch_depth,
                                fn_depth,
                                labels,
                            )?;
                            if let Some(t) = test {
                                self.check_expr(t)?;
                            }
                            if let Some(u) = update {
                                self.check_expr(u)?;
                            }
                            self.check_stmt(body, loop_depth + 1, switch_depth, fn_depth, labels)
                        })();
                        self.binder.pop_scope();
                        return result;
                    }
                }
                if let Some(init) = init {
                    self.check_stmt(init, loop_depth, switch_depth, fn_depth, labels)?;
                }
                if let Some(t) = test {
                    self.check_expr(t)?;
                }
                if let Some(u) = update {
                    self.check_expr(u)?;
                }
                self.check_stmt(body, loop_depth + 1, switch_depth, fn_depth, labels)
            }
            Stmt::ForIn {
                left, right, body, ..
            } => self.check_for_in_of(
                left,
                right,
                body,
                true,
                false,
                None,
                loop_depth,
                switch_depth,
                fn_depth,
                labels,
            ),
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
                span,
            } => {
                if self.typecheck && *is_await && !self.in_async {
                    return Err(Diagnostic::new(
                        "for await is only valid in async functions and modules".to_string(),
                        *span,
                    ));
                }
                self.check_for_in_of(
                    left,
                    right,
                    body,
                    false,
                    *is_await,
                    Some(*span),
                    loop_depth,
                    switch_depth,
                    fn_depth,
                    labels,
                )
            }
            Stmt::Break { label, span } => {
                if self.typecheck {
                    if let Some(label) = label {
                        if !labels.iter().any(|(n, _)| n == &label.name) {
                            return Err(Diagnostic::new(
                                format!("Undefined label `{}`", label.name),
                                label.span,
                            ));
                        }
                    } else if loop_depth == 0 && switch_depth == 0 {
                        return Err(Diagnostic::new(
                            "Illegal break statement".to_string(),
                            *span,
                        ));
                    }
                }
                Ok(())
            }
            Stmt::Continue { label, span } => {
                if !self.typecheck {
                    Ok(())
                } else if let Some(label) = label {
                    match labels.iter().rev().find(|(n, _)| n == &label.name) {
                        Some((_, true)) => Ok(()),
                        Some((_, false)) => Err(Diagnostic::new(
                            format!(
                                "Undefined label `{}` (not an iteration statement)",
                                label.name
                            ),
                            label.span,
                        )),
                        None => Err(Diagnostic::new(
                            format!("Undefined label `{}`", label.name),
                            label.span,
                        )),
                    }
                } else if loop_depth == 0 {
                    Err(Diagnostic::new(
                        "Illegal continue statement".to_string(),
                        *span,
                    ))
                } else {
                    Ok(())
                }
            }
            Stmt::Labeled { label, body, span } => {
                if self.typecheck && labels.iter().any(|(n, _)| n == &label.name) {
                    return Err(Diagnostic::new(
                        format!("Label `{}` has already been declared", label.name),
                        *span,
                    ));
                }
                let is_iteration = is_iteration_labelled_item(body);
                labels.push((label.name.clone(), is_iteration));
                let result = self.check_stmt(body, loop_depth, switch_depth, fn_depth, labels);
                labels.pop();
                result
            }
            Stmt::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.check_expr(discriminant)?;
                self.binder.push_scope();
                let result = (|| {
                    let mut all_stmts = Vec::new();
                    for case in cases {
                        if let Some(test) = &case.test {
                            self.check_expr(test)?;
                        }
                        all_stmts.extend(case.body.iter());
                    }
                    check_statement_list_early_errors(
                        all_stmts.iter().copied(),
                        self.binder.strict,
                        false,
                    )?;
                    for stmt in &all_stmts {
                        self.binder.declare_list_item(stmt)?;
                    }
                    self.sync_symbols();
                    for case in cases {
                        for s in &case.body {
                            self.check_stmt(s, loop_depth, switch_depth + 1, fn_depth, labels)?;
                        }
                    }
                    Ok(())
                })();
                self.binder.pop_scope();
                result
            }
            Stmt::FunctionDeclaration {
                name,
                type_params,
                params,
                return_type,
                body,
                is_async,
                is_generator,
                span,
                ..
            } => {
                let prev_strict = self.binder.strict;
                if body_has_use_strict(body) {
                    if !is_simple_parameter_list(params) {
                        return Err(Diagnostic::new(
                            "\"use strict\" not allowed in function with non-simple parameter list"
                                .to_string(),
                            *span,
                        ));
                    }
                    self.binder.strict = true;
                }
                if self.binder.strict && (name.name == "eval" || name.name == "arguments") {
                    return Err(Diagnostic::new(
                        format!("binding `{}` is invalid in strict mode", name.name),
                        name.span,
                    ));
                }
                if params_contain_super(params) || stmt_contains_super(body) {
                    return Err(Diagnostic::new(
                        "function cannot contain super".to_string(),
                        *span,
                    ));
                }
                let prev_super = self.binder.super_allowed;
                self.binder.super_allowed = false;
                self.binder.push_scope_kind(true);
                let allow_sloppy_dups = !*is_async && !*is_generator;
                let result = (|| {
                    self.binder.bind_params(params, allow_sloppy_dups)?;
                    self.binder.install_arguments_object()?;
                    self.sync_symbols();
                    self.binder
                        .check_params_body_lexical_conflict(params, body)?;
                    // E19.49: undeclared function name (e.g. with-body before parse reject) → diagnostic.
                    let Some(id) = self
                        .symbols()
                        .iter()
                        .find(|s| s.span == name.span)
                        .map(|s| s.id)
                    else {
                        return Err(Diagnostic::new(
                            format!("function binding `{}` must be declared", name.name),
                            *span,
                        ));
                    };
                    let fn_ty = if type_params.is_empty() {
                        Type::Function
                    } else {
                        let sig = GenericFnSig {
                            type_params: type_params.iter().map(|p| p.name.name.clone()).collect(),
                            param_types: params.iter().map(|p| p.type_ann.clone()).collect(),
                            return_type: return_type.clone(),
                        };
                        let gid = self.generic_fns.len() as u32;
                        self.generic_fns.push(sig);
                        Type::GenericFn(gid)
                    };
                    self.symbol_types[id.0 as usize] = fn_ty;
                    let saved_env = self.type_param_env.clone();
                    if self.typecheck {
                        for tp in type_params {
                            if self.type_param_env.contains_key(&tp.name.name) {
                                return Err(Diagnostic::new(
                                    format!("duplicate type parameter `{}`", tp.name.name),
                                    tp.name.span,
                                ));
                            }
                            let pid = self.next_type_param_id;
                            self.next_type_param_id += 1;
                            self.type_param_env
                                .insert(tp.name.name.clone(), Type::TypeParam(pid));
                        }
                    }
                    self.check_params_await_yield(
                        params,
                        *is_async && *is_generator,
                        *is_generator,
                    )?;
                    if type_params.is_empty() {
                        if let Some(sig) = self.fn_sig_from_params(params) {
                            self.fn_sigs[id.0 as usize] = Some(sig);
                        }
                    }
                    let mut inner_labels = Vec::new();
                    let prev_async = self.in_async;
                    let prev_generator = self.in_generator;
                    let prev_ret = self.expected_return;
                    self.in_async = *is_async;
                    self.in_generator = *is_generator;
                    let ret_ty = if self.typecheck {
                        match return_type {
                            Some(ann) => Some(self.resolve_type_ann(ann)?),
                            None => None,
                        }
                    } else {
                        None
                    };
                    self.expected_return = ret_ty;
                    let body_result =
                        self.walk_function_body(body, fn_depth + 1, &mut inner_labels);
                    if body_result.is_ok() {
                        if let Some(ty) = ret_ty {
                            self.check_missing_return(body, ty)?;
                        }
                    }
                    self.in_async = prev_async;
                    self.in_generator = prev_generator;
                    self.expected_return = prev_ret;
                    self.type_param_env = saved_env;
                    body_result
                })();
                self.binder.pop_scope();
                self.binder.super_allowed = prev_super;
                self.binder.strict = prev_strict;
                result
            }
            Stmt::ClassDeclaration {
                name,
                super_class,
                body,
                ..
            } => {
                let id = self
                    .symbols()
                    .iter()
                    .find(|s| s.span == name.span)
                    .map(|s| s.id)
                    .ok_or_else(|| {
                        Diagnostic::new(
                            format!("undeclared class binding `{}`", name.name),
                            name.span,
                        )
                    })?;
                self.symbol_types[id.0 as usize] = Type::Function;
                if let Some(sc) = super_class {
                    self.check_expr(sc)?;
                }
                self.walk_class_elements(body, fn_depth)
            }
            Stmt::Return { argument, span } => {
                if self.typecheck && fn_depth == 0 {
                    return Err(Diagnostic::new(
                        "Illegal return statement".to_string(),
                        *span,
                    ));
                }
                let actual = if let Some(arg) = argument {
                    self.check_expr(arg)?
                } else {
                    // Bare `return;` yields undefined — treat as Any for coarse types.
                    Type::Any
                };
                if let Some(expected) = self.expected_return {
                    if let Some(arg) = argument {
                        self.require_assignable_expr(actual, expected, arg)?;
                    } else if expected != Type::Any {
                        return Err(Diagnostic::new(
                            format!(
                                "return type `{expected}` requires a value; bare `return` is not assignable"
                            ),
                            *span,
                        ));
                    }
                }
                Ok(())
            }
            Stmt::Throw { argument, .. } => {
                self.check_expr(argument)?;
                Ok(())
            }
            Stmt::Try {
                block,
                handler_param,
                handler,
                finalizer,
                ..
            } => {
                self.check_stmt(block, loop_depth, switch_depth, fn_depth, labels)?;
                if let Some(handler) = handler {
                    if let Some(param) = handler_param {
                        if let Some((name, span)) = catch_lexical_conflict(param, handler) {
                            return Err(Diagnostic::new(
                                format!("duplicate declaration of `{name}`"),
                                span,
                            ));
                        }
                    }
                    self.binder.push_scope();
                    let result = (|| {
                        if let Some(param) = handler_param {
                            if matches!(param, BindingPattern::Member(_)) {
                                return Err(Diagnostic::new(
                                    "member expression is not a valid catch binding".to_string(),
                                    param.span(),
                                ));
                            }
                            self.binder.declare_binding(param, BindingKind::Let)?;
                            self.sync_symbols();
                            self.check_binding_pattern(param, Type::Any)?;
                        }
                        self.check_stmt(handler, loop_depth, switch_depth, fn_depth, labels)
                    })();
                    self.binder.pop_scope();
                    result?;
                }
                if let Some(finalizer) = finalizer {
                    self.check_stmt(finalizer, loop_depth, switch_depth, fn_depth, labels)?;
                }
                Ok(())
            }
            Stmt::With { object, body, .. } => {
                self.check_expr(object)?;
                self.binder.with_depth += 1;
                let result = self.check_stmt(body, loop_depth, switch_depth, fn_depth, labels);
                self.binder.with_depth -= 1;
                result
            }
            Stmt::ImportDeclaration { span, .. }
            | Stmt::ExportNamedDeclaration { span, .. }
            | Stmt::ExportDefaultDeclaration { span, .. }
            | Stmt::ExportAllDeclaration { span, .. } => Err(Diagnostic::new(
                "import/export must be linked before bind/check".to_string(),
                *span,
            )),
        }
    }
}
