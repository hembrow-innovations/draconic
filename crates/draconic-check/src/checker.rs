use std::collections::HashMap;

use draconic_ast::{
    ArrayPatternElement, BindingKind, BindingPattern, ClassElement, Expr, ObjectPatternProp, Param,
    Program, Stmt, TypeAnn,
};
use draconic_diagnostics::{codes, Diagnostic, Span};

use super::binder::Binder;
use super::host_api;
use super::{
    body_has_use_strict, check_statement_list_early_errors, collect_var_declared_names_stmt,
    expr_has_optional_chain, is_simple_parameter_list, stmt_cannot_fall_through,
    stmt_list_has_use_strict, stmt_span, BoundProgram, CheckedProgram, CompileTarget, GenericFnSig,
    IntersectionType, ObjectShape, Symbol, SymbolId, Type, UnionType,
};

/// Resolved signature for a non-generic annotated function (T07.01).
/// Recorded for call-site argument checking: arity and per-param assignability.
/// Only built when the function has at least one annotated parameter, so
/// untyped (E19-era) JS stays fully permissive.
#[derive(Debug, Clone)]
pub(crate) struct FnSig {
    /// Resolved type per parameter; `None` = unannotated (permissive).
    pub(crate) param_types: Vec<Option<Type>>,
    /// Number of annotated parameters without a default or rest that must be supplied.
    pub(crate) required: usize,
    /// Whether the final parameter is a rest param (extra args allowed).
    pub(crate) has_rest: bool,
}

/// Generic type alias body (T04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct GenericAlias {
    pub(crate) params: Vec<String>,
    pub(crate) body: TypeAnn,
}

pub(crate) struct Checker {
    pub(crate) binder: Binder,
    /// When false, the walk only resolves symbols (`bind`). When true, it also checks types.
    pub(crate) typecheck: bool,
    pub(crate) symbol_types: Vec<Type>,
    /// True when the binding's type came from a type annotation (not inference).
    /// Untyped JS assignment may widen inferred bindings (E19.12 / E19.48).
    pub(crate) symbol_annotated: Vec<bool>,
    /// Resolved call signatures for annotated functions (T07.01), parallel to `symbol_types`.
    pub(crate) fn_sigs: Vec<Option<FnSig>>,
    pub(crate) expr_types: HashMap<Span, Type>,
    /// Structural object shapes (`Type::Shape` indices).
    pub(crate) shapes: Vec<ObjectShape>,
    /// Union members (`Type::Union` indices).
    pub(crate) unions: Vec<UnionType>,
    /// Intersection members (`Type::Intersection` indices).
    pub(crate) intersections: Vec<IntersectionType>,
    /// Generic function signatures (`Type::GenericFn` indices).
    pub(crate) generic_fns: Vec<GenericFnSig>,
    /// Concrete type aliases in scope (name → resolved type). Program-level for T02.
    pub(crate) type_aliases: HashMap<String, Type>,
    /// Generic type aliases (`type Box<T> = …`).
    pub(crate) generic_aliases: HashMap<String, GenericAlias>,
    /// Active type parameter bindings while resolving/checking (name → Type::TypeParam).
    pub(crate) type_param_env: HashMap<String, Type>,
    /// Monotonic id source for open `Type::TypeParam` values.
    pub(crate) next_type_param_id: u32,
    /// True while typechecking an `async` function body.
    pub(crate) in_async: bool,
    /// True while typechecking a generator function body.
    pub(crate) in_generator: bool,
    /// Expected return type from an enclosing annotated function (T01).
    pub(crate) expected_return: Option<Type>,
    /// When set, free host API references are checked against this target (H00.01).
    pub(crate) host_target: Option<CompileTarget>,
}

