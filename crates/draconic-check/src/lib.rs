//! Binder (scopes + symbol resolution) and Checker (TypeScript-inspired).
//! Binder: ROADMAP B04. Checker: ROADMAP B05.

use draconic_ast::{BinaryOp, Expr, Program, Stmt, UnaryOp};
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
}

/// TypeScript-inspired types for the minimal Program surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    Number,
    String,
    Boolean,
    Null,
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

struct Binder {
    /// Program-level scope: name → symbol id.
    scope: HashMap<String, SymbolId>,
    symbols: Vec<Symbol>,
    resolutions: HashMap<Span, SymbolId>,
}

impl Binder {
    fn new() -> Self {
        Self {
            scope: HashMap::new(),
            symbols: Vec::new(),
            resolutions: HashMap::new(),
        }
    }

    fn bind_program(&mut self, program: Program) -> Result<BoundProgram, Diagnostic> {
        // Pass 1: collect top-level `let` bindings (program scope).
        for stmt in &program.body {
            if let Stmt::Let { name, .. } = stmt {
                self.declare(name.name.clone(), name.span)?;
            }
        }

        // Pass 2: resolve identifier uses in initializers and expressions.
        for stmt in &program.body {
            self.bind_stmt(stmt)?;
        }

        Ok(BoundProgram {
            program,
            symbols: std::mem::take(&mut self.symbols),
            resolutions: std::mem::take(&mut self.resolutions),
        })
    }

    fn declare(&mut self, name: String, span: Span) -> Result<SymbolId, Diagnostic> {
        if self.scope.contains_key(&name) {
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
        });
        self.scope.insert(name, id);
        Ok(id)
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
        }
    }

    fn bind_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Ident(id) => {
                let Some(sym) = self.scope.get(&id.name).copied() else {
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
        for stmt in &self.bound.program.body {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    fn check_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Expression { expr, .. } => {
                self.check_expr(expr)?;
                Ok(())
            }
            Stmt::Let { name, init, .. } => {
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
            Expr::Call {
                callee,
                args,
                span,
            } => {
                let callee_ty = self.check_expr(callee)?;
                for arg in args {
                    self.check_expr(arg)?;
                }
                // Minimal surface has no function values yet.
                if callee_ty != Type::Any {
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
            UnaryOp::Plus | UnaryOp::Minus => {
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
            BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
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
            BinaryOp::And | BinaryOp::Or => {
                if left == right {
                    Ok(left)
                } else if left == Type::Any || right == Type::Any {
                    Ok(Type::Any)
                } else {
                    // TS-style union collapsed to any for the minimal surface.
                    Ok(Type::Any)
                }
            }
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
                Expr::Call { callee, args, .. } => {
                    walk_expr(callee, name, out);
                    for a in args {
                        walk_expr(a, name, out);
                    }
                }
            }
        }

        let mut found = None;
        for stmt in &program.body {
            match stmt {
                Stmt::Expression { expr, .. } => walk_expr(expr, name, &mut found),
                Stmt::Let {
                    init: Some(init), ..
                } => walk_expr(init, name, &mut found),
                Stmt::Let { init: None, .. } | Stmt::Empty { .. } => {}
            }
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
                Expr::Unary { arg, .. } | Expr::Paren { expr: arg, .. } => walk(arg, op, out),
                Expr::Call { callee, args, .. } => {
                    walk(callee, op, out);
                    for a in args {
                        walk(a, op, out);
                    }
                }
                _ => {}
            }
        }
        let mut found = None;
        for stmt in &program.body {
            match stmt {
                Stmt::Expression { expr, .. } => walk(expr, op, &mut found),
                Stmt::Let {
                    init: Some(init), ..
                } => walk(init, op, &mut found),
                _ => {}
            }
        }
        found.expect("binary op not found")
    }
}
