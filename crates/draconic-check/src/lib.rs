//! Binder (scopes + symbol resolution) and Checker (TypeScript-inspired).
//! Binder: ROADMAP B04. Checker: ROADMAP B05.

use draconic_ast::{BinaryOp, BindingKind, Expr, Program, Stmt, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SymbolId(pub u32);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub id: SymbolId,
    pub name: String,
    /// Span of the binding name at the declaration site.
    pub span: Span,
    pub kind: BindingKind,
}

/// TypeScript-inspired types for the minimal Program surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    Number,
    String,
    Boolean,
    Null,
    /// Callable function value (declaration or expression).
    Function,
    /// Flexible / unannotated (e.g. `let x;` with no initializer).
    Any,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Type::Number => "number",
            Type::String => "string",
            Type::Boolean => "boolean",
            Type::Null => "null",
            Type::Function => "function",
            Type::Any => "any",
        };
        write!(f, "{s}")
    }
}

/// Program after scope analysis and identifier resolution.
#[derive(Debug)]
pub struct BoundProgram {
    pub program: Program,
    symbols: Vec<Symbol>,
    /// Use-site identifier span → declared symbol.
    resolutions: HashMap<Span, SymbolId>,
}

impl BoundProgram {
    pub fn symbols(&self) -> &[Symbol] {
        &self.symbols
    }

    pub fn resolve(&self, use_span: Span) -> Option<SymbolId> {
        self.resolutions.get(&use_span).copied()
    }

    pub fn symbol(&self, id: SymbolId) -> &Symbol {
        &self.symbols[id.0 as usize]
    }
}

/// Bound program with inferred / checked types.
#[derive(Debug)]
pub struct CheckedProgram {
    pub bound: BoundProgram,
    /// Declaration symbol → type.
    symbol_types: Vec<Type>,
    /// Expression span → type.
    expr_types: HashMap<Span, Type>,
}

impl CheckedProgram {
    pub fn type_of_symbol(&self, id: SymbolId) -> Type {
        self.symbol_types[id.0 as usize]
    }

    pub fn type_of_expr(&self, span: Span) -> Option<Type> {
        self.expr_types.get(&span).copied()
    }
}

/// Bind scopes and resolve identifiers for a minimal Program.
pub fn bind(program: Program) -> Result<BoundProgram, Diagnostic> {
    let mut binder = Binder::new();
    binder.bind_program(program)
}

pub fn check(program: Program) -> Result<CheckedProgram, Diagnostic> {
    let bound = bind(program)?;
    let mut checker = Checker::new(&bound);
    checker.check_program()?;
    let symbol_types = checker.symbol_types;
    let expr_types = checker.expr_types;
    Ok(CheckedProgram {
        bound,
        symbol_types,
        expr_types,
    })
}

/// Whether a labelled item is (or wraps) an iteration statement — needed for
/// `continue label` validity (ECMA-262 LabelledStatement).
fn is_iteration_labelled_item(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::While { .. }
        | Stmt::DoWhile { .. }
        | Stmt::For { .. }
        | Stmt::ForIn { .. }
        | Stmt::ForOf { .. } => true,
        Stmt::Labeled { body, .. } => is_iteration_labelled_item(body),
        _ => false,
    }
}

struct Binder {
    /// Scope stack (innermost last): name → symbol id.
    scopes: Vec<HashMap<String, SymbolId>>,
    symbols: Vec<Symbol>,
    resolutions: HashMap<Span, SymbolId>,
}