impl Checker {
    pub(crate) fn new() -> Self {
        let binder = Binder::new();
        let n = binder.symbols.len();
        let mut symbol_types = vec![Type::Any; n];
        for s in &binder.symbols {
            // Host globals installed with Span::dummy() (E08.05+).
            if s.span == Span::dummy() {
                symbol_types[s.id.0 as usize] = match s.name.as_str() {
                    "Math" | "Reflect" | "globalThis" | "JSON" => Type::Object,
                    "Number" | "Symbol" | "Promise" | "Proxy" | "Object" | "Function" | "Array"
                    | "String" | "Boolean" | "Error" | "TypeError" | "RangeError"
                    | "ReferenceError" | "SyntaxError" | "URIError" | "EvalError"
                    | "AggregateError" | "parseInt" | "parseFloat" | "isNaN" | "isFinite"
                    | "encodeURI" | "decodeURI" | "encodeURIComponent" | "decodeURIComponent"
                    | "Date" | "RegExp" | "Map" | "Set" | "WeakMap" | "WeakSet" | "ArrayBuffer"
                    | "DataView" | "Int8Array" | "Uint8Array" | "Uint8ClampedArray"
                    | "Int16Array" | "Uint16Array" | "Int32Array" | "Uint32Array"
                    | "Float32Array" | "Float64Array" | "BigInt64Array" | "BigUint64Array"
                    | "TextEncoder" | "TextDecoder" | "eval" | "escape" | "unescape"
                    | "ShadowRealm" => Type::Function,
                    "NaN" | "Infinity" => Type::Number,
                    // `undefined` is its own ES language type; coarse `any` until refined.
                    "undefined" => Type::Any,
                    _ => Type::Any,
                };
            }
        }
        Self {
            binder,
            typecheck: true,
            symbol_types,
            symbol_annotated: vec![false; n],
            fn_sigs: vec![None; n],
            expr_types: HashMap::new(),
            shapes: Vec::new(),
            unions: Vec::new(),
            intersections: Vec::new(),
            generic_fns: Vec::new(),
            type_aliases: HashMap::new(),
            generic_aliases: HashMap::new(),
            type_param_env: HashMap::new(),
            next_type_param_id: 0,
            in_async: false,
            in_generator: false,
            expected_return: None,
            host_target: None,
        }
    }

    pub(crate) fn sync_symbols(&mut self) {
        while self.symbol_types.len() < self.binder.symbols.len() {
            self.symbol_types.push(Type::Any);
            self.symbol_annotated.push(false);
            self.fn_sigs.push(None);
        }
    }

    pub(crate) fn resolve_span(&self, span: Span) -> Option<SymbolId> {
        self.binder.resolutions.get(&span).copied()
    }

    pub(crate) fn symbols(&self) -> &[Symbol] {
        &self.binder.symbols
    }

