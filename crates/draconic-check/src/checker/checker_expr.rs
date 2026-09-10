use draconic_ast::{
    Arg, ArrayElement, ArrayPatternElement, ArrowBody, BindingKind, Expr, ObjectKey,
    ObjectPatternProp, ObjectProp, UnaryOp,
};
use draconic_diagnostics::{codes, Diagnostic};

use super::Checker;
use crate::host_api;
use crate::{
    body_has_use_strict, expr_contains_super, expr_has_optional_chain, format_type_full,
    is_simple_parameter_list, params_contain_super, params_contain_super_call, peel_parens,
    stmt_contains_super, stmt_contains_super_call, strict_forbidden_assign_target, Type,
};

impl Checker {
    pub(crate) fn check_expr(&mut self, expr: &Expr) -> Result<Type, Diagnostic> {
        let ty = match expr {
            Expr::Number(n) => {
                self.record(n.span, Type::Number);
                Type::Number
            }
            Expr::BigInt(n) => {
                self.record(n.span, Type::BigInt);
                Type::BigInt
            }
            Expr::String(s) => {
                self.record(s.span, Type::String);
                Type::String
            }
            Expr::RegExp { span, .. } => {
                // RegExp instance (typeof "object"); not modeled as a distinct type yet.
                self.record(*span, Type::Any);
                Type::Any
            }
            Expr::TemplateLiteral {
                expressions, span, ..
            } => {
                for e in expressions {
                    self.check_expr(e)?;
                }
                self.record(*span, Type::String);
                Type::String
            }
            Expr::TaggedTemplate {
                tag,
                expressions,
                span,
                ..
            } => {
                self.check_expr(tag)?;
                for e in expressions {
                    self.check_expr(e)?;
                }
                // Tag return type is not modeled yet; treat as any (like untyped Call).
                self.record(*span, Type::Any);
                Type::Any
            }
            Expr::Boolean { span, .. } => {
                self.record(*span, Type::Boolean);
                Type::Boolean
            }
            Expr::Null { span } => {
                self.record(*span, Type::Null);
                Type::Null
            }
            Expr::This { span } => {
                self.record(*span, Type::Any);
                Type::Any
            }
            Expr::Super { span } => {
                self.record(*span, Type::Any);
                Type::Any
            }
            Expr::NewTarget { span } => {
                self.record(*span, Type::Any);
                Type::Any
            }
            Expr::ImportMeta { span } => {
                self.record(*span, Type::Any);
                Type::Any
            }
            Expr::ImportCall {
                source,
                options,
                span,
                ..
            } => {
                self.check_expr(source)?;
                if let Some(opts) = options {
                    self.check_expr(opts)?;
                }
                // Returns a Promise; untyped JS surface uses Any.
                self.record(*span, Type::Any);
                Type::Any
            }
            Expr::Ident(id) => {
                self.binder.bind_ident_use(id)?;
                if let Some(sym) = self.resolve_span(id.span) {
                    let ty = self.symbol_types[sym.0 as usize];
                    self.record(id.span, ty);
                    ty
                } else {
                    // Free / with-chain name (Object Environment).
                    // H00.01: free host API refs hard-error when unavailable on target.
                    if let Some(target) = self.host_target {
                        if let Some(d) = host_api::unsupported_diagnostic(&id.name, target, id.span)
                        {
                            return Err(d);
                        }
                    }
                    self.record(id.span, Type::Any);
                    Type::Any
                }
            }
            Expr::Paren { expr: inner, span } => {
                let ty = self.check_expr(inner)?;
                self.record(*span, ty);
                ty
            }
            Expr::As {
                expr: inner,
                ty: ann,
                span,
            } => {
                let from = self.check_expr(inner)?;
                if !self.typecheck {
                    self.record(*span, from);
                    from
                } else {
                    let to = self.resolve_type_ann(ann)?;
                    if !self.is_assignable(from, to) && !from.is_dual_world_boundary(to) {
                        let from_s =
                            format_type_full(from, &self.shapes, &self.unions, &self.intersections);
                        let to_s =
                            format_type_full(to, &self.shapes, &self.unions, &self.intersections);
                        return Err(Diagnostic::new(
                            format!(
                                "cannot convert type `{from_s}` to `{to_s}` across dual-worlds boundary"
                            ),
                            *span,
                        ));
                    }
                    self.record(*span, to);
                    to
                }
            }
            Expr::Unary { op, arg, span } => {
                // E19.39: `delete IdentifierReference` is early SyntaxError in strict mode
                // (including parenthesized forms: `delete ((id))`).
                if matches!(op, UnaryOp::Delete) && self.binder.strict {
                    if matches!(peel_parens(arg), Expr::Ident(_)) {
                        return Err(Diagnostic::new(
                            "cannot delete unqualified identifier in strict mode".to_string(),
                            *span,
                        ));
                    }
                }
                if self.typecheck && *op == UnaryOp::Await && !self.in_async {
                    return Err(Diagnostic::new(
                        "await is only valid in async functions and modules".to_string(),
                        *span,
                    ));
                }
                if self.typecheck
                    && matches!(op, UnaryOp::Yield | UnaryOp::YieldStar)
                    && !self.in_generator
                {
                    return Err(Diagnostic::new(
                        "yield is only valid in generator functions".to_string(),
                        *span,
                    ));
                }
                let arg_ty = self.check_expr(arg)?;
                let ty = self.check_unary(*op, arg_ty, *span)?;
                self.record(*span, ty);
                ty
            }
            Expr::Binary {
                left,
                op,
                right,
                span,
            } => {
                let left_ty = self.check_expr(left)?;
                let right_ty = self.check_expr(right)?;
                let ty = self.check_binary(*op, left_ty, right_ty, *span, left, right)?;
                self.record(*span, ty);
                ty
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                span,
            } => {
                self.check_expr(test)?;
                let cons_ty = self.check_expr(consequent)?;
                let alt_ty = self.check_expr(alternate)?;
                let ty = if cons_ty == alt_ty {
                    cons_ty
                } else if cons_ty == Type::Any || alt_ty == Type::Any {
                    Type::Any
                } else {
                    Type::Any
                };
                self.record(*span, ty);
                ty
            }
            Expr::Assign {
                target,
                op,
                value,
                span,
            } => {
                // E19.49: strict mode — `eval`/`arguments` are not valid simple assignment targets.
                if self.binder.strict {
                    if let Some((name, span)) = strict_forbidden_assign_target(target) {
                        return Err(Diagnostic::new(
                            format!("cannot assign to `{name}` in strict mode"),
                            span,
                        ));
                    }
                }
                let value_ty = self.check_expr(value)?;
                // E19.60: peel cover parentheses so `(id) = v` is a simple assignment target.
                match peel_parens(target.as_ref()) {
                    Expr::Ident(id) => {
                        self.binder.bind_ident_use(id)?;
                        let Some(sym) = self.resolve_span(id.span) else {
                            // Free / with-chain assign target.
                            self.record(id.span, Type::Any);
                            self.record(*span, value_ty);
                            return Ok(value_ty);
                        };
                        // E19.57 / E19.60: const/using/function-name PutValue is runtime
                        // TypeError (or silent); do not compile-reject.
                        let kind = self.symbol(sym).kind;
                        let left_ty = self.symbol_types[sym.0 as usize];
                        let result_ty = if let Some(bin_op) = op.binary_op() {
                            self.check_binary(bin_op, left_ty, value_ty, *span, target, value)?
                        } else {
                            value_ty
                        };
                        // Immutable bindings keep their static type; assignment does not stick.
                        let immutable = matches!(
                            kind,
                            BindingKind::Const
                                | BindingKind::Using
                                | BindingKind::AwaitUsing
                                | BindingKind::Function
                        );
                        if !immutable {
                            if left_ty == Type::Any {
                                self.symbol_types[sym.0 as usize] = result_ty;
                            } else {
                                // T07.05: fresh object literal vs annotated strict shape.
                                if let Some(diag) = self.excess_prop_diag(value, left_ty) {
                                    return Err(diag);
                                }
                                if !self.is_assignable(result_ty, left_ty) {
                                    // E19.12 / E19.48: untyped JS assign (simple + compound) may
                                    // replace an inferred binding type (ToNumber/ToString at runtime
                                    // for compound; plain store for simple). Annotated + native stay
                                    // strict (with number-literal contextual typing for natives).
                                    let annotated = self.symbol_annotated[sym.0 as usize];
                                    let native =
                                        left_ty.is_native_world() || result_ty.is_native_world();
                                    if annotated || native {
                                        self.require_assignable_expr(result_ty, left_ty, value)?;
                                    } else {
                                        self.symbol_types[sym.0 as usize] = result_ty;
                                    }
                                }
                            }
                        }
                        self.record(id.span, self.symbol_types[sym.0 as usize]);
                        self.record(*span, result_ty);
                        result_ty
                    }
                    Expr::MemberExpression {
                        object,
                        property,
                        computed,
                        span: mspan,
                        ..
                    } => {
                        // E19.58: OptionalExpression is not a valid AssignmentTarget.
                        if expr_has_optional_chain(target) {
                            return Err(Diagnostic::new(
                                "invalid assignment target".to_string(),
                                *mspan,
                            ));
                        }
                        // Property write: object + key are checked; result is the assigned value
                        // (simple `=`) or the compound binary result (`op=`).
                        let obj_ty = self.check_expr(object)?;
                        let left_ty = if *computed {
                            self.check_expr(property)?;
                            if let Some(idx) = Self::const_index_key(property) {
                                self.prop_type(obj_ty, &idx).unwrap_or(Type::Any)
                            } else {
                                Type::Any
                            }
                        } else if let Expr::Ident(id) = property.as_ref() {
                            self.member_prop_type(obj_ty, &id.name, id.span)?
                        } else {
                            Type::Any
                        };
                        let result_ty = if let Some(bin_op) = op.binary_op() {
                            self.check_binary(bin_op, left_ty, value_ty, *span, target, value)?
                        } else {
                            value_ty
                        };
                        self.record(*span, result_ty);
                        result_ty
                    }
                    // E19.58: optional call is not a valid AssignmentTarget.
                    Expr::Call {
                        optional: true,
                        span: cspan,
                        ..
                    } => {
                        return Err(Diagnostic::new(
                            "invalid assignment target".to_string(),
                            *cspan,
                        ));
                    }
                    // N03.03: `*p = v` store through native pointer.
                    Expr::Unary {
                        op: UnaryOp::Deref,
                        arg,
                        ..
                    } => {
                        if op.binary_op().is_some() {
                            return Err(Diagnostic::new(
                                "compound assignment through pointer not yet supported".to_string(),
                                *span,
                            ));
                        }
                        let ptr_ty = self.check_expr(arg)?;
                        let Type::Ptr(n) = ptr_ty else {
                            return Err(Diagnostic::new(
                                format!("cannot assign through type `{ptr_ty}` (pointer required)"),
                                *span,
                            ));
                        };
                        let dest = Type::Native(n);
                        self.require_assignable_expr(value_ty, dest, value)?;
                        // Contextual: number literal → native pointee type.
                        self.record(*span, dest);
                        dest
                    }
                    Expr::ArrayPattern { elements, .. } => {
                        if op.binary_op().is_some() {
                            return Err(Diagnostic::new(
                                "compound assignment to array pattern not supported".to_string(),
                                *span,
                            ));
                        }
                        for el in elements {
                            match el {
                                ArrayPatternElement::Elision => {}
                                ArrayPatternElement::Pattern { binding, default } => {
                                    self.check_assign_pattern(binding, *span)?;
                                    if let Some(def) = default {
                                        self.check_expr(def)?;
                                    }
                                }
                                ArrayPatternElement::Rest(binding) => {
                                    self.check_assign_pattern(binding, *span)?;
                                }
                            }
                        }
                        self.record(*span, value_ty);
                        value_ty
                    }
                    Expr::ObjectPattern { properties, .. } => {
                        if op.binary_op().is_some() {
                            return Err(Diagnostic::new(
                                "compound assignment to object pattern not supported".to_string(),
                                *span,
                            ));
                        }
                        for p in properties {
                            match p {
                                ObjectPatternProp::Prop {
                                    key,
                                    binding,
                                    default,
                                    ..
                                } => {
                                    self.check_object_key(key)?;
                                    self.check_assign_pattern(binding, *span)?;
                                    if let Some(def) = default {
                                        self.check_expr(def)?;
                                    }
                                }
                                ObjectPatternProp::Rest(binding) => {
                                    self.check_assign_pattern(binding, *span)?;
                                }
                            }
                        }
                        self.record(*span, value_ty);
                        value_ty
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            "invalid assignment target".to_string(),
                            *span,
                        ));
                    }
                }
            }
            Expr::ArrayPattern { span, .. } => {
                return Err(Diagnostic::new(
                    "array pattern cannot be used as a value".to_string(),
                    *span,
                ));
            }
            Expr::ObjectPattern { span, .. } => {
                return Err(Diagnostic::new(
                    "object pattern cannot be used as a value".to_string(),
                    *span,
                ));
            }
            Expr::Update { arg, span, .. } => {
                // E19.49: strict mode — `eval`/`arguments` are not valid update targets.
                if self.binder.strict {
                    if let Some((name, span)) = strict_forbidden_assign_target(arg) {
                        return Err(Diagnostic::new(
                            format!("cannot assign to `{name}` in strict mode"),
                            span,
                        ));
                    }
                }
                // E19.60: peel cover parentheses so `(id)++` is a valid update target.
                match peel_parens(arg.as_ref()) {
                    Expr::Ident(id) => {
                        self.binder.bind_ident_use(id)?;
                        let Some(sym) = self.resolve_span(id.span) else {
                            self.record(id.span, Type::Any);
                            self.record(*span, Type::Number);
                            return Ok(Type::Number);
                        };
                        // E19.57 / E19.60: const/using/function-name PutValue is runtime
                        // TypeError (or silent); do not compile-reject.
                        let kind = self.symbol(sym).kind;
                        let left_ty = self.symbol_types[sym.0 as usize];
                        let out = self.check_update_operand(left_ty, *span)?;
                        let immutable = matches!(
                            kind,
                            BindingKind::Const
                                | BindingKind::Using
                                | BindingKind::AwaitUsing
                                | BindingKind::Function
                        );
                        if !immutable && left_ty == Type::Any {
                            self.symbol_types[sym.0 as usize] = out;
                        }
                        self.record(id.span, out);
                        self.record(*span, out);
                        return Ok(out);
                    }
                    // E19.13: property ++/-- (ToNumber via valueOf/toString at runtime).
                    Expr::MemberExpression {
                        object,
                        property,
                        computed,
                        span: mspan,
                        ..
                    } => {
                        // E19.58: OptionalExpression is not a valid update target.
                        if expr_has_optional_chain(arg) {
                            return Err(Diagnostic::new(
                                "invalid update target".to_string(),
                                *mspan,
                            ));
                        }
                        let obj_ty = self.check_expr(object)?;
                        let left_ty = if *computed {
                            self.check_expr(property)?;
                            if let Some(idx) = Self::const_index_key(property) {
                                self.prop_type(obj_ty, &idx).unwrap_or(Type::Any)
                            } else {
                                Type::Any
                            }
                        } else if let Expr::Ident(id) = property.as_ref() {
                            self.member_prop_type(obj_ty, &id.name, id.span)?
                        } else {
                            Type::Any
                        };
                        let out = self.check_update_operand(left_ty, *span)?;
                        self.record(*span, out);
                        return Ok(out);
                    }
                    // E19.58: optional call is not a valid update target.
                    Expr::Call {
                        optional: true,
                        span: cspan,
                        ..
                    } => {
                        return Err(Diagnostic::new("invalid update target".to_string(), *cspan));
                    }
                    _ => {
                        return Err(Diagnostic::new("invalid update target".to_string(), *span));
                    }
                }
            }
            Expr::Call {
                callee, args, span, ..
            } => {
                let callee_ty = self.check_expr(callee)?;
                let mut arg_tys = Vec::with_capacity(args.len());
                for arg in args {
                    match arg {
                        Arg::Expr(expr) | Arg::Spread(expr) => {
                            arg_tys.push(self.check_expr(expr)?);
                        }
                    }
                }
                // T07.01: annotated non-generic functions get call-site argument checking.
                if let Expr::Ident(id) = peel_parens(callee) {
                    if let Some(sym_id) = self.resolve_span(id.span) {
                        if let Some(sig) = self.fn_sigs[sym_id.0 as usize].as_ref() {
                            self.check_call_sig(sig, args, &arg_tys, *span)?;
                        }
                        // T07.04: calling an annotated non-callable value (e.g.
                        // `let x: number = 1; x()`) is a compile diagnostic. Untyped
                        // /inferred JS values (E19.13/E19.59) stay permissive.
                        if self.symbol_annotated[sym_id.0 as usize]
                            && !self.type_is_callable(self.symbol_types[sym_id.0 as usize])
                        {
                            let callee_s = format_type_full(
                                self.symbol_types[sym_id.0 as usize],
                                &self.shapes,
                                &self.unions,
                                &self.intersections,
                            );
                            return Err(Diagnostic::new(
                                format!("type `{callee_s}` is not callable"),
                                *span,
                            )
                            .with_code(codes::NOT_CALLABLE)
                            .with_help(
                                "only functions (and values with a call signature) can be called",
                            ));
                        }
                    }
                }
                // Native/ptr have no JS [[Call]].
                if callee_ty.is_native_world() {
                    return Err(Diagnostic::new(
                        format!("type `{callee_ty}` is not callable"),
                        *span,
                    )
                    .with_code(codes::NOT_CALLABLE)
                    .with_help("only functions (and values with a call signature) can be called"));
                }
                let result_ty = match callee_ty {
                    Type::GenericFn(gid) => self.instantiate_generic_call(gid, &arg_tys, *span)?,
                    // E19.13 / E19.59: JS values may lack [[Call]]; TypeError is runtime.
                    _ => Type::Any,
                };
                self.record(*span, result_ty);
                result_ty
            }
            Expr::New { callee, args, span } => {
                let callee_ty = self.check_expr(callee)?;
                let mut arg_tys = Vec::with_capacity(args.len());
                for arg in args {
                    match arg {
                        Arg::Expr(expr) | Arg::Spread(expr) => {
                            arg_tys.push(self.check_expr(expr)?);
                        }
                    }
                }
                // Native/ptr have no JS [[Construct]]. E19.59: boolean/number/string/null
                // (and other JS values) — TypeError is runtime, not compile reject.
                if callee_ty.is_native_world() {
                    return Err(Diagnostic::new(
                        format!("type `{callee_ty}` is not constructable"),
                        *span,
                    )
                    .with_code(codes::NOT_CONSTRUCTABLE)
                    .with_help("only constructors and classes can be used with `new`"));
                }
                // T07.04: `new` of an annotated non-constructable value (e.g.
                // `let x: number = 1; new x()`) is a compile diagnostic.
                if let Expr::Ident(id) = peel_parens(callee) {
                    if let Some(sym_id) = self.resolve_span(id.span) {
                        if self.symbol_annotated[sym_id.0 as usize]
                            && !self.type_is_callable(self.symbol_types[sym_id.0 as usize])
                        {
                            let callee_s = format_type_full(
                                self.symbol_types[sym_id.0 as usize],
                                &self.shapes,
                                &self.unions,
                                &self.intersections,
                            );
                            return Err(Diagnostic::new(
                                format!("type `{callee_s}` is not constructable"),
                                *span,
                            )
                            .with_code(codes::NOT_CONSTRUCTABLE)
                            .with_help("only constructors and classes can be used with `new`"));
                        }
                    }
                }
                // Proxy(target, handler): result is callable when target is.
                // Function(...): constructs a function from source strings.
                let result_ty = if self.is_global_ident(callee, "Proxy") {
                    match arg_tys.first().copied() {
                        Some(Type::Function) => Type::Function,
                        Some(Type::Any) => Type::Any,
                        _ => Type::Object,
                    }
                } else if self.is_global_ident(callee, "Function") {
                    Type::Function
                } else {
                    Type::Object
                };
                self.record(*span, result_ty);
                result_ty
            }
            Expr::FunctionExpression {
                name,
                params,
                return_type,
                body,
                is_async,
                is_generator,
                is_method,
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
                if *is_method
                    && (params_contain_super_call(params) || stmt_contains_super_call(body))
                {
                    return Err(Diagnostic::new(
                        "method cannot contain super call".to_string(),
                        *span,
                    ));
                }
                if !*is_method && (params_contain_super(params) || stmt_contains_super(body)) {
                    return Err(Diagnostic::new(
                        "function cannot contain super".to_string(),
                        *span,
                    ));
                }
                let prev_super = self.binder.super_allowed;
                self.binder.super_allowed = *is_method;
                let named = name.is_some();
                if named {
                    self.binder.push_scope_kind(true);
                    if let Some(name) = name {
                        self.binder
                            .declare(name.name.clone(), name.span, BindingKind::Function)?;
                    }
                }
                self.binder.push_scope_kind(true);
                let allow_sloppy_dups = !*is_async && !*is_generator && !*is_method;
                let result = (|| {
                    self.binder.bind_params(params, allow_sloppy_dups)?;
                    self.binder.install_arguments_object()?;
                    self.sync_symbols();
                    self.binder
                        .check_params_body_lexical_conflict(params, body)?;
                    if let Some(name) = name {
                        if let Some(id) = self
                            .symbols()
                            .iter()
                            .find(|s| s.span == name.span)
                            .map(|s| s.id)
                        {
                            self.symbol_types[id.0 as usize] = Type::Function;
                        }
                    }
                    self.check_params_await_yield(
                        params,
                        *is_async && *is_generator,
                        *is_generator,
                    )?;
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
                    let body_result = self.walk_function_body(body, 1, &mut inner_labels);
                    if body_result.is_ok() {
                        if let Some(ty) = ret_ty {
                            self.check_missing_return(body, ty)?;
                        }
                    }
                    self.in_async = prev_async;
                    self.in_generator = prev_generator;
                    self.expected_return = prev_ret;
                    body_result
                })();
                self.binder.pop_scope();
                if named {
                    self.binder.pop_scope();
                }
                self.binder.super_allowed = prev_super;
                self.binder.strict = prev_strict;
                result?;
                self.record(*span, Type::Function);
                Type::Function
            }
            Expr::ClassExpression {
                name,
                super_class,
                body,
                span,
            } => {
                self.binder.push_scope_kind(true);
                let result = (|| {
                    if let Some(name) = name {
                        self.binder
                            .declare(name.name.clone(), name.span, BindingKind::Function)?;
                    } else {
                        self.binder
                            .declare("__class".into(), *span, BindingKind::Function)?;
                    }
                    self.sync_symbols();
                    let class_span = name.as_ref().map(|n| n.span).unwrap_or(*span);
                    if let Some(id) = self
                        .symbols()
                        .iter()
                        .find(|s| s.span == class_span)
                        .map(|s| s.id)
                    {
                        self.symbol_types[id.0 as usize] = Type::Function;
                    }
                    if let Some(sc) = super_class {
                        self.check_expr(sc)?;
                    }
                    self.walk_class_elements(body, 0)
                })();
                self.binder.pop_scope();
                result?;
                self.record(*span, Type::Function);
                Type::Function
            }
            Expr::ArrowFunction {
                params,
                return_type,
                body,
                is_async,
                span,
            } => {
                let prev_strict = self.binder.strict;
                let body_strict = match body {
                    ArrowBody::Block(stmt) => body_has_use_strict(stmt),
                    ArrowBody::Expr(_) => false,
                };
                if body_strict {
                    if !is_simple_parameter_list(params) {
                        return Err(Diagnostic::new(
                            "\"use strict\" not allowed in function with non-simple parameter list"
                                .to_string(),
                            *span,
                        ));
                    }
                    self.binder.strict = true;
                }
                let body_super = match body {
                    ArrowBody::Expr(e) => expr_contains_super(e),
                    ArrowBody::Block(s) => stmt_contains_super(s),
                };
                if !self.binder.super_allowed && (params_contain_super(params) || body_super) {
                    return Err(Diagnostic::new(
                        "arrow function cannot contain super".to_string(),
                        *span,
                    ));
                }
                self.binder.push_scope_kind(true);
                let result = (|| {
                    self.binder.bind_params(params, false)?;
                    self.sync_symbols();
                    if let ArrowBody::Block(stmt) = body {
                        self.binder
                            .check_params_body_lexical_conflict(params, stmt)?;
                    }
                    self.check_params_await_yield(params, *is_async, false)?;
                    let mut inner_labels = Vec::new();
                    let prev_async = self.in_async;
                    let prev_generator = self.in_generator;
                    let prev_ret = self.expected_return;
                    self.in_async = *is_async;
                    self.in_generator = false;
                    let ret_ty = if self.typecheck {
                        match return_type {
                            Some(ann) => Some(self.resolve_type_ann(ann)?),
                            None => None,
                        }
                    } else {
                        None
                    };
                    self.expected_return = ret_ty;
                    let body_result = match body {
                        ArrowBody::Expr(expr) => {
                            let body_ty = self.check_expr(expr)?;
                            if let Some(expected) = ret_ty {
                                self.require_assignable_expr(body_ty, expected, expr)?;
                            }
                            Ok(())
                        }
                        ArrowBody::Block(stmt) => {
                            let r = self.walk_function_body(stmt, 1, &mut inner_labels);
                            if r.is_ok() {
                                if let Some(ty) = ret_ty {
                                    self.check_missing_return(stmt, ty)?;
                                }
                            }
                            r
                        }
                    };
                    self.in_async = prev_async;
                    self.in_generator = prev_generator;
                    self.expected_return = prev_ret;
                    body_result
                })();
                self.binder.pop_scope();
                self.binder.strict = prev_strict;
                result?;
                self.record(*span, Type::Function);
                Type::Function
            }
            Expr::ObjectExpression { properties, span } => {
                let mut shape_props: Vec<(String, Type)> = Vec::new();
                let mut structural = true;
                for prop in properties {
                    match prop {
                        ObjectProp::Property { key, value, .. } => {
                            if let ObjectKey::Computed(expr) = key {
                                self.check_expr(expr)?;
                                structural = false;
                            }
                            let val_ty = self.check_expr(value)?;
                            if structural {
                                match key {
                                    ObjectKey::Ident(id) => {
                                        shape_props.push((id.name.clone(), val_ty));
                                    }
                                    ObjectKey::String(s) => {
                                        shape_props.push((s.value.to_string_lossy(), val_ty));
                                    }
                                    ObjectKey::Computed(_) => unreachable!(),
                                }
                            }
                        }
                        ObjectProp::Accessor {
                            key,
                            params,
                            body,
                            span,
                            ..
                        } => {
                            if let ObjectKey::Computed(expr) = key {
                                self.check_expr(expr)?;
                            }
                            if body_has_use_strict(body) && !is_simple_parameter_list(params) {
                                return Err(Diagnostic::new(
                                    "\"use strict\" not allowed in function with non-simple parameter list".to_string(),
                                    *span,
                                ));
                            }
                            if params_contain_super_call(params) || stmt_contains_super_call(body) {
                                return Err(Diagnostic::new(
                                    "method cannot contain super call".to_string(),
                                    *span,
                                ));
                            }
                            let prev_strict = self.binder.strict;
                            if body_has_use_strict(body) {
                                self.binder.strict = true;
                            }
                            self.binder.push_scope_kind(true);
                            let acc_result = (|| {
                                self.binder.bind_params(params, false)?;
                                self.binder.install_arguments_object()?;
                                self.sync_symbols();
                                self.binder
                                    .check_params_body_lexical_conflict(params, body)?;
                                self.check_params_await_yield(params, false, false)?;
                                let mut inner_labels = Vec::new();
                                let prev_async = self.in_async;
                                let prev_generator = self.in_generator;
                                self.in_async = false;
                                self.in_generator = false;
                                let r = self.walk_function_body(body, 1, &mut inner_labels);
                                self.in_async = prev_async;
                                self.in_generator = prev_generator;
                                r
                            })();
                            self.binder.pop_scope();
                            self.binder.strict = prev_strict;
                            acc_result?;
                            // Accessors make the shape dynamic for structural typing.
                            structural = false;
                        }
                        ObjectProp::Spread { expr, .. } => {
                            self.check_expr(expr)?;
                            structural = false;
                        }
                    }
                }
                let ty = if structural {
                    self.intern_shape(shape_props, false)
                } else {
                    Type::Object
                };
                self.record(*span, ty);
                ty
            }
            Expr::ArrayExpression { elements, span, .. } => {
                for el in elements {
                    match el {
                        ArrayElement::Expr(expr) | ArrayElement::Spread(expr) => {
                            self.check_expr(expr)?;
                        }
                        ArrayElement::Elision => {}
                    }
                }
                self.record(*span, Type::Object);
                Type::Object
            }
            Expr::MemberExpression {
                object,
                property,
                computed,
                span,
                ..
            } => {
                let obj_ty = self.check_expr(object)?;
                let ty = if *computed {
                    self.check_expr(property)?;
                    // Tuple / fixed-array index: `a[0]` → shape prop `"0"` (N03.02).
                    if let Some(idx) = Self::const_index_key(property) {
                        self.prop_type(obj_ty, &idx).unwrap_or(Type::Any)
                    } else {
                        Type::Any
                    }
                } else if let Expr::Ident(id) = property.as_ref() {
                    self.member_prop_type(obj_ty, &id.name, id.span)?
                } else {
                    Type::Any
                };
                self.record(*span, ty);
                ty
            }
            Expr::PrivateIn { object, span, .. } => {
                self.check_expr(object)?;
                self.record(*span, Type::Boolean);
                Type::Boolean
            }
        };
        Ok(ty)
    }
}