impl Binder {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
            symbols: Vec::new(),
            resolutions: HashMap::new(),
        }
    }

    fn bind_program(&mut self, program: Program) -> Result<BoundProgram, Diagnostic> {
        self.bind_stmt_list(&program.body)?;

        Ok(BoundProgram {
            program,
            symbols: std::mem::take(&mut self.symbols),
            resolutions: std::mem::take(&mut self.resolutions),
        })
    }

    /// Two-pass list bind: declare lexical bindings in this scope, then bind each statement.
    fn bind_stmt_list(&mut self, stmts: &[Stmt]) -> Result<(), Diagnostic> {
        for stmt in stmts {
            match stmt {
                Stmt::Let { kind, name, .. } => {
                    self.declare(name.name.clone(), name.span, *kind)?;
                }
                Stmt::FunctionDeclaration { name, .. } => {
                    self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                }
                _ => {}
            }
        }
        for stmt in stmts {
            self.bind_stmt(stmt)?;
        }
        Ok(())
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(
        &mut self,
        name: String,
        span: Span,
        kind: BindingKind,
    ) -> Result<SymbolId, Diagnostic> {
        let scope = self.scopes.last_mut().expect("scope stack non-empty");
        if scope.contains_key(&name) {
            return Err(Diagnostic::new(
                format!("duplicate declaration of `{name}`"),
                span,
            ));
        }
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.clone(),
            span,
            kind,
        });
        scope.insert(name, id);
        Ok(id)
    }

    fn resolve_name(&self, name: &str) -> Option<SymbolId> {
        for scope in self.scopes.iter().rev() {
            if let Some(id) = scope.get(name) {
                return Some(*id);
            }
        }
        None
    }

    fn bind_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Expression { expr, .. } => self.bind_expr(expr),
            Stmt::Let { init, .. } => {
                if let Some(init) = init {
                    self.bind_expr(init)?;
                }
                Ok(())
            }
            Stmt::Empty { .. } => Ok(()),
            Stmt::Block { body, .. } => {
                self.push_scope();
                self.bind_stmt_list(body)?;
                self.pop_scope();
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.bind_expr(test)?;
                self.bind_stmt(consequent)?;
                if let Some(alt) = alternate {
                    self.bind_stmt(alt)?;
                }
                Ok(())
            }
            Stmt::While { test, body, .. } => {
                self.bind_expr(test)?;
                self.bind_stmt(body)
            }
            Stmt::DoWhile { body, test, .. } => {
                self.bind_stmt(body)?;
                self.bind_expr(test)
            }
            Stmt::For {
                init,
                test,
                update,
                body,
                ..
            } => {
                // `for (let …)` introduces a loop-scoped binding visible in
                // test, update, and body.
                if let Some(Stmt::Let {
                    kind,
                    name,
                    init: let_init,
                    ..
                }) = init.as_deref()
                {
                    self.push_scope();
                    self.declare(name.name.clone(), name.span, *kind)?;
                    if let Some(e) = let_init {
                        self.bind_expr(e)?;
                    }
                    if let Some(t) = test {
                        self.bind_expr(t)?;
                    }
                    if let Some(u) = update {
                        self.bind_expr(u)?;
                    }
                    self.bind_stmt(body)?;
                    self.pop_scope();
                    Ok(())
                } else {
                    if let Some(init) = init {
                        self.bind_stmt(init)?;
                    }
                    if let Some(t) = test {
                        self.bind_expr(t)?;
                    }
                    if let Some(u) = update {
                        self.bind_expr(u)?;
                    }
                    self.bind_stmt(body)
                }
            }
            Stmt::ForIn {
                left, right, body, ..
            }
            | Stmt::ForOf {
                left, right, body, ..
            } => self.bind_for_in_of(left, right, body),
            Stmt::Break { .. } | Stmt::Continue { .. } => Ok(()),
            Stmt::Labeled { body, .. } => self.bind_stmt(body),
            Stmt::Switch {
                discriminant,
                cases,
                ..
            } => {
                self.bind_expr(discriminant)?;
                // Switch body is one block scope for all case clauses (ES lexical).
                self.push_scope();
                let mut all_stmts = Vec::new();
                for case in cases {
                    if let Some(test) = &case.test {
                        self.bind_expr(test)?;
                    }
                    all_stmts.extend(case.body.iter());
                }
                // Two-pass bind over concatenated case bodies.
                for stmt in &all_stmts {
                    match stmt {
                        Stmt::Let { kind, name, .. } => {
                            self.declare(name.name.clone(), name.span, *kind)?;
                        }
                        Stmt::FunctionDeclaration { name, .. } => {
                            self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                        }
                        _ => {}
                    }
                }
                for stmt in all_stmts {
                    self.bind_stmt(stmt)?;
                }
                self.pop_scope();
                Ok(())
            }
            Stmt::FunctionDeclaration { params, body, .. } => {
                // Name already declared in the enclosing list's first pass.
                self.push_scope();
                for p in params {
                    self.declare(p.name.clone(), p.span, BindingKind::Let)?;
                }
                // Body is a Block; bind its statements in the param scope (no extra
                // block scope layer needed beyond the block's own push).
                self.bind_stmt(body)?;
                self.pop_scope();
                Ok(())
            }
            Stmt::Return { argument, .. } => {
                if let Some(arg) = argument {
                    self.bind_expr(arg)?;
                }
                Ok(())
            }
        }
    }

    fn bind_for_in_of(
        &mut self,
        left: &Stmt,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), Diagnostic> {
        // `for (let/const name in/of right)` — loop-scoped binding for name.
        if let Stmt::Let {
            kind, name, init, ..
        } = left
        {
            if init.is_some() {
                return Err(Diagnostic::new(
                    "for-in/of binding cannot have an initializer".to_string(),
                    name.span,
                ));
            }
            self.push_scope();
            self.declare(name.name.clone(), name.span, *kind)?;
            self.bind_expr(right)?;
            self.bind_stmt(body)?;
            self.pop_scope();
            Ok(())
        } else {
            self.bind_stmt(left)?;
            self.bind_expr(right)?;
            self.bind_stmt(body)
        }
    }

    fn bind_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Ident(id) => {
                let Some(sym) = self.resolve_name(&id.name) else {
                    return Err(Diagnostic::new(
                        format!("unresolved identifier `{name}`", name = id.name),
                        id.span,
                    ));
                };
                self.resolutions.insert(id.span, sym);
                Ok(())
            }
            Expr::Number(_) | Expr::String(_) | Expr::Boolean { .. } | Expr::Null { .. } => Ok(()),
            Expr::Unary { arg, .. } => self.bind_expr(arg),
            Expr::Binary { left, right, .. } => {
                self.bind_expr(left)?;
                self.bind_expr(right)
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.bind_expr(test)?;
                self.bind_expr(consequent)?;
                self.bind_expr(alternate)
            }
            Expr::Assign { target, value, .. } => {
                self.bind_expr(target)?;
                self.bind_expr(value)
            }
            Expr::Update { arg, .. } => self.bind_expr(arg),
            Expr::Call { callee, args, .. } => {
                self.bind_expr(callee)?;
                for arg in args {
                    self.bind_expr(arg)?;
                }
                Ok(())
            }
            Expr::Paren { expr, .. } => self.bind_expr(expr),
        }
    }
}