    pub(crate) fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.binder.symbols[id.0 as usize]
    }

    pub(crate) fn declare_type_aliases(&mut self, body: &[Stmt]) -> Result<(), Diagnostic> {
        // Program-level type aliases (T02/T04): declare names, then resolve non-generic bodies.
        for stmt in body {
            if let Stmt::TypeAlias {
                name, type_params, ..
            } = stmt
            {
                if self.type_aliases.contains_key(&name.name)
                    || self.generic_aliases.contains_key(&name.name)
                {
                    return Err(Diagnostic::new(
                        format!("duplicate type alias `{}`", name.name),
                        name.span,
                    ));
                }
                if type_params.is_empty() {
                    self.type_aliases.insert(name.name.clone(), Type::Any);
                } else {
                    // Placeholder so mutual refs among generics are not "unknown".
                    self.generic_aliases.insert(
                        name.name.clone(),
                        GenericAlias {
                            params: type_params.iter().map(|p| p.name.name.clone()).collect(),
                            body: TypeAnn::Named {
                                name: "any".into(),
                                span: name.span,
                            },
                        },
                    );
                }
            }
        }
        let alias_bodies: Vec<(String, Vec<String>, TypeAnn)> = body
            .iter()
            .filter_map(|s| match s {
                Stmt::TypeAlias {
                    name,
                    type_params,
                    ty,
                    ..
                } => Some((
                    name.name.clone(),
                    type_params.iter().map(|p| p.name.name.clone()).collect(),
                    ty.clone(),
                )),
                _ => None,
            })
            .collect();
        for (name, params, ty) in alias_bodies {
            if params.is_empty() {
                let resolved = self.resolve_type_ann(&ty)?;
                self.type_aliases.insert(name, resolved);
            } else {
                // Validate body resolves under open type params.
                let saved = self.type_param_env.clone();
                for p in &params {
                    let id = self.next_type_param_id;
                    self.next_type_param_id += 1;
                    self.type_param_env.insert(p.clone(), Type::TypeParam(id));
                }
                let _ = self.resolve_type_ann(&ty)?;
                self.type_param_env = saved;
                self.generic_aliases
                    .insert(name, GenericAlias { params, body: ty });
            }
        }
        Ok(())
    }

    pub(crate) fn into_bound(self, program: Program) -> BoundProgram {
        BoundProgram {
            program,
            symbols: self.binder.symbols,
            resolutions: self.binder.resolutions,
        }
    }

    pub(crate) fn into_checked(self, program: Program) -> CheckedProgram {
        CheckedProgram {
            bound: BoundProgram {
                program,
                symbols: self.binder.symbols,
                resolutions: self.binder.resolutions,
            },
            symbol_types: self.symbol_types,
            expr_types: self.expr_types,
            shapes: self.shapes,
            unions: self.unions,
            intersections: self.intersections,
            generic_fns: self.generic_fns,
            type_aliases: self.type_aliases,
        }
    }

    pub(crate) fn analyze(
        &mut self,
        program: &Program,
        module_goal: bool,
    ) -> Result<(), Diagnostic> {
        if stmt_list_has_use_strict(&program.body) {
            self.binder.strict = true;
        }
        if self.typecheck {
            self.declare_type_aliases(&program.body)?;
        }
        let mut labels = Vec::new();
        self.walk_stmt_list(&program.body, !module_goal, 0, 0, 0, &mut labels)
    }

    /// Declare list bindings, then walk each statement (bind + optional typecheck).
    pub(crate) fn walk_stmt_list(
        &mut self,
        stmts: &[Stmt],
        top_level: bool,
        loop_depth: u32,
        switch_depth: u32,
        fn_depth: u32,
        labels: &mut Vec<(String, bool)>,
    ) -> Result<(), Diagnostic> {
        check_statement_list_early_errors(stmts, self.binder.strict, top_level)?;
        for stmt in stmts {
            self.binder.declare_list_item(stmt)?;
        }
        self.sync_symbols();
        for stmt in stmts {
            self.check_stmt(stmt, loop_depth, switch_depth, fn_depth, labels)?;
        }
        Ok(())
    }

    pub(crate) fn walk_function_body(
        &mut self,
        body: &Stmt,
        fn_depth: u32,
        labels: &mut Vec<(String, bool)>,
    ) -> Result<(), Diagnostic> {
        match body {
            Stmt::Block { body, .. } => {
                self.binder.push_scope();
                self.walk_stmt_list(body, true, 0, 0, fn_depth, labels)?;
                self.binder.pop_scope();
                Ok(())
            }
            other => self.check_stmt(other, 0, 0, fn_depth, labels),
        }
    }

    pub(crate) fn walk_method_like(
        &mut self,
        params: &[Param],
        body: &Stmt,
        is_async: bool,
        is_generator: bool,
        fn_depth: u32,
        super_allowed: bool,
        class_strict: bool,
    ) -> Result<(), Diagnostic> {
        let prev_strict = self.binder.strict;
        let prev_super = self.binder.super_allowed;
        if class_strict {
            self.binder.strict = true;
        }
        self.binder.super_allowed = super_allowed;
        self.binder.push_scope_kind(true);
        let result = (|| {
            self.binder.bind_params(params, false)?;
            self.binder.install_arguments_object()?;
            self.sync_symbols();
            self.binder
                .check_params_body_lexical_conflict(params, body)?;
            self.check_params_await_yield(params, is_async && is_generator, is_generator)?;
            let mut inner_labels = Vec::new();
            let prev_async = self.in_async;
            let prev_generator = self.in_generator;
            self.in_async = is_async;
            self.in_generator = is_generator;
            let r = self.walk_function_body(body, fn_depth, &mut inner_labels);
            self.in_async = prev_async;
            self.in_generator = prev_generator;
            r
        })();
        self.binder.pop_scope();
        self.binder.super_allowed = prev_super;
        self.binder.strict = prev_strict;
        result
    }

    pub(crate) fn walk_class_elements(
        &mut self,
        body: &[ClassElement],
        fn_depth: u32,
    ) -> Result<(), Diagnostic> {
        for el in body {
            match el {
                ClassElement::Constructor { params, body, .. } => {
                    self.walk_method_like(params, body, false, false, fn_depth + 1, true, true)?;
                }
                ClassElement::Method {
                    key,
                    params,
                    body,
                    is_async,
                    is_generator,
                    span,
                    ..
                } => {
                    self.check_object_key(key)?;
                    if body_has_use_strict(body) && !is_simple_parameter_list(params) {
                        return Err(Diagnostic::new(
                            "\"use strict\" not allowed in function with non-simple parameter list"
                                .to_string(),
                            *span,
                        ));
                    }
                    self.walk_method_like(
                        params,
                        body,
                        *is_async,
                        *is_generator,
                        fn_depth + 1,
                        true,
                        true,
                    )?;
                }
                ClassElement::Accessor {
                    key,
                    params,
                    body,
                    span,
                    ..
                } => {
                    self.check_object_key(key)?;
                    if body_has_use_strict(body) && !is_simple_parameter_list(params) {
                        return Err(Diagnostic::new(
                            "\"use strict\" not allowed in function with non-simple parameter list"
                                .to_string(),
                            *span,
                        ));
                    }
                    self.walk_method_like(params, body, false, false, fn_depth + 1, true, true)?;
                }
                ClassElement::Field { key, value, .. } => {
                    self.check_object_key(key)?;
                    if let Some(v) = value {
                        let prev_super = self.binder.super_allowed;
                        self.binder.super_allowed = true;
                        let r = self.check_expr(v);
                        self.binder.super_allowed = prev_super;
                        r?;
                    }
                }
                ClassElement::StaticBlock { body, .. } => {
                    let prev_strict = self.binder.strict;
                    let mut inner_labels = Vec::new();
                    let prev_async = self.in_async;
                    let prev_generator = self.in_generator;
                    self.binder.strict = true;
                    self.in_async = false;
                    self.in_generator = false;
                    let r = self.check_stmt(body, 0, 0, fn_depth + 1, &mut inner_labels);
                    self.in_async = prev_async;
                    self.in_generator = prev_generator;
                    self.binder.strict = prev_strict;
                    r?;
                }
            }
        }
        Ok(())
    }

    pub(crate) fn check_for_in_of(
        &mut self,
        left: &Stmt,
        right: &Expr,
        body: &Stmt,
        is_for_in: bool,
        _is_await: bool,
        _span: Option<Span>,
        loop_depth: u32,
        switch_depth: u32,
        fn_depth: u32,
        labels: &mut Vec<(String, bool)>,
    ) -> Result<(), Diagnostic> {
        if let Stmt::Let {
            kind,
            binding,
            init,
            ..
        } = left
        {
            if init.is_some() && !(is_for_in && *kind == BindingKind::Var) {
                return Err(Diagnostic::new(
                    "for-in/of binding cannot have an initializer".to_string(),
                    binding.span(),
                ));
            }
            if kind.is_lexical() {
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
            }
            if kind.is_lexical() {
                self.binder.push_scope();
                let result = (|| {
                    self.binder.declare_binding(binding, *kind)?;
                    self.sync_symbols();
                    self.check_for_in_of_left(left)?;
                    self.check_expr(right)?;
                    self.check_stmt(body, loop_depth + 1, switch_depth, fn_depth, labels)
                })();
                self.binder.pop_scope();
                result
            } else {
                self.check_for_in_of_left(left)?;
                self.check_expr(right)?;
                self.check_stmt(body, loop_depth + 1, switch_depth, fn_depth, labels)
            }
        } else {
            self.check_stmt(left, loop_depth, switch_depth, fn_depth, labels)?;
            self.check_expr(right)?;
            self.check_stmt(body, loop_depth + 1, switch_depth, fn_depth, labels)
        }
    }

    /// Left side of `for-in` / `for-of`: `let`/`const`/`var` binding or assignable LHS.
    /// Annex B.3.5 allows `var name = init` only on for-in (checked by the parser).
    pub(crate) fn check_for_in_of_left(&mut self, left: &Stmt) -> Result<(), Diagnostic> {
        match left {
            Stmt::Let {
                kind,
                binding,
                init,
                span,
                ..
            } => {
                if init.is_some() && *kind != BindingKind::Var {
                    return Err(Diagnostic::new(
                        "for-in/of binding cannot have an initializer".to_string(),
                        *span,
                    ));
                }
                if let Some(init) = init {
                    self.check_expr(init)?;
                }
                // Iteration values are JS values; leave bindings as Any until finer types.
                self.check_binding_pattern(binding, Type::Any)
            }
            Stmt::Expression {
                expr: Expr::Ident(id),
                ..
            } => {
                self.binder.bind_ident_use(id)?;
                // E17.02.09 / E19.05: free IdentifierReference is runtime PutValue
                // (non-strict creates a global; strict → ReferenceError), not a check error.
                if let Some(sym) = self.resolve_span(id.span) {
                    let ty = self.symbol_types[sym.0 as usize];
                    self.record(id.span, ty);
                } else {
                    if let Some(target) = self.host_target {
                        if let Some(d) = host_api::unsupported_diagnostic(&id.name, target, id.span)
                        {
                            return Err(d);
                        }
                    }
                    self.record(id.span, Type::Any);
                }
                Ok(())
            }
            Stmt::Expression {
                expr: Expr::ArrayPattern { elements, span },
                ..
            } => {
                let binding = BindingPattern::Array {
                    elements: elements.clone(),
                    span: *span,
                };
                self.check_assign_pattern(&binding, *span)
            }
            Stmt::Expression {
                expr: Expr::ObjectPattern { properties, span },
                ..
            } => {
                let binding = BindingPattern::Object {
                    properties: properties.clone(),
                    span: *span,
                };
                self.check_assign_pattern(&binding, *span)
            }
            Stmt::Expression {
                expr:
                    Expr::MemberExpression {
                        optional: false, ..
                    },
                span,
            } => {
                // `for (obj.p of …)` / `for (obj[k] in …)` — validate member LHS.
                if let Stmt::Expression { expr, .. } = left {
                    self.check_expr(expr)?;
                }
                let _ = span;
                Ok(())
            }
            Stmt::Expression { span, .. } => Err(Diagnostic::new(
                "for-in/of left-hand side must be a binding or assignment target".to_string(),
                *span,
            )),
            other => Err(Diagnostic::new(
                "for-in/of left-hand side must be a binding or assignment target".to_string(),
                match other {
                    Stmt::Empty { span }
                    | Stmt::Block { span, .. }
                    | Stmt::If { span, .. }
                    | Stmt::While { span, .. }
                    | Stmt::DoWhile { span, .. }
                    | Stmt::For { span, .. }
                    | Stmt::ForIn { span, .. }
                    | Stmt::ForOf { span, .. }
                    | Stmt::Break { span, .. }
                    | Stmt::Continue { span, .. }
                    | Stmt::Labeled { span, .. }
                    | Stmt::Switch { span, .. }
                    | Stmt::FunctionDeclaration { span, .. }
                    | Stmt::ClassDeclaration { span, .. }
                    | Stmt::Return { span, .. }
                    | Stmt::Throw { span, .. }
                    | Stmt::Try { span, .. }
                    | Stmt::With { span, .. }
                    | Stmt::Let { span, .. }
                    | Stmt::Expression { span, .. }
                    | Stmt::ImportDeclaration { span, .. }
                    | Stmt::ExportNamedDeclaration { span, .. }
                    | Stmt::ExportDefaultDeclaration { span, .. }
                    | Stmt::ExportAllDeclaration { span, .. }
                    | Stmt::TypeAlias { span, .. }
                    | Stmt::ExternFunctionDeclaration { span, .. } => *span,
                },
            )),
        }
    }

    pub(crate) fn check_binding_pattern(
        &mut self,
        binding: &BindingPattern,
        ty: Type,
    ) -> Result<(), Diagnostic> {
        self.check_binding_pattern_annotated(binding, ty, false)
    }

    pub(crate) fn check_binding_pattern_annotated(
        &mut self,
        binding: &BindingPattern,
        ty: Type,
        annotated: bool,
    ) -> Result<(), Diagnostic> {
        match binding {
            BindingPattern::Ident(name) => {
                let id = self
                    .symbols()
                    .iter()
                    .find(|s| s.span == name.span)
                    .map(|s| s.id)
                    .ok_or_else(|| {
                        Diagnostic::new(format!("undeclared binding `{}`", name.name), name.span)
                    })?;
                self.symbol_types[id.0 as usize] = ty;
                if annotated {
                    self.symbol_annotated[id.0 as usize] = true;
                }
                Ok(())
            }
            BindingPattern::Member(expr) => {
                // Declaration patterns must not use member targets.
                Err(Diagnostic::new(
                    "member expression is not a valid declaration binding".to_string(),
                    expr_span_of(expr),
                ))
            }
            BindingPattern::Array { elements, .. } => {
                // Element types are not refined yet; bind as Any.
                for el in elements {
                    match el {
                        ArrayPatternElement::Elision => {}
                        ArrayPatternElement::Pattern { binding, default } => {
                            self.check_binding_pattern(binding, Type::Any)?;
                            if let Some(def) = default {
                                self.check_expr(def)?;
                            }
                        }
                        ArrayPatternElement::Rest(binding) => {
                            self.check_binding_pattern(binding, Type::Any)?;
                        }
                    }
                }
                Ok(())
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
                            self.check_object_key(key)?;
                            self.check_binding_pattern(binding, Type::Any)?;
                            if let Some(def) = default {
                                self.check_expr(def)?;
                            }
                        }
                        ObjectPatternProp::Rest(binding) => {
                            self.check_binding_pattern(binding, Type::Any)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    pub(crate) fn check_assign_pattern(
        &mut self,
        binding: &BindingPattern,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match binding {
            BindingPattern::Ident(id) => {
                if self.binder.strict && (id.name == "eval" || id.name == "arguments") {
                    return Err(Diagnostic::new(
                        format!("cannot assign to `{}` in strict mode", id.name),
                        id.span,
                    ));
                }
                self.binder.bind_ident_use(id)?;
                let Some(sym) = self.resolve_span(id.span) else {
                    // Free / with-chain assign target (global object property).
                    self.record(id.span, Type::Any);
                    return Ok(());
                };
                // E19.57 / E19.60: const/using/function-name PutValue is runtime TypeError
                // (or silent); do not compile-reject.
                self.record(id.span, self.symbol_types[sym.0 as usize]);
                Ok(())
            }
            BindingPattern::Member(expr) => {
                // Validate member LHS the same way as a simple property assign.
                match expr.as_ref() {
                    Expr::MemberExpression {
                        object,
                        property,
                        computed,
                        private: _,
                        span: mspan,
                        ..
                    } => {
                        // E19.58: OptionalExpression is not a valid AssignmentTarget.
                        if expr_has_optional_chain(expr) {
                            return Err(Diagnostic::new(
                                "invalid assignment target".to_string(),
                                *mspan,
                            ));
                        }
                        // E19.82.10: private members are valid destructuring assign targets.
                        self.check_expr(object)?;
                        if *computed {
                            self.check_expr(property)?;
                        }
                        Ok(())
                    }
                    _ => Err(Diagnostic::new(
                        "invalid assignment target".to_string(),
                        span,
                    )),
                }
            }
            BindingPattern::Array { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Elision => {}
                        ArrayPatternElement::Pattern { binding, default } => {
                            self.check_assign_pattern(binding, span)?;
                            if let Some(def) = default {
                                self.check_expr(def)?;
                            }
                        }
                        ArrayPatternElement::Rest(binding) => {
                            self.check_assign_pattern(binding, span)?;
                        }
                    }
                }
                Ok(())
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
                            self.check_object_key(key)?;
                            self.check_assign_pattern(binding, span)?;
                            if let Some(def) = default {
                                self.check_expr(def)?;
                            }
                        }
                        ObjectPatternProp::Rest(binding) => {
                            self.check_assign_pattern(binding, span)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// T07.02: reject an annotated non-void function whose body can fall off the end
    /// without returning a value (e.g. `function f(): number { let x = 1; }`).
    pub(crate) fn check_missing_return(&self, body: &Stmt, ret_ty: Type) -> Result<(), Diagnostic> {
        if !self.typecheck {
            return Ok(());
        }
        // `any` accepts `undefined` (fall-off-end); `void` is not a Draconic annotation.
        if ret_ty == Type::Any || stmt_cannot_fall_through(body) {
            return Ok(());
        }
        Err(Diagnostic::new(
            format!(
                "missing return: function with return type `{ret_ty}` may fall off the end without returning a value"
            ),
            stmt_span(body),
        )
        .with_code(codes::MISSING_RETURN)
        .with_help(
            "add a return on every path, or change the return type annotation",
        ))
    }
}

pub(crate) fn expr_span_of(expr: &Expr) -> Span {
    match expr {
        Expr::Ident(i) => i.span,
        Expr::Number(n) => n.span,
        Expr::BigInt(n) => n.span,
        Expr::String(s) => s.span,
        Expr::RegExp { span, .. } => *span,
        Expr::Boolean { span, .. }
        | Expr::Null { span }
        | Expr::This { span }
        | Expr::Super { span }
        | Expr::NewTarget { span }
        | Expr::ImportMeta { span }
        | Expr::ImportCall { span, .. }
        | Expr::TemplateLiteral { span, .. }
        | Expr::TaggedTemplate { span, .. }
        | Expr::Unary { span, .. }
        | Expr::Binary { span, .. }
        | Expr::Conditional { span, .. }
        | Expr::Assign { span, .. }
        | Expr::Update { span, .. }
        | Expr::Call { span, .. }
        | Expr::New { span, .. }
        | Expr::FunctionExpression { span, .. }
        | Expr::ClassExpression { span, .. }
        | Expr::ArrowFunction { span, .. }
        | Expr::ObjectExpression { span, .. }
        | Expr::ArrayExpression { span, .. }
        | Expr::ArrayPattern { span, .. }
        | Expr::ObjectPattern { span, .. }
        | Expr::MemberExpression { span, .. }
        | Expr::PrivateIn { span, .. }
        | Expr::Paren { span, .. }
        | Expr::As { span, .. } => *span,
    }
}