struct Checker<'a> {
    bound: &'a BoundProgram,
    symbol_types: Vec<Type>,
    expr_types: HashMap<Span, Type>,
}

impl<'a> Checker<'a> {
    fn new(bound: &'a BoundProgram) -> Self {
        Self {
            bound,
            symbol_types: vec![Type::Any; bound.symbols().len()],
            expr_types: HashMap::new(),
        }
    }

    fn check_program(&mut self) -> Result<(), Diagnostic> {
        let mut labels = Vec::new();
        for stmt in &self.bound.program.body {
            self.check_stmt(stmt, 0, 0, 0, &mut labels)?;
        }
        Ok(())
    }

    /// Left side of `for-in` / `for-of`: `let name` or assignable identifier.
    fn check_for_in_of_left(&mut self, left: &Stmt) -> Result<(), Diagnostic> {
        match left {
            Stmt::Let { name, init, span, .. } => {
                if init.is_some() {
                    return Err(Diagnostic::new(
                        "for-in/of binding cannot have an initializer".to_string(),
                        *span,
                    ));
                }
                let id = self
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == name.span)
                    .map(|s| s.id)
                    .expect("for-in/of let binding must be declared");
                // Iteration values are JS values; leave as Any until finer types.
                self.symbol_types[id.0 as usize] = Type::Any;
                Ok(())
            }
            Stmt::Expression {
                expr: Expr::Ident(id),
                ..
            } => {
                let sym = self.bound.resolve(id.span).ok_or_else(|| {
                    Diagnostic::new(format!("unresolved identifier `{}`", id.name), id.span)
                })?;
                let ty = self.symbol_types[sym.0 as usize];
                self.record(id.span, ty);
                Ok(())
            }
            Stmt::Expression { span, .. } => Err(Diagnostic::new(
                "for-in/of left-hand side must be a binding or identifier".to_string(),
                *span,
            )),
            other => Err(Diagnostic::new(
                "for-in/of left-hand side must be a binding or identifier".to_string(),
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
                    | Stmt::Return { span, .. }
                    | Stmt::Let { span, .. }
                    | Stmt::Expression { span, .. } => *span,
                },
            )),
        }
    }

    fn check_stmt(
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
            Stmt::Let { name, init, .. } => {
                // Bare `const` without init is rejected in the parser; for-in/of
                // left may be `const name` with no initializer.
                let ty = if let Some(init) = init {
                    self.check_expr(init)?
                } else {
                    Type::Any
                };
                let id = self
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == name.span)
                    .map(|s| s.id)
                    .expect("let binding must be declared");
                self.symbol_types[id.0 as usize] = ty;
                Ok(())
            }
            Stmt::Empty { .. } => Ok(()),
            Stmt::Block { body, .. } => {
                for s in body {
                    self.check_stmt(s, loop_depth, switch_depth, fn_depth, labels)?;
                }
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
                ..
            } => {
                self.check_expr(test)?;
                self.check_stmt(consequent, loop_depth, switch_depth, fn_depth, labels)?;
                if let Some(alt) = alternate {
                    self.check_stmt(alt, loop_depth, switch_depth, fn_depth, labels)?;
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
            }
            | Stmt::ForOf {
                left, right, body, ..
            } => {
                self.check_for_in_of_left(left)?;
                self.check_expr(right)?;
                self.check_stmt(body, loop_depth + 1, switch_depth, fn_depth, labels)
            }
            Stmt::Break { label, span } => {
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
                Ok(())
            }
            Stmt::Continue { label, span } => {
                if let Some(label) = label {
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
                if labels.iter().any(|(n, _)| n == &label.name) {
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
                for case in cases {
                    if let Some(test) = &case.test {
                        self.check_expr(test)?;
                    }
                    for s in &case.body {
                        self.check_stmt(s, loop_depth, switch_depth + 1, fn_depth, labels)?;
                    }
                }
                Ok(())
            }
            Stmt::FunctionDeclaration {
                name, params, body, ..
            } => {
                let id = self
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == name.span)
                    .map(|s| s.id)
                    .expect("function binding must be declared");
                self.symbol_types[id.0 as usize] = Type::Function;
                for p in params {
                    let pid = self
                        .bound
                        .symbols()
                        .iter()
                        .find(|s| s.span == p.span)
                        .map(|s| s.id)
                        .expect("param binding must be declared");
                    self.symbol_types[pid.0 as usize] = Type::Any;
                }
                // Fresh label set inside functions (labels do not cross function boundaries).
                let mut inner_labels = Vec::new();
                self.check_stmt(body, 0, 0, fn_depth + 1, &mut inner_labels)
            }
            Stmt::Return { argument, span } => {
                if fn_depth == 0 {
                    return Err(Diagnostic::new(
                        "Illegal return statement".to_string(),
                        *span,
                    ));
                }
                if let Some(arg) = argument {
                    self.check_expr(arg)?;
                }
                Ok(())
            }
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<Type, Diagnostic> {
        let ty = match expr {
            Expr::Number(n) => {
                self.record(n.span, Type::Number);
                Type::Number
            }
            Expr::String(s) => {
                self.record(s.span, Type::String);
                Type::String
            }
            Expr::Boolean { span, .. } => {
                self.record(*span, Type::Boolean);
                Type::Boolean
            }
            Expr::Null { span } => {
                self.record(*span, Type::Null);
                Type::Null
            }
            Expr::Ident(id) => {
                let sym = self.bound.resolve(id.span).ok_or_else(|| {
                    Diagnostic::new(format!("unresolved identifier `{}`", id.name), id.span)
                })?;
                let ty = self.symbol_types[sym.0 as usize];
                self.record(id.span, ty);
                ty
            }
            Expr::Paren { expr: inner, span } => {
                let ty = self.check_expr(inner)?;
                self.record(*span, ty);
                ty
            }
            Expr::Unary { op, arg, span } => {
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
                let ty = self.check_binary(*op, left_ty, right_ty, *span)?;
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
                let value_ty = self.check_expr(value)?;
                match target.as_ref() {
                    Expr::Ident(id) => {
                        let sym = self.bound.resolve(id.span).ok_or_else(|| {
                            Diagnostic::new(
                                format!("unresolved identifier `{}`", id.name),
                                id.span,
                            )
                        })?;
                        match self.bound.symbol(sym).kind {
                            BindingKind::Const => {
                                return Err(Diagnostic::new(
                                    format!("cannot assign to const binding `{}`", id.name),
                                    *span,
                                ));
                            }
                            BindingKind::Function => {
                                return Err(Diagnostic::new(
                                    format!("cannot assign to function binding `{}`", id.name),
                                    *span,
                                ));
                            }
                            BindingKind::Let => {}
                        }
                        let left_ty = self.symbol_types[sym.0 as usize];
                        let result_ty = if let Some(bin_op) = op.binary_op() {
                            self.check_binary(bin_op, left_ty, value_ty, *span)?
                        } else {
                            value_ty
                        };
                        if left_ty == Type::Any {
                            self.symbol_types[sym.0 as usize] = result_ty;
                        } else if left_ty != result_ty && result_ty != Type::Any {
                            return Err(Diagnostic::new(
                                format!(
                                    "cannot assign type `{result_ty}` to binding of type `{left_ty}`"
                                ),
                                *span,
                            ));
                        }
                        self.record(id.span, self.symbol_types[sym.0 as usize]);
                        self.record(*span, result_ty);
                        result_ty
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            "invalid assignment target".to_string(),
                            *span,
                        ));
                    }
                }
            }
            Expr::Update { arg, span, .. } => {
                match arg.as_ref() {
                    Expr::Ident(id) => {
                        let sym = self.bound.resolve(id.span).ok_or_else(|| {
                            Diagnostic::new(
                                format!("unresolved identifier `{}`", id.name),
                                id.span,
                            )
                        })?;
                        match self.bound.symbol(sym).kind {
                            BindingKind::Const => {
                                return Err(Diagnostic::new(
                                    format!("cannot assign to const binding `{}`", id.name),
                                    *span,
                                ));
                            }
                            BindingKind::Function => {
                                return Err(Diagnostic::new(
                                    format!("cannot assign to function binding `{}`", id.name),
                                    *span,
                                ));
                            }
                            BindingKind::Let => {}
                        }
                        let left_ty = self.symbol_types[sym.0 as usize];
                        if left_ty != Type::Number && left_ty != Type::Any {
                            return Err(Diagnostic::new(
                                format!("update operator cannot be applied to type `{left_ty}`"),
                                *span,
                            ));
                        }
                        if left_ty == Type::Any {
                            self.symbol_types[sym.0 as usize] = Type::Number;
                        }
                        self.record(id.span, Type::Number);
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            "invalid update target".to_string(),
                            *span,
                        ));
                    }
                }
                self.record(*span, Type::Number);
                Type::Number
            }
            Expr::Call {
                callee,
                args,
                span,
            } => {
                let callee_ty = self.check_expr(callee)?;
                for arg in args {
                    self.check_expr(arg)?;
                }
                if callee_ty != Type::Any && callee_ty != Type::Function {
                    return Err(Diagnostic::new(
                        format!("type `{callee_ty}` is not callable"),
                        *span,
                    ));
                }
                self.record(*span, Type::Any);
                Type::Any
            }
        };
        Ok(ty)
    }

    fn record(&mut self, span: Span, ty: Type) {
        self.expr_types.insert(span, ty);
    }

    fn check_unary(&self, op: UnaryOp, arg: Type, span: Span) -> Result<Type, Diagnostic> {
        match op {
            UnaryOp::Plus | UnaryOp::Minus | UnaryOp::BitNot => {
                if arg == Type::Number || arg == Type::Any {
                    Ok(Type::Number)
                } else {
                    Err(Diagnostic::new(
                        format!("unary `{op}` cannot be applied to type `{arg}`"),
                        span,
                    ))
                }
            }
            UnaryOp::Not => Ok(Type::Boolean),
            UnaryOp::TypeOf => Ok(Type::String),
            UnaryOp::Void => Ok(Type::Null),
            UnaryOp::Delete => Ok(Type::Boolean),
        }
    }

    fn check_binary(
        &self,
        op: BinaryOp,
        left: Type,
        right: Type,
        span: Span,
    ) -> Result<Type, Diagnostic> {
        match op {
            BinaryOp::Add => match (left, right) {
                (Type::Number, Type::Number) => Ok(Type::Number),
                (Type::String, Type::String)
                | (Type::String, Type::Number)
                | (Type::Number, Type::String) => Ok(Type::String),
                (Type::Any, Type::String) | (Type::String, Type::Any) => Ok(Type::String),
                (Type::Any, _) | (_, Type::Any) => Ok(Type::Any),
                _ => Err(Diagnostic::new(
                    format!("operator `+` cannot be applied to types `{left}` and `{right}`"),
                    span,
                )),
            },
            BinaryOp::Sub
            | BinaryOp::Mul
            | BinaryOp::Div
            | BinaryOp::Rem
            | BinaryOp::Pow
            | BinaryOp::BitAnd
            | BinaryOp::BitOr
            | BinaryOp::BitXor
            | BinaryOp::Shl
            | BinaryOp::Shr
            | BinaryOp::UShr => {
                if self.is_numberish(left) && self.is_numberish(right) {
                    Ok(Type::Number)
                } else {
                    Err(Diagnostic::new(
                        format!("operator `{op}` cannot be applied to types `{left}` and `{right}`"),
                        span,
                    ))
                }
            }
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                if self.is_numberish(left) && self.is_numberish(right) {
                    Ok(Type::Boolean)
                } else {
                    Err(Diagnostic::new(
                        format!("operator `{op}` cannot be applied to types `{left}` and `{right}`"),
                        span,
                    ))
                }
            }
            BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq => {
                Ok(Type::Boolean)
            }
            BinaryOp::And | BinaryOp::Or | BinaryOp::Nullish => {
                if left == right {
                    Ok(left)
                } else if left == Type::Any || right == Type::Any {
                    Ok(Type::Any)
                } else {
                    // TS-style union collapsed to any for the minimal surface.
                    Ok(Type::Any)
                }
            }
            // Comma yields the RHS type (both sides still typechecked for effects).
            BinaryOp::Comma => Ok(right),
        }
    }

    fn is_numberish(&self, ty: Type) -> bool {
        matches!(ty, Type::Number | Type::Any)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_parser::parse;

    #[test]
    fn bind_let_declares_symbol() {
        let program = parse("let x = 1;").unwrap();
        let bound = bind(program).unwrap();
        assert_eq!(bound.symbols().len(), 1);
        assert_eq!(bound.symbols()[0].name, "x");
        assert_eq!(bound.symbols()[0].kind, BindingKind::Let);
    }

    #[test]
    fn bind_const_declares_symbol() {
        let program = parse("const x = 1;").unwrap();
        let bound = bind(program).unwrap();
        assert_eq!(bound.symbols().len(), 1);
        assert_eq!(bound.symbols()[0].name, "x");
        assert_eq!(bound.symbols()[0].kind, BindingKind::Const);
    }

    #[test]
    fn check_const_rejects_reassignment() {
        let program = parse("const x = 1; x = 2;").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("const") && err.message.contains("x"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn check_const_rejects_update() {
        let program = parse("const x = 1; x++;").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("const") && err.message.contains("x"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn check_const_ok_read() {
        let program = parse("const x = 1; let y = x + 2;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(checked.type_of_symbol(checked.bound.symbols()[0].id), Type::Number);
    }

    #[test]
    fn bind_resolves_reference_to_let() {
        let program = parse("let x = 1; x;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "x");
        let id = bound.resolve(use_span).expect("x should resolve");
        assert_eq!(bound.symbol(id).name, "x");
    }

    #[test]
    fn bind_resolves_ident_in_initializer() {
        let program = parse("let x = 1; let y = x + 2;").unwrap();
        let bound = bind(program).unwrap();
        assert_eq!(bound.symbols().len(), 2);
        let use_span = find_ident_use(&bound.program, "x");
        let id = bound.resolve(use_span).expect("x in init should resolve");
        assert_eq!(bound.symbol(id).name, "x");
    }

    #[test]
    fn bind_unresolved_identifier_errors() {
        let program = parse("y;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("unresolved") && err.message.contains("y"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_let_errors() {
        let program = parse("let x = 1; let x = 2;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("x"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_resolves_call_callee_and_args() {
        let program = parse("let f = 1; let a = 2; f(a);").unwrap();
        let bound = bind(program).unwrap();
        let f_span = find_ident_use(&bound.program, "f");
        let a_span = find_ident_use(&bound.program, "a");
        assert_eq!(bound.symbol(bound.resolve(f_span).unwrap()).name, "f");
        assert_eq!(bound.symbol(bound.resolve(a_span).unwrap()).name, "a");
    }

    #[test]
    fn check_infers_literal_and_let_types() {
        let program = parse(r#"let n = 1; let s = "hi"; let b = true; let z = null;"#).unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "n"), Type::Number);
        assert_eq!(sym_type(&checked, "s"), Type::String);
        assert_eq!(sym_type(&checked, "b"), Type::Boolean);
        assert_eq!(sym_type(&checked, "z"), Type::Null);
    }

    #[test]
    fn check_infers_binary_number_and_string() {
        let program = parse("let a = 1 + 2; let b = \"a\" + \"b\"; let c = 1 + \"x\";").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Number);
        assert_eq!(sym_type(&checked, "b"), Type::String);
        assert_eq!(sym_type(&checked, "c"), Type::String);
    }

    #[test]
    fn check_propagates_binding_types() {
        let program = parse("let x = 1; let y = x; let z = y + 2;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "x"), Type::Number);
        assert_eq!(sym_type(&checked, "y"), Type::Number);
        assert_eq!(sym_type(&checked, "z"), Type::Number);
    }

    #[test]
    fn check_comparison_is_boolean() {
        let program = parse("let ok = 1 < 2;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "ok"), Type::Boolean);
    }

    #[test]
    fn check_unary_ops() {
        let program = parse("let a = -1; let b = !false; let c = typeof 1;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Number);
        assert_eq!(sym_type(&checked, "b"), Type::Boolean);
        assert_eq!(sym_type(&checked, "c"), Type::String);
    }

    #[test]
    fn check_uninitialized_let_is_any() {
        let program = parse("let x;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "x"), Type::Any);
    }

    #[test]
    fn check_arithmetic_on_string_errors() {
        let program = parse(r#"let x = "a" - 1;"#).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("operator") && err.message.contains("string"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn check_unary_minus_on_string_errors() {
        let program = parse(r#"let x = -"a";"#).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("unary") && err.message.contains("string"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn check_call_on_number_errors() {
        let program = parse("let f = 1; f();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn check_records_expr_types() {
        let program = parse("let x = 1 + 2;").unwrap();
        let checked = check(program).unwrap();
        let add_span = find_binary_span(&checked.bound.program, BinaryOp::Add);
        assert_eq!(checked.type_of_expr(add_span), Some(Type::Number));
    }

    fn sym_type(checked: &CheckedProgram, name: &str) -> Type {
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == name)
            .unwrap_or_else(|| panic!("no symbol `{name}`"));
        checked.type_of_symbol(sym.id)
    }

    /// First non-declaration Ident use of `name` (expression reference).
    fn find_ident_use(program: &Program, name: &str) -> Span {
        fn walk_expr(expr: &Expr, name: &str, out: &mut Option<Span>) {
            if out.is_some() {
                return;
            }
            match expr {
                Expr::Ident(id) if id.name == name => *out = Some(id.span),
                Expr::Ident(_)
                | Expr::Number(_)
                | Expr::String(_)
                | Expr::Boolean { .. }
                | Expr::Null { .. } => {}
                Expr::Unary { arg, .. } | Expr::Paren { expr: arg, .. } => {
                    walk_expr(arg, name, out)
                }
                Expr::Binary { left, right, .. } => {
                    walk_expr(left, name, out);
                    walk_expr(right, name, out);
                }
                Expr::Conditional {
                    test,
                    consequent,
                    alternate,
                    ..
                } => {
                    walk_expr(test, name, out);
                    walk_expr(consequent, name, out);
                    walk_expr(alternate, name, out);
                }
                Expr::Assign { target, value, .. } => {
                    walk_expr(target, name, out);
                    walk_expr(value, name, out);
                }
                Expr::Update { arg, .. } => walk_expr(arg, name, out),
                Expr::Call { callee, args, .. } => {
                    walk_expr(callee, name, out);
                    for a in args {
                        walk_expr(a, name, out);
                    }
                }
            }
        }

        fn walk_stmt(stmt: &Stmt, name: &str, out: &mut Option<Span>) {
            if out.is_some() {
                return;
            }
            match stmt {
                Stmt::Expression { expr, .. } => walk_expr(expr, name, out),
                Stmt::Let {
                    init: Some(init), ..
                } => walk_expr(init, name, out),
                Stmt::Let { init: None, .. } | Stmt::Empty { .. } => {}
                Stmt::Block { body, .. } => {
                    for s in body {
                        walk_stmt(s, name, out);
                    }
                }
                Stmt::If {
                    test,
                    consequent,
                    alternate,
                    ..
                } => {
                    walk_expr(test, name, out);
                    walk_stmt(consequent, name, out);
                    if let Some(alt) = alternate {
                        walk_stmt(alt, name, out);
                    }
                }
                Stmt::While { test, body, .. } => {
                    walk_expr(test, name, out);
                    walk_stmt(body, name, out);
                }
                Stmt::DoWhile { body, test, .. } => {
                    walk_stmt(body, name, out);
                    walk_expr(test, name, out);
                }
                Stmt::For {
                    init,
                    test,
                    update,
                    body,
                    ..
                } => {
                    if let Some(init) = init {
                        walk_stmt(init, name, out);
                    }
                    if let Some(t) = test {
                        walk_expr(t, name, out);
                    }
                    if let Some(u) = update {
                        walk_expr(u, name, out);
                    }
                    walk_stmt(body, name, out);
                }
                Stmt::ForIn {
                    left, right, body, ..
                }
                | Stmt::ForOf {
                    left, right, body, ..
                } => {
                    walk_stmt(left, name, out);
                    walk_expr(right, name, out);
                    walk_stmt(body, name, out);
                }
                Stmt::Break { .. } | Stmt::Continue { .. } => {}
                Stmt::Labeled { body, .. } => walk_stmt(body, name, out),
                Stmt::Switch {
                    discriminant,
                    cases,
                    ..
                } => {
                    walk_expr(discriminant, name, out);
                    for case in cases {
                        if let Some(test) = &case.test {
                            walk_expr(test, name, out);
                        }
                        for s in &case.body {
                            walk_stmt(s, name, out);
                        }
                    }
                }
                Stmt::FunctionDeclaration {
                    params, body, ..
                } => {
                    let _ = params;
                    walk_stmt(body, name, out);
                }
                Stmt::Return {
                    argument: Some(arg),
                    ..
                } => walk_expr(arg, name, out),
                Stmt::Return {
                    argument: None, ..
                } => {}
            }
        }

        let mut found = None;
        for stmt in &program.body {
            walk_stmt(stmt, name, &mut found);
            if found.is_some() {
                break;
            }
        }
        found.unwrap_or_else(|| panic!("no ident use of `{name}` found"))
    }

    fn find_binary_span(program: &Program, op: BinaryOp) -> Span {
        fn walk(expr: &Expr, op: BinaryOp, out: &mut Option<Span>) {
            if out.is_some() {
                return;
            }
            match expr {
                Expr::Binary {
                    left,
                    op: bop,
                    right,
                    span,
                } => {
                    if *bop == op {
                        *out = Some(*span);
                        return;
                    }
                    walk(left, op, out);
                    walk(right, op, out);
                }
                Expr::Unary { arg, .. }
                | Expr::Paren { expr: arg, .. }
                | Expr::Update { arg, .. } => walk(arg, op, out),
                Expr::Conditional {
                    test,
                    consequent,
                    alternate,
                    ..
                } => {
                    walk(test, op, out);
                    walk(consequent, op, out);
                    walk(alternate, op, out);
                }
                Expr::Assign { target, value, .. } => {
                    walk(target, op, out);
                    walk(value, op, out);
                }
                Expr::Call { callee, args, .. } => {
                    walk(callee, op, out);
                    for a in args {
                        walk(a, op, out);
                    }
                }
                _ => {}
            }
        }
        fn walk_stmt(stmt: &Stmt, op: BinaryOp, out: &mut Option<Span>) {
            if out.is_some() {
                return;
            }
            match stmt {
                Stmt::Expression { expr, .. } => walk(expr, op, out),
                Stmt::Let {
                    init: Some(init), ..
                } => walk(init, op, out),
                Stmt::Block { body, .. } => {
                    for s in body {
                        walk_stmt(s, op, out);
                    }
                }
                Stmt::If {
                    test,
                    consequent,
                    alternate,
                    ..
                } => {
                    walk(test, op, out);
                    walk_stmt(consequent, op, out);
                    if let Some(alt) = alternate {
                        walk_stmt(alt, op, out);
                    }
                }
                Stmt::While { test, body, .. } => {
                    walk(test, op, out);
                    walk_stmt(body, op, out);
                }
                Stmt::DoWhile { body, test, .. } => {
                    walk_stmt(body, op, out);
                    walk(test, op, out);
                }
                Stmt::For {
                    init,
                    test,
                    update,
                    body,
                    ..
                } => {
                    if let Some(init) = init {
                        walk_stmt(init, op, out);
                    }
                    if let Some(t) = test {
                        walk(t, op, out);
                    }
                    if let Some(u) = update {
                        walk(u, op, out);
                    }
                    walk_stmt(body, op, out);
                }
                Stmt::ForIn {
                    left, right, body, ..
                }
                | Stmt::ForOf {
                    left, right, body, ..
                } => {
                    walk_stmt(left, op, out);
                    walk(right, op, out);
                    walk_stmt(body, op, out);
                }
                _ => {}
            }
        }

        let mut found = None;
        for stmt in &program.body {
            walk_stmt(stmt, op, &mut found);
        }
        found.expect("binary op not found")
    }
}
