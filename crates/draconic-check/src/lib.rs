//! Binder (scopes + symbol resolution) and Checker (TypeScript-inspired).
//! Binder: ROADMAP B04. Checker: ROADMAP B05.

use draconic_ast::{
    Arg, ArrayElement, ArrayPatternElement, ArrowBody, BinaryOp, BindingKind, BindingPattern,
    ClassElement, Expr, ObjectKey, ObjectPatternProp, ObjectProp, Param, Program, Stmt, TypeAnn,
    UnaryOp,
};
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
    /// `with` nesting depth when this binding was declared (0 = outside any `with`).
    /// Used so identifier uses inside `with` only rewrite to Locals declared in the
    /// innermost with body; outer names stay bare for Object Environment shadowing.
    pub with_depth: u32,
}

/// Unboxed native / systems types (T05). Outside the JS value heap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
}

impl NativeType {
    fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "i8" => Self::I8,
            "i16" => Self::I16,
            "i32" => Self::I32,
            "i64" => Self::I64,
            "u8" => Self::U8,
            "u16" => Self::U16,
            "u32" => Self::U32,
            "u64" => Self::U64,
            "f32" => Self::F32,
            "f64" => Self::F64,
            _ => return None,
        })
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::I8 => "i8",
            Self::I16 => "i16",
            Self::I32 => "i32",
            Self::I64 => "i64",
            Self::U8 => "u8",
            Self::U16 => "u16",
            Self::U32 => "u32",
            Self::U64 => "u64",
            Self::F32 => "f32",
            Self::F64 => "f64",
        }
    }

    fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }
}

/// TypeScript-inspired types for the minimal Program surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Type {
    Number,
    BigInt,
    String,
    Boolean,
    Null,
    /// Callable function value (declaration or expression).
    Function,
    /// Ordinary object value without a known shape.
    Object,
    /// Structural object type; index into the shape table on `CheckedProgram`.
    Shape(u32),
    /// Union type; index into the unions table on `CheckedProgram`.
    Union(u32),
    /// Intersection type; index into the intersections table on `CheckedProgram`.
    Intersection(u32),
    /// Open type parameter while checking a generic body (T04); unique id.
    TypeParam(u32),
    /// Generic function signature; index into the generic_fns table (T04).
    GenericFn(u32),
    /// Unboxed native type (`i32`, `f64`, …); T05.
    Native(NativeType),
    /// Flexible / unannotated (e.g. `let x;` with no initializer).
    Any,
}

/// Generic function signature stored for call-site instantiation (T04).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GenericFnSig {
    pub type_params: Vec<String>,
    pub param_types: Vec<Option<TypeAnn>>,
    pub return_type: Option<TypeAnn>,
}

/// Generic type alias body (T04).
#[derive(Debug, Clone, PartialEq, Eq)]
struct GenericAlias {
    params: Vec<String>,
    body: TypeAnn,
}

/// Property list for a structural object type (`Type::Shape`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectShape {
    pub props: Vec<(String, Type)>,
}

/// Members of a union type (`Type::Union`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnionType {
    pub members: Vec<Type>,
}

/// Members of an intersection type (`Type::Intersection`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntersectionType {
    pub members: Vec<Type>,
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Type::Number => "number",
            Type::BigInt => "bigint",
            Type::String => "string",
            Type::Boolean => "boolean",
            Type::Null => "null",
            Type::Function => "function",
            Type::Object => "object",
            Type::Shape(_) => "object",
            Type::Union(_) => "union",
            Type::Intersection(_) => "intersection",
            Type::TypeParam(_) => "type parameter",
            Type::GenericFn(_) => "function",
            Type::Native(n) => n.as_str(),
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
    /// Structural object shapes referenced by `Type::Shape`.
    shapes: Vec<ObjectShape>,
    /// Union members referenced by `Type::Union`.
    unions: Vec<UnionType>,
    /// Intersection members referenced by `Type::Intersection`.
    intersections: Vec<IntersectionType>,
    /// Generic function signatures referenced by `Type::GenericFn`.
    generic_fns: Vec<GenericFnSig>,
}

impl CheckedProgram {
    pub fn type_of_symbol(&self, id: SymbolId) -> Type {
        self.symbol_types[id.0 as usize]
    }

    pub fn type_of_expr(&self, span: Span) -> Option<Type> {
        self.expr_types.get(&span).copied()
    }

    pub fn shapes(&self) -> &[ObjectShape] {
        &self.shapes
    }

    pub fn unions(&self) -> &[UnionType] {
        &self.unions
    }

    pub fn intersections(&self) -> &[IntersectionType] {
        &self.intersections
    }

    pub fn generic_fns(&self) -> &[GenericFnSig] {
        &self.generic_fns
    }

    /// Pretty-print a type, expanding structural shapes and unions/intersections.
    pub fn format_type(&self, ty: Type) -> String {
        format_type_full(ty, &self.shapes, &self.unions, &self.intersections)
    }
}

fn format_type_full(
    ty: Type,
    shapes: &[ObjectShape],
    unions: &[UnionType],
    intersections: &[IntersectionType],
) -> String {
    match ty {
        Type::Shape(id) => {
            let Some(shape) = shapes.get(id as usize) else {
                return "object".to_string();
            };
            let props: Vec<String> = shape
                .props
                .iter()
                .map(|(n, t)| {
                    format!("{n}: {}", format_type_full(*t, shapes, unions, intersections))
                })
                .collect();
            format!("{{ {} }}", props.join("; "))
        }
        Type::Union(id) => {
            let Some(u) = unions.get(id as usize) else {
                return "union".to_string();
            };
            u.members
                .iter()
                .map(|t| format_type_full(*t, shapes, unions, intersections))
                .collect::<Vec<_>>()
                .join(" | ")
        }
        Type::Intersection(id) => {
            let Some(i) = intersections.get(id as usize) else {
                return "intersection".to_string();
            };
            i.members
                .iter()
                .map(|t| format_type_full(*t, shapes, unions, intersections))
                .collect::<Vec<_>>()
                .join(" & ")
        }
        Type::TypeParam(_) => "type parameter".to_string(),
        Type::GenericFn(_) => "function".to_string(),
        Type::Native(n) => n.as_str().to_string(),
        other => other.to_string(),
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
    let shapes = checker.shapes;
    let unions = checker.unions;
    let intersections = checker.intersections;
    let generic_fns = checker.generic_fns;
    Ok(CheckedProgram {
        bound,
        symbol_types,
        expr_types,
        shapes,
        unions,
        intersections,
        generic_fns,
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

/// If `param` appears in LexicallyDeclaredNames of the catch Block, return the
/// conflicting name and its declaration span. Annex B.3.4 allows the same name
/// in VarDeclaredNames (`var`); only lexical `let`/`const`/`class`/`function`
/// at the top level of the catch block are rejected.
fn catch_lexical_conflict(param: &str, handler: &Stmt) -> Option<(String, Span)> {
    let body: &[Stmt] = match handler {
        Stmt::Block { body, .. } => body.as_slice(),
        other => std::slice::from_ref(other),
    };
    for stmt in body {
        if let Some(span) = catch_stmt_lexical_name(stmt, param) {
            return Some((param.to_string(), span));
        }
    }
    None
}

fn catch_stmt_lexical_name(stmt: &Stmt, param: &str) -> Option<Span> {
    let mut s = stmt;
    while let Stmt::Labeled { body, .. } = s {
        s = body;
    }
    match s {
        Stmt::Let {
            kind: BindingKind::Let | BindingKind::Const,
            binding,
            ..
        } => {
            let mut found = None;
            binding.for_each_ident(&mut |id| {
                if found.is_none() && id.name == param {
                    found = Some(id.span);
                }
            });
            found
        }
        Stmt::ClassDeclaration { name, .. } | Stmt::FunctionDeclaration { name, .. }
            if name.name == param =>
        {
            Some(name.span)
        }
        _ => None,
    }
}

struct Binder {
    /// Scope stack (innermost last): name → symbol id.
    scopes: Vec<HashMap<String, SymbolId>>,
    /// Indices into `scopes` that are var environments (program + function).
    var_env_indices: Vec<usize>,
    /// Host globals (e.g. `Math`): resolve after lexical scopes so `let Math` can shadow.
    builtins: HashMap<String, SymbolId>,
    symbols: Vec<Symbol>,
    resolutions: HashMap<Span, SymbolId>,
    /// Nesting depth of enclosing `with` statements.
    with_depth: u32,
}

impl Binder {
    fn new() -> Self {
        let mut binder = Self {
            scopes: vec![HashMap::new()],
            // Program/script body is a var environment.
            var_env_indices: vec![0],
            builtins: HashMap::new(),
            symbols: Vec::new(),
            resolutions: HashMap::new(),
            with_depth: 0,
        };
        binder.install_builtin("Math", BindingKind::Const);
        binder.install_builtin("Number", BindingKind::Const);
        binder.install_builtin("NaN", BindingKind::Const);
        binder.install_builtin("Infinity", BindingKind::Const);
        binder.install_builtin("Symbol", BindingKind::Const);
        binder.install_builtin("Promise", BindingKind::Const);
        binder.install_builtin("Proxy", BindingKind::Const);
        binder.install_builtin("Reflect", BindingKind::Const);
        // E15.01: global object basics
        binder.install_builtin("undefined", BindingKind::Const);
        binder.install_builtin("globalThis", BindingKind::Const);
        binder.install_builtin("Object", BindingKind::Const);
        binder.install_builtin("Function", BindingKind::Const);
        binder.install_builtin("Array", BindingKind::Const);
        binder.install_builtin("String", BindingKind::Const);
        binder.install_builtin("Boolean", BindingKind::Const);
        // E15.02: Error constructors
        binder.install_builtin("Error", BindingKind::Const);
        binder.install_builtin("TypeError", BindingKind::Const);
        binder.install_builtin("RangeError", BindingKind::Const);
        binder.install_builtin("ReferenceError", BindingKind::Const);
        binder.install_builtin("SyntaxError", BindingKind::Const);
        binder.install_builtin("URIError", BindingKind::Const);
        binder.install_builtin("EvalError", BindingKind::Const);
        binder.install_builtin("AggregateError", BindingKind::Const);
        // E15.03: global number-parsing / predicate functions
        binder.install_builtin("parseInt", BindingKind::Const);
        binder.install_builtin("parseFloat", BindingKind::Const);
        binder.install_builtin("isNaN", BindingKind::Const);
        binder.install_builtin("isFinite", BindingKind::Const);
        // E15.04: URI encode/decode
        binder.install_builtin("encodeURI", BindingKind::Const);
        binder.install_builtin("decodeURI", BindingKind::Const);
        binder.install_builtin("encodeURIComponent", BindingKind::Const);
        binder.install_builtin("decodeURIComponent", BindingKind::Const);
        // E15.05: JSON
        binder.install_builtin("JSON", BindingKind::Const);
        // E15.06: Date
        binder.install_builtin("Date", BindingKind::Const);
        // E15.07: RegExp
        binder.install_builtin("RegExp", BindingKind::Const);
        // E15.08: Map / Set
        binder.install_builtin("Map", BindingKind::Const);
        binder.install_builtin("Set", BindingKind::Const);
        // E15.09: WeakMap / WeakSet
        binder.install_builtin("WeakMap", BindingKind::Const);
        binder.install_builtin("WeakSet", BindingKind::Const);
        // E15.10: ArrayBuffer / DataView / TypedArrays
        binder.install_builtin("ArrayBuffer", BindingKind::Const);
        binder.install_builtin("DataView", BindingKind::Const);
        binder.install_builtin("Int8Array", BindingKind::Const);
        binder.install_builtin("Uint8Array", BindingKind::Const);
        binder.install_builtin("Uint8ClampedArray", BindingKind::Const);
        binder.install_builtin("Int16Array", BindingKind::Const);
        binder.install_builtin("Uint16Array", BindingKind::Const);
        binder.install_builtin("Int32Array", BindingKind::Const);
        binder.install_builtin("Uint32Array", BindingKind::Const);
        binder.install_builtin("Float32Array", BindingKind::Const);
        binder.install_builtin("Float64Array", BindingKind::Const);
        binder.install_builtin("BigInt64Array", BindingKind::Const);
        binder.install_builtin("BigUint64Array", BindingKind::Const);
        // E16.01: direct eval
        binder.install_builtin("eval", BindingKind::Const);
        // E18.01: Annex B escape / unescape
        binder.install_builtin("escape", BindingKind::Const);
        binder.install_builtin("unescape", BindingKind::Const);
        binder
    }

    /// Register a host global. Not placed in lexical scopes so programs may shadow it.
    fn install_builtin(&mut self, name: &str, kind: BindingKind) {
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.to_string(),
            span: Span::dummy(),
            kind,
            with_depth: 0,
        });
        self.builtins.insert(name.to_string(), id);
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
            self.declare_list_item(stmt)?;
        }
        for stmt in stmts {
            self.bind_stmt(stmt)?;
        }
        Ok(())
    }

    /// Hoistable declarations for one statement-list item (Annex B.3.2 peels labels).
    fn declare_list_item(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Let {
                kind: BindingKind::Var,
                binding,
                ..
            } => self.declare_var_binding(binding),
            Stmt::Let { kind, binding, .. } => self.declare_binding(binding, *kind),
            Stmt::ClassDeclaration { name, .. } => {
                self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                Ok(())
            }
            Stmt::FunctionDeclaration { name, .. } => {
                // Annex B.3.2: outer var-like binding already hosts this name — do not
                // shadow with a block-local binding (IR + uses share the outer symbol).
                if let Some(existing) = self.resolve_name(&name.name) {
                    let in_current = self
                        .scopes
                        .last()
                        .is_some_and(|s| s.contains_key(&name.name));
                    if !in_current
                        && matches!(
                            self.symbols[existing.0 as usize].kind,
                            BindingKind::Function | BindingKind::Var
                        )
                    {
                        self.declare_annex_b_function_span(name);
                        return Ok(());
                    }
                }
                // `var f` then `function f` in the same list: reuse var binding.
                let scope = self.scopes.last().expect("scope stack non-empty");
                if let Some(&existing) = scope.get(&name.name) {
                    if self.symbols[existing.0 as usize].kind == BindingKind::Var {
                        self.declare_annex_b_function_span(name);
                        return Ok(());
                    }
                }
                self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                Ok(())
            }
            // Annex B.3.2: `label: function f() {}` hoists `f` in this list.
            Stmt::Labeled { body, .. } => self.declare_list_item(body),
            // Annex B.3.2: block-level `function` → enclosing var-like binding.
            // E18.14: also hoist nested `var` into the current var environment.
            Stmt::Block { body, .. } => {
                for s in body {
                    self.declare_annex_b_block_functions(s)?;
                    self.hoist_vars_from_stmt(s)?;
                }
                Ok(())
            }
            // Annex B.3.4: bare `function` as if/else Statement clause.
            // Annex B.3.2: also hoist from block bodies (`if (c) { function f(){} }`).
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                self.declare_if_function_clause(consequent)?;
                if let Some(alt) = alternate {
                    self.declare_if_function_clause(alt)?;
                }
                self.declare_annex_b_block_functions(consequent)?;
                if let Some(alt) = alternate {
                    self.declare_annex_b_block_functions(alt)?;
                }
                self.hoist_vars_from_stmt(consequent)?;
                if let Some(alt) = alternate {
                    self.hoist_vars_from_stmt(alt)?;
                }
                Ok(())
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                self.declare_annex_b_block_functions(body)?;
                self.hoist_vars_from_stmt(body)
            }
            Stmt::For { init, body, .. } => {
                if let Some(init) = init {
                    self.declare_annex_b_block_functions(init)?;
                    self.hoist_vars_from_stmt(init)?;
                }
                self.declare_annex_b_block_functions(body)?;
                self.hoist_vars_from_stmt(body)
            }
            Stmt::ForIn { body, left, .. } | Stmt::ForOf { body, left, .. } => {
                self.declare_annex_b_block_functions(left)?;
                self.hoist_vars_from_stmt(left)?;
                self.declare_annex_b_block_functions(body)?;
                self.hoist_vars_from_stmt(body)
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                self.declare_annex_b_block_functions(block)?;
                self.hoist_vars_from_stmt(block)?;
                if let Some(handler) = handler {
                    self.declare_annex_b_block_functions(handler)?;
                    self.hoist_vars_from_stmt(handler)?;
                }
                if let Some(finalizer) = finalizer {
                    self.declare_annex_b_block_functions(finalizer)?;
                    self.hoist_vars_from_stmt(finalizer)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Hoist `var` declarations from a nested statement into the current var environment.
    fn hoist_vars_from_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        let mut s = stmt;
        while let Stmt::Labeled { body, .. } = s {
            s = body;
        }
        match s {
            Stmt::Let {
                kind: BindingKind::Var,
                binding,
                ..
            } => self.declare_var_binding(binding),
            Stmt::Block { body, .. } => {
                for child in body {
                    self.hoist_vars_from_stmt(child)?;
                }
                Ok(())
            }
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                self.hoist_vars_from_stmt(consequent)?;
                if let Some(alt) = alternate {
                    self.hoist_vars_from_stmt(alt)?;
                }
                Ok(())
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                self.hoist_vars_from_stmt(body)
            }
            Stmt::For { init, body, .. } => {
                if let Some(init) = init {
                    self.hoist_vars_from_stmt(init)?;
                }
                self.hoist_vars_from_stmt(body)
            }
            Stmt::ForIn { body, left, .. } | Stmt::ForOf { body, left, .. } => {
                self.hoist_vars_from_stmt(left)?;
                self.hoist_vars_from_stmt(body)
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                self.hoist_vars_from_stmt(block)?;
                if let Some(handler) = handler {
                    self.hoist_vars_from_stmt(handler)?;
                }
                if let Some(finalizer) = finalizer {
                    self.hoist_vars_from_stmt(finalizer)?;
                }
                Ok(())
            }
            // Nested function bodies have their own var environment — do not hoist out.
            Stmt::FunctionDeclaration { .. } | Stmt::ClassDeclaration { .. } => Ok(()),
            _ => Ok(()),
        }
    }

    fn declare_var_binding(&mut self, binding: &BindingPattern) -> Result<(), Diagnostic> {
        let mut err = None;
        binding.for_each_ident(&mut |id| {
            if err.is_some() {
                return;
            }
            if let Err(e) = self.declare_var(id.name.clone(), id.span) {
                err = Some(e);
            }
        });
        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    /// Declare a function-scoped `var` in the nearest var environment.
    /// Redeclaration with `var`/`function` is allowed; creates a span alias for IR.
    fn declare_var(&mut self, name: String, span: Span) -> Result<SymbolId, Diagnostic> {
        let env_idx = *self
            .var_env_indices
            .last()
            .expect("var environment stack non-empty");
        if let Some(&existing) = self.scopes[env_idx].get(&name) {
            let existing_kind = self.symbols[existing.0 as usize].kind;
            match existing_kind {
                BindingKind::Var | BindingKind::Function => {
                    self.declare_var_span(&name, span);
                    return Ok(existing);
                }
                BindingKind::Let | BindingKind::Const => {
                    return Err(Diagnostic::new(
                        format!("duplicate declaration of `{name}`"),
                        span,
                    ));
                }
            }
        }
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.clone(),
            span,
            kind: BindingKind::Var,
            with_depth: self.with_depth,
        });
        self.scopes[env_idx].insert(name, id);
        Ok(id)
    }

    /// Extra symbol keyed by declaration span for IR; uses keep the scoped binding.
    fn declare_var_span(&mut self, name: &str, span: Span) {
        if self
            .symbols
            .iter()
            .any(|s| s.span == span && s.name == name)
        {
            return;
        }
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.to_string(),
            span,
            kind: BindingKind::Var,
            with_depth: self.with_depth,
        });
    }

    /// Annex B.3.2: walk a statement and hoist nested block-level function names
    /// into the current (enclosing) scope as var-like bindings.
    fn declare_annex_b_block_functions(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        let mut s = stmt;
        while let Stmt::Labeled { body, .. } = s {
            s = body;
        }
        match s {
            Stmt::Block { body, .. } => {
                for child in body {
                    self.declare_annex_b_block_functions(child)?;
                }
                Ok(())
            }
            Stmt::FunctionDeclaration { name, .. } => self.declare_annex_b_function_name(name),
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                self.declare_if_function_clause(consequent)?;
                if let Some(alt) = alternate {
                    self.declare_if_function_clause(alt)?;
                }
                self.declare_annex_b_block_functions(consequent)?;
                if let Some(alt) = alternate {
                    self.declare_annex_b_block_functions(alt)?;
                }
                Ok(())
            }
            Stmt::While { body, .. } | Stmt::DoWhile { body, .. } => {
                self.declare_annex_b_block_functions(body)
            }
            Stmt::For { init, body, .. } => {
                if let Some(init) = init {
                    self.declare_annex_b_block_functions(init)?;
                }
                self.declare_annex_b_block_functions(body)
            }
            Stmt::ForIn { body, .. } | Stmt::ForOf { body, .. } => {
                self.declare_annex_b_block_functions(body)
            }
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                self.declare_annex_b_block_functions(block)?;
                if let Some(handler) = handler {
                    self.declare_annex_b_block_functions(handler)?;
                }
                if let Some(finalizer) = finalizer {
                    self.declare_annex_b_block_functions(finalizer)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    /// Annex B.3.2 / B.3.4: declare a function name in the current scope (var-like).
    fn declare_annex_b_function_name(
        &mut self,
        name: &draconic_ast::Ident,
    ) -> Result<(), Diagnostic> {
        let scope = self.scopes.last().expect("scope stack non-empty");
        if let Some(&existing) = scope.get(&name.name) {
            let existing_kind = self.symbols[existing.0 as usize].kind;
            if !matches!(existing_kind, BindingKind::Function | BindingKind::Var) {
                return Err(Diagnostic::new(
                    format!("duplicate declaration of `{}`", name.name),
                    name.span,
                ));
            }
            self.declare_annex_b_function_span(name);
            return Ok(());
        }
        self.declare(name.name.clone(), name.span, BindingKind::Function)?;
        Ok(())
    }

    /// Extra symbol keyed by declaration span for IR; uses keep the scoped binding.
    fn declare_annex_b_function_span(&mut self, name: &draconic_ast::Ident) {
        if self
            .symbols
            .iter()
            .any(|s| s.span == name.span && s.name == name.name)
        {
            return;
        }
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.name.clone(),
            span: name.span,
            kind: BindingKind::Function,
            with_depth: self.with_depth,
        });
    }

    /// Annex B.3.4: hoist `function f` / `label: function f` when it is the if/else clause.
    /// Does not walk into blocks (`if (c) { function f(){} }` is B.3.2 / B.3.3).
    fn declare_if_function_clause(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        let mut s = stmt;
        while let Stmt::Labeled { body, .. } = s {
            s = body;
        }
        let Stmt::FunctionDeclaration { name, .. } = s else {
            return Ok(());
        };
        self.declare_annex_b_function_name(name)
    }

    fn declare_binding(
        &mut self,
        binding: &BindingPattern,
        kind: BindingKind,
    ) -> Result<(), Diagnostic> {
        let mut err = None;
        binding.for_each_ident(&mut |id| {
            if err.is_some() {
                return;
            }
            if let Err(e) = self.declare(id.name.clone(), id.span, kind) {
                err = Some(e);
            }
        });
        match err {
            Some(e) => Err(e),
            None => Ok(()),
        }
    }

    fn push_scope(&mut self) {
        self.push_scope_kind(false);
    }

    /// Push a scope. `is_var_env` is true for function scopes (params + body share it).
    fn push_scope_kind(&mut self, is_var_env: bool) {
        self.scopes.push(HashMap::new());
        if is_var_env {
            self.var_env_indices.push(self.scopes.len() - 1);
        }
    }

    fn pop_scope(&mut self) {
        let idx = self.scopes.len() - 1;
        if self.var_env_indices.last().copied() == Some(idx) {
            self.var_env_indices.pop();
        }
        self.scopes.pop();
    }

    fn declare(
        &mut self,
        name: String,
        span: Span,
        kind: BindingKind,
    ) -> Result<SymbolId, Diagnostic> {
        let scope = self.scopes.last_mut().expect("scope stack non-empty");
        if let Some(&existing) = scope.get(&name) {
            // `var` then `let`/`const` in the same var environment is a conflict.
            let existing_kind = self.symbols[existing.0 as usize].kind;
            if existing_kind == BindingKind::Var
                && matches!(kind, BindingKind::Let | BindingKind::Const)
            {
                return Err(Diagnostic::new(
                    format!("duplicate declaration of `{name}`"),
                    span,
                ));
            }
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
            with_depth: self.with_depth,
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
        self.builtins.get(name).copied()
    }

    /// Resolve an identifier use. Inside `with`, only bindings declared in the
    /// innermost with body (or nested deeper) become static resolutions; outer
    /// names stay unresolved so the JS backend can emit bare idents for the
    /// Object Environment chain.
    fn bind_ident_use(&mut self, id: &draconic_ast::Ident) -> Result<(), Diagnostic> {
        if let Some(sym) = self.resolve_name(&id.name) {
            let decl_depth = self.symbols[sym.0 as usize].with_depth;
            if self.with_depth == 0 || decl_depth >= self.with_depth {
                self.resolutions.insert(id.span, sym);
            }
            return Ok(());
        }
        if self.with_depth > 0 {
            return Ok(());
        }
        Err(Diagnostic::new(
            format!("unresolved identifier `{name}`", name = id.name),
            id.span,
        ))
    }

    fn bind_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Expression { expr, .. } => self.bind_expr(expr),
            Stmt::Let { binding, init, .. } => {
                if let Some(init) = init {
                    self.bind_expr(init)?;
                }
                self.bind_pattern_defaults(binding)?;
                Ok(())
            }
            Stmt::TypeAlias { .. } => Ok(()),
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
                // `for (let/const …)` introduces a loop-scoped binding visible in
                // test, update, and body. `for (var …)` is function-scoped (already hoisted).
                if let Some(Stmt::Let {
                    kind,
                    binding,
                    init: let_init,
                    ..
                }) = init.as_deref()
                {
                    if matches!(kind, BindingKind::Let | BindingKind::Const) {
                        self.push_scope();
                        self.declare_binding(binding, *kind)?;
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
                        return Ok(());
                    }
                }
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
            Stmt::ForIn {
                left, right, body, ..
            } => self.bind_for_in_of(left, right, body, true),
            Stmt::ForOf {
                left, right, body, ..
            } => self.bind_for_in_of(left, right, body, false),
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
                    self.declare_list_item(stmt)?;
                }
                for stmt in all_stmts {
                    self.bind_stmt(stmt)?;
                }
                self.pop_scope();
                Ok(())
            }
            Stmt::FunctionDeclaration { params, body, .. } => {
                // Name already declared in the enclosing list's first pass.
                // Function scope is a var environment.
                self.push_scope_kind(true);
                self.bind_params(params)?;
                self.install_arguments_object()?;
                // Body is a Block; bind its statements in the param scope (no extra
                // block scope layer needed beyond the block's own push).
                self.bind_stmt(body)?;
                self.pop_scope();
                Ok(())
            }
            Stmt::ClassDeclaration {
                super_class,
                body,
                ..
            } => {
                // Name already declared in the enclosing list's first pass.
                if let Some(sc) = super_class {
                    self.bind_expr(sc)?;
                }
                for el in body {
                    match el {
                        ClassElement::Constructor { params, body, .. }
                        | ClassElement::Method { params, body, .. }
                        | ClassElement::Accessor { params, body, .. } => {
                            self.push_scope_kind(true);
                            self.bind_params(params)?;
                            self.install_arguments_object()?;
                            self.bind_stmt(body)?;
                            self.pop_scope();
                        }
                    }
                }
                Ok(())
            }
            Stmt::Return { argument, .. } => {
                if let Some(arg) = argument {
                    self.bind_expr(arg)?;
                }
                Ok(())
            }
            Stmt::Throw { argument, .. } => self.bind_expr(argument),
            Stmt::Try {
                block,
                handler_param,
                handler,
                finalizer,
                ..
            } => {
                self.bind_stmt(block)?;
                if let Some(handler) = handler {
                    // Catch binding is scoped to the catch block only.
                    // Early error: CatchParameter ∩ LexicallyDeclaredNames(Block).
                    // Annex B.3.4: CatchParameter ∩ VarDeclaredNames(Block) is allowed.
                    if let Some(param) = handler_param {
                        if let Some((name, span)) =
                            catch_lexical_conflict(&param.name, handler)
                        {
                            return Err(Diagnostic::new(
                                format!("duplicate declaration of `{name}`"),
                                span,
                            ));
                        }
                    }
                    self.push_scope();
                    if let Some(param) = handler_param {
                        self.declare(param.name.clone(), param.span, BindingKind::Let)?;
                    }
                    self.bind_stmt(handler)?;
                    self.pop_scope();
                }
                if let Some(finalizer) = finalizer {
                    self.bind_stmt(finalizer)?;
                }
                Ok(())
            }
            Stmt::With { object, body, .. } => {
                self.bind_expr(object)?;
                self.with_depth += 1;
                let result = self.bind_stmt(body);
                self.with_depth -= 1;
                result
            }
            Stmt::ImportDeclaration { span, .. }
            | Stmt::ExportNamedDeclaration { span, .. }
            | Stmt::ExportDefaultDeclaration { span, .. } => Err(Diagnostic::new(
                "import/export must be linked before bind/check".to_string(),
                *span,
            )),
        }
    }

    fn bind_for_in_of(
        &mut self,
        left: &Stmt,
        right: &Expr,
        body: &Stmt,
        is_for_in: bool,
    ) -> Result<(), Diagnostic> {
        // `for (let/const name in/of right)` — loop-scoped binding for name.
        // `for (var name in/of right)` — function-scoped (already hoisted).
        // Annex B.3.5: `for (var name = init in right)` only.
        if let Stmt::Let {
            kind,
            binding,
            init,
            ..
        } = left
        {
            if init.is_some() {
                if !(is_for_in && *kind == BindingKind::Var) {
                    return Err(Diagnostic::new(
                        "for-in/of binding cannot have an initializer".to_string(),
                        binding.span(),
                    ));
                }
            }
            let BindingPattern::Ident(name) = binding else {
                return Err(Diagnostic::new(
                    "for-in/of destructuring binding is not supported yet".to_string(),
                    binding.span(),
                ));
            };
            if matches!(kind, BindingKind::Let | BindingKind::Const) {
                self.push_scope();
                self.declare(name.name.clone(), name.span, *kind)?;
                if let Some(e) = init {
                    self.bind_expr(e)?;
                }
                self.bind_expr(right)?;
                self.bind_stmt(body)?;
                self.pop_scope();
                Ok(())
            } else {
                // var: already hoisted into the enclosing var environment.
                if let Some(e) = init {
                    self.bind_expr(e)?;
                }
                self.bind_expr(right)?;
                self.bind_stmt(body)
            }
        } else {
            self.bind_stmt(left)?;
            self.bind_expr(right)?;
            self.bind_stmt(body)
        }
    }

    fn bind_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Ident(id) => self.bind_ident_use(id),
            Expr::Number(_)
            | Expr::BigInt(_)
            | Expr::String(_)
            | Expr::RegExp { .. }
            | Expr::Boolean { .. }
            | Expr::Null { .. }
            | Expr::This { .. }
            | Expr::Super { .. } => Ok(()),
            Expr::TemplateLiteral { expressions, .. } => {
                for e in expressions {
                    self.bind_expr(e)?;
                }
                Ok(())
            }
            Expr::TaggedTemplate {
                tag, expressions, ..
            } => {
                self.bind_expr(tag)?;
                for e in expressions {
                    self.bind_expr(e)?;
                }
                Ok(())
            }
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
            Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
                self.bind_expr(callee)?;
                for arg in args {
                    match arg {
                        Arg::Expr(expr) | Arg::Spread(expr) => self.bind_expr(expr)?,
                    }
                }
                Ok(())
            }
            Expr::FunctionExpression {
                name, params, body, ..
            } => {
                // Name (if any) is local to the function body only (ES named FE).
                self.push_scope_kind(true);
                if let Some(name) = name {
                    self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                }
                self.bind_params(params)?;
                self.install_arguments_object()?;
                self.bind_stmt(body)?;
                self.pop_scope();
                Ok(())
            }
            Expr::ArrowFunction { params, body, .. } => {
                self.push_scope_kind(true);
                self.bind_params(params)?;
                match body {
                    ArrowBody::Expr(expr) => self.bind_expr(expr)?,
                    ArrowBody::Block(stmt) => self.bind_stmt(stmt)?,
                }
                self.pop_scope();
                Ok(())
            }
            Expr::ObjectExpression { properties, .. } => {
                for prop in properties {
                    match prop {
                        ObjectProp::Property { key, value, .. } => {
                            match key {
                                ObjectKey::Ident(_) | ObjectKey::String(_) => {}
                                ObjectKey::Computed(expr) => self.bind_expr(expr)?,
                            }
                            self.bind_expr(value)?;
                        }
                        ObjectProp::Accessor {
                            key, params, body, ..
                        } => {
                            match key {
                                ObjectKey::Ident(_) | ObjectKey::String(_) => {}
                                ObjectKey::Computed(expr) => self.bind_expr(expr)?,
                            }
                            self.push_scope_kind(true);
                            self.bind_params(params)?;
                            self.install_arguments_object()?;
                            self.bind_stmt(body)?;
                            self.pop_scope();
                        }
                        ObjectProp::Spread { expr, .. } => self.bind_expr(expr)?,
                    }
                }
                Ok(())
            }
            Expr::ArrayExpression { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayElement::Expr(expr) | ArrayElement::Spread(expr) => {
                            self.bind_expr(expr)?;
                        }
                    }
                }
                Ok(())
            }
            Expr::MemberExpression {
                object,
                property,
                computed,
                ..
            } => {
                self.bind_expr(object)?;
                if *computed {
                    self.bind_expr(property)?;
                }
                // Non-computed property name is not a variable reference.
                Ok(())
            }
            Expr::Paren { expr, .. } => self.bind_expr(expr),
            Expr::As { expr, .. } => self.bind_expr(expr),
            Expr::ArrayPattern { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Pattern { binding, default } => {
                            self.bind_assign_pattern(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ArrayPatternElement::Rest(id) => {
                            self.bind_expr(&Expr::Ident(id.clone()))?;
                        }
                    }
                }
                Ok(())
            }
            Expr::ObjectPattern { properties, .. } => {
                for p in properties {
                    match p {
                        ObjectPatternProp::Prop {
                            binding, default, ..
                        } => {
                            self.bind_assign_pattern(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ObjectPatternProp::Rest(id) => {
                            self.bind_expr(&Expr::Ident(id.clone()))?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    fn bind_assign_pattern(&mut self, pat: &BindingPattern) -> Result<(), Diagnostic> {
        match pat {
            BindingPattern::Ident(id) => self.bind_expr(&Expr::Ident(id.clone())),
            BindingPattern::Array { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Pattern { binding, default } => {
                            self.bind_assign_pattern(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ArrayPatternElement::Rest(id) => {
                            self.bind_expr(&Expr::Ident(id.clone()))?;
                        }
                    }
                }
                Ok(())
            }
            BindingPattern::Object { properties, .. } => {
                for p in properties {
                    match p {
                        ObjectPatternProp::Prop {
                            binding, default, ..
                        } => {
                            self.bind_assign_pattern(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ObjectPatternProp::Rest(id) => {
                            self.bind_expr(&Expr::Ident(id.clone()))?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// Bind free references in pattern default initializers (`pat = expr`).
    fn bind_pattern_defaults(&mut self, pat: &BindingPattern) -> Result<(), Diagnostic> {
        match pat {
            BindingPattern::Ident(_) => Ok(()),
            BindingPattern::Array { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Pattern { binding, default } => {
                            self.bind_pattern_defaults(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ArrayPatternElement::Rest(_) => {}
                    }
                }
                Ok(())
            }
            BindingPattern::Object { properties, .. } => {
                for p in properties {
                    match p {
                        ObjectPatternProp::Prop {
                            binding, default, ..
                        } => {
                            self.bind_pattern_defaults(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ObjectPatternProp::Rest(_) => {}
                    }
                }
                Ok(())
            }
        }
    }

    fn bind_params(&mut self, params: &[Param]) -> Result<(), Diagnostic> {
        for p in params {
            self.declare(p.name.name.clone(), p.name.span, BindingKind::Let)?;
        }
        for p in params {
            if let Some(default) = &p.default {
                self.bind_expr(default)?;
            }
        }
        Ok(())
    }

    /// Implicit `arguments` binding for non-arrow functions (E18.24).
    /// Skipped when a param already shadows the name. Arrows inherit lexically.
    fn install_arguments_object(&mut self) -> Result<(), Diagnostic> {
        let scope = self.scopes.last().expect("scope stack non-empty");
        if scope.contains_key("arguments") {
            return Ok(());
        }
        self.declare(
            "arguments".into(),
            Span::dummy(),
            BindingKind::Var,
        )?;
        Ok(())
    }
}

struct Checker<'a> {
    bound: &'a BoundProgram,
    symbol_types: Vec<Type>,
    expr_types: HashMap<Span, Type>,
    /// Structural object shapes (`Type::Shape` indices).
    shapes: Vec<ObjectShape>,
    /// Union members (`Type::Union` indices).
    unions: Vec<UnionType>,
    /// Intersection members (`Type::Intersection` indices).
    intersections: Vec<IntersectionType>,
    /// Generic function signatures (`Type::GenericFn` indices).
    generic_fns: Vec<GenericFnSig>,
    /// Concrete type aliases in scope (name → resolved type). Program-level for T02.
    type_aliases: HashMap<String, Type>,
    /// Generic type aliases (`type Box<T> = …`).
    generic_aliases: HashMap<String, GenericAlias>,
    /// Active type parameter bindings while resolving/checking (name → Type::TypeParam).
    type_param_env: HashMap<String, Type>,
    /// Monotonic id source for open `Type::TypeParam` values.
    next_type_param_id: u32,
    /// True while typechecking an `async` function body.
    in_async: bool,
    /// True while typechecking a generator function body.
    in_generator: bool,
    /// Expected return type from an enclosing annotated function (T01).
    expected_return: Option<Type>,
}

impl<'a> Checker<'a> {
    fn new(bound: &'a BoundProgram) -> Self {
        let mut symbol_types = vec![Type::Any; bound.symbols().len()];
        for s in bound.symbols() {
            // Host globals installed with Span::dummy() (E08.05+).
            if s.span == Span::dummy() {
                symbol_types[s.id.0 as usize] = match s.name.as_str() {
                    "Math" | "Reflect" | "globalThis" | "JSON" => Type::Object,
                    "Number" | "Symbol" | "Promise" | "Proxy" | "Object" | "Function" | "Array"
                    | "String" | "Boolean" | "Error" | "TypeError" | "RangeError"
                    | "ReferenceError" | "SyntaxError" | "URIError" | "EvalError"
                    | "AggregateError" | "parseInt" | "parseFloat" | "isNaN" | "isFinite"
                    | "encodeURI" | "decodeURI" | "encodeURIComponent" | "decodeURIComponent"
                    | "Date" | "RegExp" | "Map" | "Set" | "WeakMap" | "WeakSet"
                    | "ArrayBuffer" | "DataView" | "Int8Array" | "Uint8Array"
                    | "Uint8ClampedArray" | "Int16Array" | "Uint16Array" | "Int32Array"
                    | "Uint32Array" | "Float32Array" | "Float64Array" | "BigInt64Array"
                    | "BigUint64Array" | "eval" | "escape" | "unescape" => Type::Function,
                    "NaN" | "Infinity" => Type::Number,
                    // `undefined` is its own ES language type; coarse `any` until refined.
                    "undefined" => Type::Any,
                    _ => Type::Any,
                };
            }
        }
        Self {
            bound,
            symbol_types,
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
        }
    }

    fn check_program(&mut self) -> Result<(), Diagnostic> {
        // Program-level type aliases (T02/T04): declare names, then resolve non-generic bodies.
        for stmt in &self.bound.program.body {
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
        let alias_bodies: Vec<(String, Vec<String>, TypeAnn)> = self
            .bound
            .program
            .body
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
                self.generic_aliases.insert(
                    name,
                    GenericAlias {
                        params,
                        body: ty,
                    },
                );
            }
        }
        let mut labels = Vec::new();
        for stmt in &self.bound.program.body {
            self.check_stmt(stmt, 0, 0, 0, &mut labels)?;
        }
        Ok(())
    }

    /// Left side of `for-in` / `for-of`: `let`/`const`/`var` name or assignable identifier.
    /// Annex B.3.5 allows `var name = init` only on for-in (checked by the parser).
    fn check_for_in_of_left(&mut self, left: &Stmt) -> Result<(), Diagnostic> {
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
                let BindingPattern::Ident(name) = binding else {
                    return Err(Diagnostic::new(
                        "for-in/of destructuring binding is not supported yet".to_string(),
                        binding.span(),
                    ));
                };
                let id = self
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == name.span)
                    .map(|s| s.id)
                    .expect("for-in/of binding must be declared");
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
                    | Stmt::TypeAlias { span, .. } => *span,
                },
            )),
        }
    }

    fn check_binding_pattern(
        &mut self,
        binding: &BindingPattern,
        ty: Type,
    ) -> Result<(), Diagnostic> {
        match binding {
            BindingPattern::Ident(name) => {
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
            BindingPattern::Array { elements, .. } => {
                // Element types are not refined yet; bind as Any.
                for el in elements {
                    match el {
                        ArrayPatternElement::Pattern { binding, default } => {
                            self.check_binding_pattern(binding, Type::Any)?;
                            if let Some(def) = default {
                                self.check_expr(def)?;
                            }
                        }
                        ArrayPatternElement::Rest(id) => {
                            let sym = self
                                .bound
                                .symbols()
                                .iter()
                                .find(|s| s.span == id.span)
                                .map(|s| s.id)
                                .expect("rest binding must be declared");
                            // Rest always binds an array.
                            self.symbol_types[sym.0 as usize] = Type::Any;
                        }
                    }
                }
                Ok(())
            }
            BindingPattern::Object { properties, .. } => {
                for p in properties {
                    match p {
                        ObjectPatternProp::Prop {
                            binding, default, ..
                        } => {
                            self.check_binding_pattern(binding, Type::Any)?;
                            if let Some(def) = default {
                                self.check_expr(def)?;
                            }
                        }
                        ObjectPatternProp::Rest(id) => {
                            let sym = self
                                .bound
                                .symbols()
                                .iter()
                                .find(|s| s.span == id.span)
                                .map(|s| s.id)
                                .expect("rest binding must be declared");
                            self.symbol_types[sym.0 as usize] = Type::Any;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    fn check_assign_pattern(
        &mut self,
        binding: &BindingPattern,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match binding {
            BindingPattern::Ident(id) => {
                let sym = self.bound.resolve(id.span).ok_or_else(|| {
                    Diagnostic::new(format!("unresolved identifier `{}`", id.name), id.span)
                })?;
                match self.bound.symbol(sym).kind {
                    BindingKind::Const => {
                        return Err(Diagnostic::new(
                            format!("cannot assign to const binding `{}`", id.name),
                            span,
                        ));
                    }
                    BindingKind::Function => {
                        return Err(Diagnostic::new(
                            format!("cannot assign to function binding `{}`", id.name),
                            span,
                        ));
                    }
                    BindingKind::Let | BindingKind::Var => {}
                }
                let left_ty = self.symbol_types[sym.0 as usize];
                if left_ty == Type::Any {
                    // leave Any; element values are untyped here
                }
                self.record(id.span, self.symbol_types[sym.0 as usize]);
                Ok(())
            }
            BindingPattern::Array { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Pattern { binding, default } => {
                            self.check_assign_pattern(binding, span)?;
                            if let Some(def) = default {
                                self.check_expr(def)?;
                            }
                        }
                        ArrayPatternElement::Rest(id) => {
                            self.check_assign_pattern(
                                &BindingPattern::Ident(id.clone()),
                                span,
                            )?;
                        }
                    }
                }
                Ok(())
            }
            BindingPattern::Object { properties, .. } => {
                for p in properties {
                    match p {
                        ObjectPatternProp::Prop {
                            binding, default, ..
                        } => {
                            self.check_assign_pattern(binding, span)?;
                            if let Some(def) = default {
                                self.check_expr(def)?;
                            }
                        }
                        ObjectPatternProp::Rest(id) => {
                            self.check_assign_pattern(
                                &BindingPattern::Ident(id.clone()),
                                span,
                            )?;
                        }
                    }
                }
                Ok(())
            }
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
            Stmt::TypeAlias { .. } => Ok(()),
            Stmt::Let {
                binding,
                type_ann,
                init,
                ..
            } => {
                // Bare `const` without init is rejected in the parser; for-in/of
                // left may be `const name` with no initializer.
                let ann_ty = match type_ann {
                    Some(ann) => Some(self.resolve_type_ann(ann)?),
                    None => None,
                };
                let init_ty = if let Some(init) = init {
                    self.check_expr(init)?
                } else {
                    Type::Any
                };
                let ty = if let Some(ann_ty) = ann_ty {
                    if let Some(init) = init {
                        self.require_assignable_expr(init_ty, ann_ty, init)?;
                    }
                    ann_ty
                } else {
                    init_ty
                };
                self.check_binding_pattern(binding, ty)?;
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
                name,
                type_params,
                params,
                return_type,
                body,
                is_async,
                is_generator,
                ..
            } => {
                let id = self
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == name.span)
                    .map(|s| s.id)
                    .expect("function binding must be declared");
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
                self.check_params(params)?;
                // Fresh label set inside functions (labels do not cross function boundaries).
                let mut inner_labels = Vec::new();
                let prev_async = self.in_async;
                let prev_generator = self.in_generator;
                let prev_ret = self.expected_return;
                self.in_async = *is_async;
                self.in_generator = *is_generator;
                self.expected_return = match return_type {
                    Some(ann) => Some(self.resolve_type_ann(ann)?),
                    None => None,
                };
                let result = self.check_stmt(body, 0, 0, fn_depth + 1, &mut inner_labels);
                self.in_async = prev_async;
                self.in_generator = prev_generator;
                self.expected_return = prev_ret;
                self.type_param_env = saved_env;
                result
            }
            Stmt::ClassDeclaration {
                name,
                super_class,
                body,
                ..
            } => {
                let id = self
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == name.span)
                    .map(|s| s.id)
                    .expect("class binding must be declared");
                self.symbol_types[id.0 as usize] = Type::Function;
                if let Some(sc) = super_class {
                    self.check_expr(sc)?;
                }
                for el in body {
                    match el {
                        ClassElement::Constructor { params, body, .. } => {
                            self.check_params(params)?;
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = false;
                            self.in_generator = false;
                            self.check_stmt(body, 0, 0, fn_depth + 1, &mut inner_labels)?;
                            self.in_async = prev_async;
                            self.in_generator = prev_generator;
                        }
                        ClassElement::Method {
                            params,
                            body,
                            is_generator,
                            ..
                        } => {
                            self.check_params(params)?;
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = false;
                            self.in_generator = *is_generator;
                            self.check_stmt(body, 0, 0, fn_depth + 1, &mut inner_labels)?;
                            self.in_async = prev_async;
                            self.in_generator = prev_generator;
                        }
                        ClassElement::Accessor {
                            params, body, ..
                        } => {
                            self.check_params(params)?;
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = false;
                            self.in_generator = false;
                            self.check_stmt(body, 0, 0, fn_depth + 1, &mut inner_labels)?;
                            self.in_async = prev_async;
                            self.in_generator = prev_generator;
                        }
                    }
                }
                Ok(())
            }
            Stmt::Return { argument, span } => {
                if fn_depth == 0 {
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
                        let id = self
                            .bound
                            .symbols()
                            .iter()
                            .find(|s| s.span == param.span)
                            .map(|s| s.id)
                            .expect("catch binding must be declared");
                        self.symbol_types[id.0 as usize] = Type::Any;
                    }
                    self.check_stmt(handler, loop_depth, switch_depth, fn_depth, labels)?;
                }
                if let Some(finalizer) = finalizer {
                    self.check_stmt(finalizer, loop_depth, switch_depth, fn_depth, labels)?;
                }
                Ok(())
            }
            Stmt::With { object, body, .. } => {
                self.check_expr(object)?;
                self.check_stmt(body, loop_depth, switch_depth, fn_depth, labels)
            }
            Stmt::ImportDeclaration { span, .. }
            | Stmt::ExportNamedDeclaration { span, .. }
            | Stmt::ExportDefaultDeclaration { span, .. } => Err(Diagnostic::new(
                "import/export must be linked before bind/check".to_string(),
                *span,
            )),
        }
    }

    fn check_expr(&mut self, expr: &Expr) -> Result<Type, Diagnostic> {
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
                expressions,
                span,
                ..
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
            Expr::Ident(id) => {
                if let Some(sym) = self.bound.resolve(id.span) {
                    let ty = self.symbol_types[sym.0 as usize];
                    self.record(id.span, ty);
                    ty
                } else {
                    // Free / with-chain name (Object Environment).
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
                let to = self.resolve_type_ann(ann)?;
                if !self.is_assignable(from, to) && !Self::is_dual_world_boundary(from, to) {
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
            Expr::Unary { op, arg, span } => {
                if *op == UnaryOp::Await && !self.in_async {
                    return Err(Diagnostic::new(
                        "await is only valid in async functions".to_string(),
                        *span,
                    ));
                }
                if matches!(op, UnaryOp::Yield | UnaryOp::YieldStar) && !self.in_generator {
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
                let value_ty = self.check_expr(value)?;
                match target.as_ref() {
                    Expr::Ident(id) => {
                        let Some(sym) = self.bound.resolve(id.span) else {
                            // Free / with-chain assign target.
                            self.record(id.span, Type::Any);
                            self.record(*span, value_ty);
                            return Ok(value_ty);
                        };
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
                            BindingKind::Let | BindingKind::Var => {}
                        }
                        let left_ty = self.symbol_types[sym.0 as usize];
                        let result_ty = if let Some(bin_op) = op.binary_op() {
                            self.check_binary(bin_op, left_ty, value_ty, *span, target, value)?
                        } else {
                            value_ty
                        };
                        if left_ty == Type::Any {
                            self.symbol_types[sym.0 as usize] = result_ty;
                        } else if op.binary_op().is_some() {
                            if !self.is_assignable(result_ty, left_ty) {
                                return Err(Diagnostic::new(
                                    format!(
                                        "cannot assign type `{result_ty}` to binding of type `{left_ty}`"
                                    ),
                                    *span,
                                ));
                            }
                        } else {
                            self.require_assignable_expr(result_ty, left_ty, value)?;
                        }
                        self.record(id.span, self.symbol_types[sym.0 as usize]);
                        self.record(*span, result_ty);
                        result_ty
                    }
                    Expr::MemberExpression {
                        object,
                        property,
                        computed,
                        ..
                    } => {
                        // Property write: object + key are checked; result is the assigned value.
                        // Compound assignment on members is out of scope for E04.02 simple `=`.
                        if op.binary_op().is_some() {
                            return Err(Diagnostic::new(
                                "compound assignment to property not yet supported".to_string(),
                                *span,
                            ));
                        }
                        self.check_expr(object)?;
                        if *computed {
                            self.check_expr(property)?;
                        }
                        self.record(*span, value_ty);
                        value_ty
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
                                ArrayPatternElement::Pattern { binding, default } => {
                                    self.check_assign_pattern(binding, *span)?;
                                    if let Some(def) = default {
                                        self.check_expr(def)?;
                                    }
                                }
                                ArrayPatternElement::Rest(id) => {
                                    self.check_assign_pattern(
                                        &BindingPattern::Ident(id.clone()),
                                        *span,
                                    )?;
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
                                    binding, default, ..
                                } => {
                                    self.check_assign_pattern(binding, *span)?;
                                    if let Some(def) = default {
                                        self.check_expr(def)?;
                                    }
                                }
                                ObjectPatternProp::Rest(id) => {
                                    self.check_assign_pattern(
                                        &BindingPattern::Ident(id.clone()),
                                        *span,
                                    )?;
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
                match arg.as_ref() {
                    Expr::Ident(id) => {
                        let Some(sym) = self.bound.resolve(id.span) else {
                            self.record(id.span, Type::Any);
                            self.record(*span, Type::Number);
                            return Ok(Type::Number);
                        };
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
                            BindingKind::Let | BindingKind::Var => {}
                        }
                        let left_ty = self.symbol_types[sym.0 as usize];
                        let ok = left_ty == Type::Number
                            || left_ty == Type::Any
                            || matches!(left_ty, Type::Native(n) if !n.is_float());
                        if !ok {
                            return Err(Diagnostic::new(
                                format!("update operator cannot be applied to type `{left_ty}`"),
                                *span,
                            ));
                        }
                        if left_ty == Type::Any {
                            self.symbol_types[sym.0 as usize] = Type::Number;
                        }
                        let out = if matches!(left_ty, Type::Native(_)) {
                            left_ty
                        } else {
                            Type::Number
                        };
                        self.record(id.span, out);
                        self.record(*span, out);
                        return Ok(out);
                    }
                    _ => {
                        return Err(Diagnostic::new(
                            "invalid update target".to_string(),
                            *span,
                        ));
                    }
                }
            }
            Expr::Call {
                callee,
                args,
                span,
                ..
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
                let result_ty = match callee_ty {
                    Type::Any | Type::Function => Type::Any,
                    Type::GenericFn(gid) => self.instantiate_generic_call(gid, &arg_tys, *span)?,
                    _ => {
                        return Err(Diagnostic::new(
                            format!("type `{callee_ty}` is not callable"),
                            *span,
                        ));
                    }
                };
                self.record(*span, result_ty);
                result_ty
            }
            Expr::New {
                callee,
                args,
                span,
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
                if callee_ty != Type::Any
                    && callee_ty != Type::Function
                    && !matches!(callee_ty, Type::GenericFn(_))
                {
                    return Err(Diagnostic::new(
                        format!("type `{callee_ty}` is not constructable"),
                        *span,
                    ));
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
                span,
            } => {
                if let Some(name) = name {
                    let id = self
                        .bound
                        .symbols()
                        .iter()
                        .find(|s| s.span == name.span)
                        .map(|s| s.id)
                        .expect("function expression name must be declared");
                    self.symbol_types[id.0 as usize] = Type::Function;
                }
                self.check_params(params)?;
                // New function boundary (return allowed; labels do not escape).
                let mut inner_labels = Vec::new();
                let prev_async = self.in_async;
                let prev_generator = self.in_generator;
                let prev_ret = self.expected_return;
                self.in_async = *is_async;
                self.in_generator = *is_generator;
                self.expected_return = match return_type {
                    Some(ann) => Some(self.resolve_type_ann(ann)?),
                    None => None,
                };
                self.check_stmt(body, 0, 0, 1, &mut inner_labels)?;
                self.in_async = prev_async;
                self.in_generator = prev_generator;
                self.expected_return = prev_ret;
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
                self.check_params(params)?;
                let mut inner_labels = Vec::new();
                let prev_async = self.in_async;
                let prev_generator = self.in_generator;
                let prev_ret = self.expected_return;
                self.in_async = *is_async;
                self.in_generator = false;
                let ret_ty = match return_type {
                    Some(ann) => Some(self.resolve_type_ann(ann)?),
                    None => None,
                };
                self.expected_return = ret_ty;
                match body {
                    ArrowBody::Expr(expr) => {
                        let body_ty = self.check_expr(expr)?;
                        if let Some(expected) = ret_ty {
                            self.require_assignable_expr(body_ty, expected, expr)?;
                        }
                    }
                    ArrowBody::Block(stmt) => {
                        self.check_stmt(stmt, 0, 0, 1, &mut inner_labels)?;
                    }
                }
                self.in_async = prev_async;
                self.in_generator = prev_generator;
                self.expected_return = prev_ret;
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
                            key, params, body, ..
                        } => {
                            if let ObjectKey::Computed(expr) = key {
                                self.check_expr(expr)?;
                            }
                            self.check_params(params)?;
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = false;
                            self.in_generator = false;
                            self.check_stmt(body, 0, 0, 1, &mut inner_labels)?;
                            self.in_async = prev_async;
                            self.in_generator = prev_generator;
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
                    self.intern_shape(shape_props)
                } else {
                    Type::Object
                };
                self.record(*span, ty);
                ty
            }
            Expr::ArrayExpression { elements, span } => {
                for el in elements {
                    match el {
                        ArrayElement::Expr(expr) | ArrayElement::Spread(expr) => {
                            self.check_expr(expr)?;
                        }
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
                    Type::Any
                } else if let Expr::Ident(id) = property.as_ref() {
                    self.prop_type(obj_ty, &id.name).unwrap_or(Type::Any)
                } else {
                    Type::Any
                };
                self.record(*span, ty);
                ty
            }
        };
        Ok(ty)
    }

    fn check_params(&mut self, params: &[Param]) -> Result<(), Diagnostic> {
        for (i, p) in params.iter().enumerate() {
            if p.rest {
                if i != params.len() - 1 {
                    return Err(Diagnostic::new(
                        "rest parameter must be last formal parameter".to_string(),
                        p.name.span,
                    ));
                }
                if p.default.is_some() {
                    return Err(Diagnostic::new(
                        "rest parameter cannot have a default".to_string(),
                        p.name.span,
                    ));
                }
            }
            let pid = self
                .bound
                .symbols()
                .iter()
                .find(|s| s.span == p.name.span)
                .map(|s| s.id)
                .expect("param binding must be declared");
            let ann_ty = match &p.type_ann {
                Some(ann) => Some(self.resolve_type_ann(ann)?),
                None => None,
            };
            if let Some(default) = &p.default {
                let def_ty = self.check_expr(default)?;
                if let Some(ann_ty) = ann_ty {
                    self.require_assignable_expr(def_ty, ann_ty, default)?;
                }
            }
            self.symbol_types[pid.0 as usize] = ann_ty.unwrap_or(Type::Any);
        }
        Ok(())
    }

    fn intern_shape(&mut self, props: Vec<(String, Type)>) -> Type {
        let id = self.shapes.len() as u32;
        self.shapes.push(ObjectShape { props });
        Type::Shape(id)
    }

    fn intern_union(&mut self, mut members: Vec<Type>) -> Type {
        let mut flat = Vec::new();
        for m in members.drain(..) {
            self.collect_union_members(m, &mut flat);
        }
        // Dedup while preserving order.
        let mut out = Vec::new();
        for m in flat {
            if !out.contains(&m) {
                out.push(m);
            }
        }
        if out.is_empty() {
            return Type::Any;
        }
        if out.len() == 1 {
            return out[0];
        }
        let id = self.unions.len() as u32;
        self.unions.push(UnionType { members: out });
        Type::Union(id)
    }

    fn intern_intersection(&mut self, mut members: Vec<Type>) -> Type {
        let mut flat = Vec::new();
        for m in members.drain(..) {
            self.collect_intersection_members(m, &mut flat);
        }
        let mut out = Vec::new();
        for m in flat {
            if !out.contains(&m) {
                out.push(m);
            }
        }
        // Merge all object shapes into one structural type when possible.
        let mut shape_props: Option<Vec<(String, Type)>> = None;
        let mut rest = Vec::new();
        for m in out {
            match m {
                Type::Shape(id) => {
                    if let Some(shape) = self.shapes.get(id as usize) {
                        let acc = shape_props.get_or_insert_with(Vec::new);
                        for (n, t) in &shape.props {
                            if let Some((_, existing)) = acc.iter_mut().find(|(en, _)| en == n) {
                                *existing = *t;
                            } else {
                                acc.push((n.clone(), *t));
                            }
                        }
                    }
                }
                other => rest.push(other),
            }
        }
        if let Some(props) = shape_props {
            rest.push(self.intern_shape(props));
        }
        if rest.is_empty() {
            return Type::Any;
        }
        if rest.len() == 1 {
            return rest[0];
        }
        let id = self.intersections.len() as u32;
        self.intersections
            .push(IntersectionType { members: rest });
        Type::Intersection(id)
    }

    fn collect_union_members(&self, ty: Type, out: &mut Vec<Type>) {
        match ty {
            Type::Union(id) => {
                if let Some(u) = self.unions.get(id as usize) {
                    for m in &u.members {
                        self.collect_union_members(*m, out);
                    }
                }
            }
            other => out.push(other),
        }
    }

    fn collect_intersection_members(&self, ty: Type, out: &mut Vec<Type>) {
        match ty {
            Type::Intersection(id) => {
                if let Some(i) = self.intersections.get(id as usize) {
                    for m in &i.members {
                        self.collect_intersection_members(*m, out);
                    }
                }
            }
            other => out.push(other),
        }
    }

    fn union_members(&self, ty: Type) -> Vec<Type> {
        let mut out = Vec::new();
        self.collect_union_members(ty, &mut out);
        out
    }

    fn prop_type(&self, obj: Type, name: &str) -> Option<Type> {
        match obj {
            Type::Shape(id) => self
                .shapes
                .get(id as usize)
                .and_then(|s| s.props.iter().find(|(n, _)| n == name).map(|(_, t)| *t)),
            Type::Intersection(id) => {
                let i = self.intersections.get(id as usize)?;
                for m in &i.members {
                    if let Some(t) = self.prop_type(*m, name) {
                        return Some(t);
                    }
                }
                None
            }
            _ => None,
        }
    }

    /// Resolve a type annotation to a Checker `Type` (T01–T04).
    fn resolve_type_ann(&mut self, ann: &TypeAnn) -> Result<Type, Diagnostic> {
        match ann {
            TypeAnn::Named { name, span } => {
                if let Some(tp) = self.type_param_env.get(name).copied() {
                    return Ok(tp);
                }
                let ty = match name.as_str() {
                    "number" => Type::Number,
                    "string" => Type::String,
                    "boolean" => Type::Boolean,
                    "bigint" => Type::BigInt,
                    "any" => Type::Any,
                    "null" => Type::Null,
                    "object" => Type::Object,
                    "function" => Type::Function,
                    other => {
                        if let Some(n) = NativeType::from_name(other) {
                            Type::Native(n)
                        } else if let Some(aliased) = self.type_aliases.get(other).copied() {
                            aliased
                        } else if self.generic_aliases.contains_key(other) {
                            return Err(Diagnostic::new(
                                format!(
                                    "generic type `{other}` requires type arguments"
                                ),
                                *span,
                            ));
                        } else {
                            return Err(Diagnostic::new(
                                format!("unknown type name `{other}`"),
                                *span,
                            ));
                        }
                    }
                };
                Ok(ty)
            }
            TypeAnn::GenericApp { name, args, span } => {
                let Some(alias) = self.generic_aliases.get(name).cloned() else {
                    if self.type_aliases.contains_key(name) {
                        return Err(Diagnostic::new(
                            format!("type `{name}` is not generic"),
                            *span,
                        ));
                    }
                    return Err(Diagnostic::new(
                        format!("unknown type name `{name}`"),
                        *span,
                    ));
                };
                if args.len() != alias.params.len() {
                    return Err(Diagnostic::new(
                        format!(
                            "generic type `{name}` expects {} type argument(s), got {}",
                            alias.params.len(),
                            args.len()
                        ),
                        *span,
                    ));
                }
                let mut arg_tys = Vec::with_capacity(args.len());
                for a in args {
                    arg_tys.push(self.resolve_type_ann(a)?);
                }
                let saved = self.type_param_env.clone();
                for (p, t) in alias.params.iter().zip(arg_tys.iter()) {
                    self.type_param_env.insert(p.clone(), *t);
                }
                let resolved = self.resolve_type_ann(&alias.body);
                self.type_param_env = saved;
                resolved
            }
            TypeAnn::Object { props, .. } => {
                let mut shape_props = Vec::with_capacity(props.len());
                for p in props {
                    let ty = self.resolve_type_ann(&p.ty)?;
                    shape_props.push((p.name.clone(), ty));
                }
                Ok(self.intern_shape(shape_props))
            }
            TypeAnn::Union { types, .. } => {
                let mut members = Vec::with_capacity(types.len());
                for t in types {
                    members.push(self.resolve_type_ann(t)?);
                }
                Ok(self.intern_union(members))
            }
            TypeAnn::Intersection { types, .. } => {
                let mut members = Vec::with_capacity(types.len());
                for t in types {
                    members.push(self.resolve_type_ann(t)?);
                }
                Ok(self.intern_intersection(members))
            }
        }
    }

    /// Instantiate a generic function at a call site via argument-driven inference (T04).
    fn instantiate_generic_call(
        &mut self,
        gid: u32,
        arg_tys: &[Type],
        span: Span,
    ) -> Result<Type, Diagnostic> {
        let sig = self
            .generic_fns
            .get(gid as usize)
            .cloned()
            .expect("generic fn id");
        // Open type params as unique placeholders, then unify from annotated params.
        let mut subst: HashMap<u32, Type> = HashMap::new();
        let mut open_ids: HashMap<String, u32> = HashMap::new();
        let saved = self.type_param_env.clone();
        for p in &sig.type_params {
            let id = self.next_type_param_id;
            self.next_type_param_id += 1;
            open_ids.insert(p.clone(), id);
            self.type_param_env
                .insert(p.clone(), Type::TypeParam(id));
        }
        // Resolve param annotations under open env and unify with arg types.
        for (i, pann) in sig.param_types.iter().enumerate() {
            let Some(ann) = pann else { continue };
            let expected = self.resolve_type_ann(ann)?;
            let got = arg_tys.get(i).copied().unwrap_or(Type::Any);
            self.unify_infer(expected, got, &mut subst, span)?;
        }
        let ret = match &sig.return_type {
            Some(ann) => {
                let open_ret = self.resolve_type_ann(ann)?;
                Ok(self.apply_subst(open_ret, &subst))
            }
            None => Ok(Type::Any),
        };
        self.type_param_env = saved;
        // Check args assignable to substituted param types.
        let saved = self.type_param_env.clone();
        for p in &sig.type_params {
            if let Some(&id) = open_ids.get(p) {
                let concrete = subst.get(&id).copied().unwrap_or(Type::Any);
                self.type_param_env.insert(p.clone(), concrete);
            }
        }
        for (i, pann) in sig.param_types.iter().enumerate() {
            if let Some(ann) = pann {
                let expected = self.resolve_type_ann(ann)?;
                let got = arg_tys.get(i).copied().unwrap_or(Type::Any);
                if let Err(e) = self.require_assignable(got, expected, span) {
                    self.type_param_env = saved;
                    return Err(e);
                }
            }
        }
        let result = ret?;
        // Re-resolve return under concrete subst for nested generics in return ann.
        let result = if let Some(ann) = &sig.return_type {
            self.resolve_type_ann(ann)?
        } else {
            result
        };
        self.type_param_env = saved;
        Ok(result)
    }

    /// Unify `pattern` (may contain open TypeParams) with `concrete`, recording subst.
    fn unify_infer(
        &self,
        pattern: Type,
        concrete: Type,
        subst: &mut HashMap<u32, Type>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match pattern {
            Type::TypeParam(id) => {
                if let Some(existing) = subst.get(&id).copied() {
                    if existing != concrete
                        && existing != Type::Any
                        && concrete != Type::Any
                        && !self.is_assignable(concrete, existing)
                        && !self.is_assignable(existing, concrete)
                    {
                        return Err(Diagnostic::new(
                            format!(
                                "type parameter inferred as both `{}` and `{}`",
                                format_type_full(
                                    existing,
                                    &self.shapes,
                                    &self.unions,
                                    &self.intersections
                                ),
                                format_type_full(
                                    concrete,
                                    &self.shapes,
                                    &self.unions,
                                    &self.intersections
                                )
                            ),
                            span,
                        ));
                    }
                    if existing == Type::Any && concrete != Type::Any {
                        subst.insert(id, concrete);
                    }
                } else {
                    subst.insert(id, concrete);
                }
                Ok(())
            }
            Type::Shape(pid) => {
                if let Type::Shape(cid) = concrete {
                    let Some(ps) = self.shapes.get(pid as usize) else {
                        return Ok(());
                    };
                    let Some(cs) = self.shapes.get(cid as usize) else {
                        return Ok(());
                    };
                    for (name, pt) in &ps.props {
                        if let Some((_, ct)) = cs.props.iter().find(|(n, _)| n == name) {
                            self.unify_infer(*pt, *ct, subst, span)?;
                        }
                    }
                }
                Ok(())
            }
            Type::Union(id) => {
                // Infer against each member; take first successful path that binds.
                if let Some(u) = self.unions.get(id as usize) {
                    for m in &u.members {
                        let mut trial = subst.clone();
                        if self.unify_infer(*m, concrete, &mut trial, span).is_ok() {
                            *subst = trial;
                            return Ok(());
                        }
                    }
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn apply_subst(&self, ty: Type, subst: &HashMap<u32, Type>) -> Type {
        match ty {
            Type::TypeParam(id) => subst.get(&id).copied().unwrap_or(ty),
            other => other,
        }
    }

    /// Number (or ±number) literal expression — may contextually type as a native numeric.
    fn is_number_literal_expr(expr: &Expr) -> bool {
        match expr {
            Expr::Number(_) => true,
            Expr::Unary {
                op: UnaryOp::Plus | UnaryOp::Minus,
                arg,
                ..
            } => matches!(arg.as_ref(), Expr::Number(_)),
            Expr::Paren { expr, .. } | Expr::As { expr, .. } => {
                Self::is_number_literal_expr(expr)
            }
            _ => false,
        }
    }

    fn number_literal_ok_for_native(to: Type) -> bool {
        matches!(to, Type::Native(_))
    }

    /// Explicit dual-worlds boundary (`as`): JS `number` ↔ unboxed native (T06).
    fn is_dual_world_boundary(from: Type, to: Type) -> bool {
        matches!(
            (from, to),
            (Type::Number, Type::Native(_)) | (Type::Native(_), Type::Number)
        )
    }

    /// Assignability with contextual typing of numeric literals to native types (T05).
    fn require_assignable_expr(
        &self,
        from: Type,
        to: Type,
        from_expr: &Expr,
    ) -> Result<(), Diagnostic> {
        if self.is_assignable(from, to) {
            return Ok(());
        }
        if Self::is_number_literal_expr(from_expr) && Self::number_literal_ok_for_native(to) {
            return Ok(());
        }
        self.require_assignable(from, to, expr_span_of(from_expr))
    }

    /// Whether `from` is assignable to `to` (exact, `any`, structural, union/intersection).
    fn is_assignable(&self, from: Type, to: Type) -> bool {
        if from == to || from == Type::Any || to == Type::Any {
            return true;
        }
        // Source union: every member must be assignable to the target.
        if let Type::Union(id) = from {
            if let Some(u) = self.unions.get(id as usize) {
                return u.members.iter().all(|m| self.is_assignable(*m, to));
            }
            return false;
        }
        // Target union: source must be assignable to some member.
        if let Type::Union(id) = to {
            if let Some(u) = self.unions.get(id as usize) {
                return u.members.iter().any(|m| self.is_assignable(from, *m));
            }
            return false;
        }
        // Target intersection: source must satisfy every member.
        if let Type::Intersection(id) = to {
            if let Some(i) = self.intersections.get(id as usize) {
                return i.members.iter().all(|m| self.is_assignable(from, *m));
            }
            return false;
        }
        // Source intersection: assignable if any member is (or the merged whole matches).
        if let Type::Intersection(id) = from {
            if let Some(i) = self.intersections.get(id as usize) {
                if i.members.iter().any(|m| self.is_assignable(*m, to)) {
                    return true;
                }
            }
        }
        // Structural: source must supply every property required by the target.
        if let Type::Shape(to_id) = to {
            let Some(to_shape) = self.shapes.get(to_id as usize) else {
                return false;
            };
            match from {
                Type::Shape(from_id) => {
                    let Some(from_shape) = self.shapes.get(from_id as usize) else {
                        return false;
                    };
                    to_shape.props.iter().all(|(name, want)| {
                        from_shape
                            .props
                            .iter()
                            .find(|(n, _)| n == name)
                            .is_some_and(|(_, got)| self.is_assignable(*got, *want))
                    })
                }
                // Unshaped object is not known to have the required props.
                Type::Object => false,
                _ => false,
            }
        } else {
            false
        }
    }

    fn require_assignable(
        &self,
        from: Type,
        to: Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
        if self.is_assignable(from, to) {
            Ok(())
        } else {
            let from_s =
                format_type_full(from, &self.shapes, &self.unions, &self.intersections);
            let to_s = format_type_full(to, &self.shapes, &self.unions, &self.intersections);
            Err(Diagnostic::new(
                format!("type `{from_s}` is not assignable to type `{to_s}`"),
                span,
            ))
        }
    }

    /// Map a `typeof` string tag to a checker type.
    fn typeof_tag_type(tag: &str) -> Option<Type> {
        match tag {
            "string" => Some(Type::String),
            "number" => Some(Type::Number),
            "boolean" => Some(Type::Boolean),
            "bigint" => Some(Type::BigInt),
            "function" => Some(Type::Function),
            "object" => Some(Type::Object),
            _ => None,
        }
    }

    /// Whether `ty` is consistent with a `typeof` tag (for filtering unions).
    fn matches_typeof_tag(&self, ty: Type, tag: &str) -> bool {
        match tag {
            "string" => ty == Type::String || ty == Type::Any,
            "number" => ty == Type::Number || ty == Type::Any,
            "boolean" => ty == Type::Boolean || ty == Type::Any,
            "bigint" => ty == Type::BigInt || ty == Type::Any,
            "function" => ty == Type::Function || ty == Type::Any,
            "object" => {
                matches!(
                    ty,
                    Type::Object | Type::Shape(_) | Type::Null | Type::Any
                )
            }
            _ => false,
        }
    }

    /// Filter `ty` to members that match / don't match a typeof tag.
    fn filter_by_typeof(&mut self, ty: Type, tag: &str, positive: bool) -> Type {
        let members = self.union_members(ty);
        let filtered: Vec<Type> = members
            .into_iter()
            .filter(|m| {
                let matches = self.matches_typeof_tag(*m, tag);
                if positive {
                    matches
                } else {
                    !matches || *m == Type::Any
                }
            })
            .collect();
        if filtered.is_empty() {
            // No remaining members: use the tag type (positive) or keep original.
            if positive {
                Self::typeof_tag_type(tag).unwrap_or(ty)
            } else {
                ty
            }
        } else if filtered.len() == 1 {
            filtered[0]
        } else {
            self.intern_union(filtered)
        }
    }

    /// Detect `typeof id === "tag"` / `!==` and produce then/else narrow maps.
    fn typeof_narrow_facts(
        &mut self,
        test: &Expr,
    ) -> (Vec<(SymbolId, Type)>, Vec<(SymbolId, Type)>) {
        let empty = (Vec::new(), Vec::new());
        let Expr::Binary {
            left, op, right, ..
        } = test
        else {
            return empty;
        };
        let positive = match op {
            BinaryOp::EqEqEq | BinaryOp::EqEq => true,
            BinaryOp::NotEqEq | BinaryOp::NotEq => false,
            _ => return empty,
        };
        // typeof x === "string"  OR  "string" === typeof x
        let (ident, tag) = if let (
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            },
            Expr::String(s),
        ) = (left.as_ref(), right.as_ref())
        {
            let Expr::Ident(id) = arg.as_ref() else {
                return empty;
            };
            (id, s.value.to_string_lossy())
        } else if let (
            Expr::String(s),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            },
        ) = (left.as_ref(), right.as_ref())
        {
            let Expr::Ident(id) = arg.as_ref() else {
                return empty;
            };
            (id, s.value.to_string_lossy())
        } else {
            return empty;
        };
        let Some(sym) = self.bound.resolve(ident.span) else {
            return empty;
        };
        let cur = self.symbol_types[sym.0 as usize];
        let then_ty = self.filter_by_typeof(cur, tag.as_str(), positive);
        let else_ty = self.filter_by_typeof(cur, tag.as_str(), !positive);
        (vec![(sym, then_ty)], vec![(sym, else_ty)])
    }

    fn with_narrows<R>(
        &mut self,
        narrows: &[(SymbolId, Type)],
        f: impl FnOnce(&mut Self) -> Result<R, Diagnostic>,
    ) -> Result<R, Diagnostic> {
        let mut saved = Vec::with_capacity(narrows.len());
        for (id, ty) in narrows {
            let idx = id.0 as usize;
            saved.push((*id, self.symbol_types[idx]));
            self.symbol_types[idx] = *ty;
        }
        let result = f(self);
        for (id, ty) in saved {
            self.symbol_types[id.0 as usize] = ty;
        }
        result
    }

    fn record(&mut self, span: Span, ty: Type) {
        self.expr_types.insert(span, ty);
    }

    /// True when `expr` is an identifier resolving to the host global `name`.
    fn is_global_ident(&self, expr: &Expr, name: &str) -> bool {
        let Expr::Ident(id) = expr else {
            return false;
        };
        let Some(sym_id) = self.bound.resolve(id.span) else {
            return false;
        };
        let sym = self.bound.symbol(sym_id);
        sym.name == name && sym.span == Span::dummy()
    }

    fn check_unary(&self, op: UnaryOp, arg: Type, span: Span) -> Result<Type, Diagnostic> {
        match op {
            // Unary `+` is ToNumber (ECMA-262); BigInt throws at runtime — reject statically.
            UnaryOp::Plus => {
                if arg == Type::BigInt {
                    Err(Diagnostic::new(
                        format!("unary `{op}` cannot be applied to type `{arg}`"),
                        span,
                    ))
                } else {
                    Ok(Type::Number)
                }
            }
            UnaryOp::Minus | UnaryOp::BitNot => {
                if arg == Type::Number || arg == Type::Any {
                    Ok(Type::Number)
                } else if arg == Type::BigInt {
                    Ok(Type::BigInt)
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
            // Await yields the fulfillment value; keep coarse `any` for now.
            UnaryOp::Await => Ok(Type::Any),
            // Yield expression value is the next `.next(arg)` resume value; coarse `any`.
            // `yield*` completion is the inner iterator's final value; coarse `any`.
            UnaryOp::Yield | UnaryOp::YieldStar => Ok(Type::Any),
        }
    }

    fn check_binary(
        &self,
        op: BinaryOp,
        left: Type,
        right: Type,
        span: Span,
        left_expr: &Expr,
        right_expr: &Expr,
    ) -> Result<Type, Diagnostic> {
        match op {
            // Binary `+`: string preference (ToString) else numeric (ToNumber), per ECMA-262.
            // Object/Function sides use runtime ToPrimitive (valueOf/toString); static type is Any.
            BinaryOp::Add => {
                if left == Type::BigInt || right == Type::BigInt {
                    if left == Type::BigInt && right == Type::BigInt {
                        Ok(Type::BigInt)
                    } else {
                        Err(Diagnostic::new(
                            format!(
                                "operator `+` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    }
                } else if left == Type::String || right == Type::String {
                    Ok(Type::String)
                } else if let Some(n) =
                    self.native_arith_result(left, right, left_expr, right_expr)
                {
                    Ok(Type::Native(n))
                } else if matches!(
                    left,
                    Type::Object | Type::Shape(_) | Type::Function | Type::Any
                ) || matches!(
                    right,
                    Type::Object | Type::Shape(_) | Type::Function | Type::Any
                ) {
                    if self.is_add_operand(left) && self.is_add_operand(right) {
                        Ok(Type::Any)
                    } else {
                        Err(Diagnostic::new(
                            format!(
                                "operator `+` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    }
                } else if self.is_primitive_numeric_coercible(left)
                    && self.is_primitive_numeric_coercible(right)
                {
                    Ok(Type::Number)
                } else {
                    Err(Diagnostic::new(
                        format!("operator `+` cannot be applied to types `{left}` and `{right}`"),
                        span,
                    ))
                }
            }
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
                if left == Type::BigInt && right == Type::BigInt {
                    // `>>>` is not defined on BigInt in ECMA-262.
                    if matches!(op, BinaryOp::UShr) {
                        Err(Diagnostic::new(
                            format!(
                                "operator `{op}` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    } else {
                        Ok(Type::BigInt)
                    }
                } else if let Some(n) =
                    self.native_arith_result(left, right, left_expr, right_expr)
                {
                    // `>>>` is JS ToUint32; reject on native types.
                    if matches!(op, BinaryOp::UShr) {
                        Err(Diagnostic::new(
                            format!(
                                "operator `{op}` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    } else {
                        Ok(Type::Native(n))
                    }
                } else if self.is_numberish(left) && self.is_numberish(right) {
                    Ok(Type::Number)
                } else {
                    Err(Diagnostic::new(
                        format!("operator `{op}` cannot be applied to types `{left}` and `{right}`"),
                        span,
                    ))
                }
            }
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                if (self.is_numberish(left) && self.is_numberish(right))
                    || (left == Type::BigInt && right == Type::BigInt)
                    || self
                        .native_arith_result(left, right, left_expr, right_expr)
                        .is_some()
                {
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
            // `in`: property-key left, object-like right; result is boolean (ECMA-262).
            BinaryOp::In => Ok(Type::Boolean),
            // `instanceof`: object left, constructor right; result is boolean (ECMA-262).
            BinaryOp::InstanceOf => Ok(Type::Boolean),
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

    /// Same native type on both sides, or native + number-literal (contextual).
    fn native_arith_result(
        &self,
        left: Type,
        right: Type,
        left_expr: &Expr,
        right_expr: &Expr,
    ) -> Option<NativeType> {
        match (left, right) {
            (Type::Native(a), Type::Native(b)) if a == b => Some(a),
            (Type::Native(a), Type::Number) if Self::is_number_literal_expr(right_expr) => Some(a),
            (Type::Number, Type::Native(b)) if Self::is_number_literal_expr(left_expr) => Some(b),
            _ => None,
        }
    }

    /// Primitives ToNumber accepts for binary `+` when neither side is string/BigInt/object.
    fn is_primitive_numeric_coercible(&self, ty: Type) -> bool {
        matches!(ty, Type::Number | Type::Boolean | Type::Null)
    }

    fn is_add_operand(&self, ty: Type) -> bool {
        !matches!(ty, Type::BigInt | Type::Native(_))
    }
}

fn expr_span_of(expr: &Expr) -> Span {
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
        | Expr::ArrowFunction { span, .. }
        | Expr::ObjectExpression { span, .. }
        | Expr::ArrayExpression { span, .. }
        | Expr::ArrayPattern { span, .. }
        | Expr::ObjectPattern { span, .. }
        | Expr::MemberExpression { span, .. }
        | Expr::Paren { span, .. }
        | Expr::As { span, .. } => *span,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_parser::parse;

    fn user_symbol<'a>(bound: &'a BoundProgram, name: &str) -> &'a Symbol {
        bound
            .symbols()
            .iter()
            .find(|s| s.name == name && s.span != Span::dummy())
            .unwrap_or_else(|| panic!("no user symbol `{name}`"))
    }

    #[test]
    fn bind_let_declares_symbol() {
        let program = parse("let x = 1;").unwrap();
        let bound = bind(program).unwrap();
        let x = user_symbol(&bound, "x");
        assert_eq!(x.kind, BindingKind::Let);
    }

    #[test]
    fn bind_const_declares_symbol() {
        let program = parse("const x = 1;").unwrap();
        let bound = bind(program).unwrap();
        let x = user_symbol(&bound, "x");
        assert_eq!(x.kind, BindingKind::Const);
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
        let x = user_symbol(&checked.bound, "x");
        assert_eq!(checked.type_of_symbol(x.id), Type::Number);
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
        assert!(user_symbol(&bound, "x").name == "x");
        assert!(user_symbol(&bound, "y").name == "y");
        let use_span = find_ident_use(&bound.program, "x");
        let id = bound.resolve(use_span).expect("x in init should resolve");
        assert_eq!(bound.symbol(id).name, "x");
    }

    #[test]
    fn bind_resolves_global_math() {
        let program = parse("Math.abs(-1);").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Math");
        let id = bound.resolve(use_span).expect("Math should resolve");
        assert_eq!(bound.symbol(id).name, "Math");
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }

    #[test]
    fn check_math_is_object() {
        let program = parse("let t = typeof Math; let a = Math.abs(-3);").unwrap();
        let checked = check(program).unwrap();
        let math = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "Math" && s.span == Span::dummy())
            .expect("Math builtin");
        assert_eq!(checked.type_of_symbol(math.id), Type::Object);
    }

    #[test]
    fn bind_let_math_shadows_builtin() {
        let program = parse("let Math = 1; Math;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Math");
        let id = bound.resolve(use_span).expect("Math should resolve");
        assert_eq!(bound.symbol(id).name, "Math");
    }

    #[test]
    fn bind_resolves_global_number_nan_infinity() {
        let program = parse("Number.isNaN(NaN); Infinity;").unwrap();
        let bound = bind(program).unwrap();
        for name in ["Number", "NaN", "Infinity"] {
            let use_span = find_ident_use(&bound.program, name);
            let id = bound.resolve(use_span).expect("should resolve");
            assert_eq!(bound.symbol(id).name, name);
        }
    }

    #[test]
    fn check_types_global_number_nan_infinity() {
        let program = parse(
            "let t = typeof Number; let a = Number.isNaN(NaN); let n = NaN; let i = Infinity;",
        )
        .unwrap();
        let checked = check(program).unwrap();
        let number = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "Number" && s.span == Span::dummy())
            .expect("Number builtin");
        assert_eq!(checked.type_of_symbol(number.id), Type::Function);
        let nan = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "NaN" && s.span == Span::dummy())
            .expect("NaN builtin");
        assert_eq!(checked.type_of_symbol(nan.id), Type::Number);
        let inf = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "Infinity" && s.span == Span::dummy())
            .expect("Infinity builtin");
        assert_eq!(checked.type_of_symbol(inf.id), Type::Number);
    }

    #[test]
    fn bind_let_number_shadows_builtin() {
        let program = parse("let Number = 1; Number;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Number");
        let id = bound.resolve(use_span).expect("Number should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_symbol() {
        let program = parse("Symbol(); Symbol.for(\"x\");").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Symbol");
        let id = bound.resolve(use_span).expect("Symbol should resolve");
        assert_eq!(bound.symbol(id).name, "Symbol");
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }

    #[test]
    fn check_symbol_is_function() {
        let program = parse("let t = typeof Symbol; let s = Symbol();").unwrap();
        let checked = check(program).unwrap();
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "Symbol" && s.span == Span::dummy())
            .expect("Symbol builtin");
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }

    #[test]
    fn bind_let_symbol_shadows_builtin() {
        let program = parse("let Symbol = 1; Symbol;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Symbol");
        let id = bound.resolve(use_span).expect("Symbol should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_promise() {
        let program = parse("new Promise(function (r) { r(1); });").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Promise");
        let id = bound.resolve(use_span).expect("Promise should resolve");
        assert_eq!(bound.symbol(id).name, "Promise");
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }

    #[test]
    fn check_promise_is_function() {
        let program = parse("let t = typeof Promise; let p = new Promise(function (r) { r(1); });").unwrap();
        let checked = check(program).unwrap();
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "Promise" && s.span == Span::dummy())
            .expect("Promise builtin");
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }

    #[test]
    fn bind_let_promise_shadows_builtin() {
        let program = parse("let Promise = 1; Promise;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Promise");
        let id = bound.resolve(use_span).expect("Promise should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_proxy() {
        let program = parse("new Proxy({}, {});").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Proxy");
        let id = bound.resolve(use_span).expect("Proxy should resolve");
        assert_eq!(bound.symbol(id).name, "Proxy");
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }

    #[test]
    fn check_proxy_is_function() {
        let program = parse("let t = typeof Proxy; let p = new Proxy({}, {});").unwrap();
        let checked = check(program).unwrap();
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "Proxy" && s.span == Span::dummy())
            .expect("Proxy builtin");
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }

    #[test]
    fn check_proxy_of_function_is_callable() {
        let program = parse(
            "let t = function (a) { return a; }; let p = new Proxy(t, {}); let r = p(1);",
        )
        .unwrap();
        let checked = check(program).unwrap();
        let p = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "p")
            .expect("p");
        assert_eq!(checked.type_of_symbol(p.id), Type::Function);
    }

    #[test]
    fn check_proxy_of_object_is_not_callable() {
        let program = parse("let p = new Proxy({}, {}); p();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_let_proxy_shadows_builtin() {
        let program = parse("let Proxy = 1; Proxy;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Proxy");
        let id = bound.resolve(use_span).expect("Proxy should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_reflect() {
        let program = parse("Reflect.get({}, \"a\");").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Reflect");
        let id = bound.resolve(use_span).expect("Reflect should resolve");
        assert_eq!(bound.symbol(id).name, "Reflect");
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }

    #[test]
    fn check_reflect_is_object() {
        let program = parse("let t = typeof Reflect; let g = Reflect.get;").unwrap();
        let checked = check(program).unwrap();
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "Reflect" && s.span == Span::dummy())
            .expect("Reflect builtin");
        assert_eq!(checked.type_of_symbol(sym.id), Type::Object);
    }

    #[test]
    fn bind_let_reflect_shadows_builtin() {
        let program = parse("let Reflect = 1; Reflect;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Reflect");
        let id = bound.resolve(use_span).expect("Reflect should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_undefined_and_global_this() {
        let program = parse("undefined; globalThis;").unwrap();
        let bound = bind(program).unwrap();
        for name in ["undefined", "globalThis"] {
            let use_span = find_ident_use(&bound.program, name);
            let id = bound.resolve(use_span).expect("should resolve");
            assert_eq!(bound.symbol(id).name, name);
            assert_eq!(bound.symbol(id).kind, BindingKind::Const);
        }
    }

    #[test]
    fn check_types_undefined_and_global_this() {
        let program = parse("let u = undefined; let g = globalThis;").unwrap();
        let checked = check(program).unwrap();
        let undef = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "undefined" && s.span == Span::dummy())
            .expect("undefined builtin");
        assert_eq!(checked.type_of_symbol(undef.id), Type::Any);
        let gt = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "globalThis" && s.span == Span::dummy())
            .expect("globalThis builtin");
        assert_eq!(checked.type_of_symbol(gt.id), Type::Object);
    }

    #[test]
    fn bind_resolves_fundamental_constructors() {
        let program = parse("Object; Function; Array; String; Boolean;").unwrap();
        let bound = bind(program).unwrap();
        for name in ["Object", "Function", "Array", "String", "Boolean"] {
            let use_span = find_ident_use(&bound.program, name);
            let id = bound.resolve(use_span).expect("should resolve");
            assert_eq!(bound.symbol(id).name, name);
            assert_eq!(bound.symbol(id).kind, BindingKind::Const);
        }
    }

    #[test]
    fn check_fundamental_constructors_are_functions() {
        let program =
            parse("let a = typeof Object; let b = typeof Function; let c = typeof Array; let d = typeof String; let e = typeof Boolean;")
                .unwrap();
        let checked = check(program).unwrap();
        for name in ["Object", "Function", "Array", "String", "Boolean"] {
            let sym = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.name == name && s.span == Span::dummy())
                .unwrap_or_else(|| panic!("{name} builtin"));
            assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
        }
    }

    #[test]
    fn bind_let_object_shadows_builtin() {
        let program = parse("let Object = 1; Object;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Object");
        let id = bound.resolve(use_span).expect("Object should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_error_constructors() {
        let program = parse(
            "Error; TypeError; RangeError; ReferenceError; SyntaxError; URIError; EvalError; AggregateError;",
        )
        .unwrap();
        let bound = bind(program).unwrap();
        for name in [
            "Error",
            "TypeError",
            "RangeError",
            "ReferenceError",
            "SyntaxError",
            "URIError",
            "EvalError",
            "AggregateError",
        ] {
            let use_span = find_ident_use(&bound.program, name);
            let id = bound.resolve(use_span).expect("should resolve");
            assert_eq!(bound.symbol(id).name, name);
            assert_eq!(bound.symbol(id).kind, BindingKind::Const);
        }
    }

    #[test]
    fn check_error_constructors_are_functions() {
        let program = parse(
            "let a = typeof Error; let b = typeof TypeError; let c = typeof AggregateError;",
        )
        .unwrap();
        let checked = check(program).unwrap();
        for name in [
            "Error",
            "TypeError",
            "RangeError",
            "ReferenceError",
            "SyntaxError",
            "URIError",
            "EvalError",
            "AggregateError",
        ] {
            let sym = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.name == name && s.span == Span::dummy())
                .unwrap_or_else(|| panic!("{name} builtin"));
            assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
        }
    }

    #[test]
    fn check_new_error_is_ok() {
        let program = parse(
            "let e = new Error(\"m\"); let t = new TypeError(\"t\"); let a = new AggregateError([], \"a\");",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_error_shadows_builtin() {
        let program = parse("let Error = 1; Error;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Error");
        let id = bound.resolve(use_span).expect("Error should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_functions() {
        let program = parse("parseInt; parseFloat; isNaN; isFinite;").unwrap();
        let bound = bind(program).unwrap();
        for name in ["parseInt", "parseFloat", "isNaN", "isFinite"] {
            let use_span = find_ident_use(&bound.program, name);
            let id = bound.resolve(use_span).expect("should resolve");
            assert_eq!(bound.symbol(id).name, name);
            assert_eq!(bound.symbol(id).kind, BindingKind::Const);
        }
    }

    #[test]
    fn check_global_functions_are_functions() {
        let program = parse(
            "let a = typeof parseInt; let b = typeof parseFloat; let c = typeof isNaN; let d = typeof isFinite;",
        )
        .unwrap();
        let checked = check(program).unwrap();
        for name in ["parseInt", "parseFloat", "isNaN", "isFinite"] {
            let sym = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.name == name && s.span == Span::dummy())
                .unwrap_or_else(|| panic!("{name} builtin"));
            assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
        }
    }

    #[test]
    fn check_global_function_calls_ok() {
        let program = parse(
            "let a = parseInt(\"42\"); let b = parseFloat(\"3.14\"); let c = isNaN(NaN); let d = isFinite(1);",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_parse_int_shadows_builtin() {
        let program = parse("let parseInt = 1; parseInt;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "parseInt");
        let id = bound.resolve(use_span).expect("parseInt should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_uri_functions() {
        let program =
            parse("encodeURI; decodeURI; encodeURIComponent; decodeURIComponent;").unwrap();
        let bound = bind(program).unwrap();
        for name in [
            "encodeURI",
            "decodeURI",
            "encodeURIComponent",
            "decodeURIComponent",
        ] {
            let use_span = find_ident_use(&bound.program, name);
            let id = bound.resolve(use_span).expect("should resolve");
            assert_eq!(bound.symbol(id).name, name);
            assert_eq!(bound.symbol(id).kind, BindingKind::Const);
        }
    }

    #[test]
    fn check_uri_functions_are_functions() {
        let program = parse(
            "let a = typeof encodeURI; let b = typeof decodeURI; let c = typeof encodeURIComponent; let d = typeof decodeURIComponent;",
        )
        .unwrap();
        let checked = check(program).unwrap();
        for name in [
            "encodeURI",
            "decodeURI",
            "encodeURIComponent",
            "decodeURIComponent",
        ] {
            let sym = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.name == name && s.span == Span::dummy())
                .unwrap_or_else(|| panic!("{name} builtin"));
            assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
        }
    }

    #[test]
    fn check_uri_function_calls_ok() {
        let program = parse(
            "let a = encodeURI(\"a b\"); let b = decodeURI(\"a%20b\"); let c = encodeURIComponent(\"a&b\"); let d = decodeURIComponent(\"a%26b\");",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_encode_uri_shadows_builtin() {
        let program = parse("let encodeURI = 1; encodeURI;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "encodeURI");
        let id = bound.resolve(use_span).expect("encodeURI should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_escape_unescape() {
        let program = parse("escape; unescape;").unwrap();
        let bound = bind(program).unwrap();
        for name in ["escape", "unescape"] {
            let use_span = find_ident_use(&bound.program, name);
            let id = bound.resolve(use_span).expect("should resolve");
            assert_eq!(bound.symbol(id).name, name);
            assert_eq!(bound.symbol(id).kind, BindingKind::Const);
        }
    }

    #[test]
    fn check_escape_unescape_are_functions() {
        let program = parse("let a = typeof escape; let b = typeof unescape;").unwrap();
        let checked = check(program).unwrap();
        for name in ["escape", "unescape"] {
            let sym = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.name == name && s.span == Span::dummy())
                .unwrap_or_else(|| panic!("{name} builtin"));
            assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
        }
    }

    #[test]
    fn check_escape_unescape_calls_ok() {
        let program = parse(
            "let a = escape(\"a b\"); let b = unescape(\"%20\"); let c = unescape(escape(\"x\"));",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_escape_shadows_builtin() {
        let program = parse("let escape = 1; escape;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "escape");
        let id = bound.resolve(use_span).expect("escape should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_json() {
        let program = parse("JSON;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "JSON");
        let id = bound.resolve(use_span).expect("JSON should resolve");
        assert_eq!(bound.symbol(id).name, "JSON");
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }

    #[test]
    fn check_json_is_object() {
        let program = parse("let t = typeof JSON; let p = JSON.parse; let s = JSON.stringify;").unwrap();
        let checked = check(program).unwrap();
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "JSON" && s.span == Span::dummy())
            .expect("JSON builtin");
        assert_eq!(checked.type_of_symbol(sym.id), Type::Object);
    }

    #[test]
    fn check_json_parse_stringify_calls_ok() {
        let program = parse(
            "let a = JSON.stringify(1); let b = JSON.parse(\"1\"); let c = JSON.parse(JSON.stringify({ x: 2 }));",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_json_shadows_builtin() {
        let program = parse("let JSON = 1; JSON;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "JSON");
        let id = bound.resolve(use_span).expect("JSON should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_date() {
        let program = parse("Date;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Date");
        let id = bound.resolve(use_span).expect("Date should resolve");
        assert_eq!(bound.symbol(id).name, "Date");
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }

    #[test]
    fn check_date_is_function() {
        let program = parse("let t = typeof Date; let n = Date.now;").unwrap();
        let checked = check(program).unwrap();
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "Date" && s.span == Span::dummy())
            .expect("Date builtin");
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }

    #[test]
    fn check_date_now_and_new_ok() {
        let program = parse(
            "let n = Date.now(); let d = new Date(0); let t = d.getTime(); let u = Date.UTC(1970, 0, 1);",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_date_shadows_builtin() {
        let program = parse("let Date = 1; Date;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Date");
        let id = bound.resolve(use_span).expect("Date should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_regexp() {
        let program = parse("RegExp;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "RegExp");
        let id = bound.resolve(use_span).expect("RegExp should resolve");
        assert_eq!(bound.symbol(id).name, "RegExp");
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
    }

    #[test]
    fn check_regexp_is_function() {
        let program = parse("let t = typeof RegExp; let s = RegExp.prototype;").unwrap();
        let checked = check(program).unwrap();
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "RegExp" && s.span == Span::dummy())
            .expect("RegExp builtin");
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }

    #[test]
    fn check_regexp_new_and_methods_ok() {
        let program = parse(
            "let r = new RegExp(\"a+\", \"i\"); let t = r.test(\"AA\"); let m = r.exec(\"xAAy\"); let s = r.source; let f = r.flags;",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_regexp_shadows_builtin() {
        let program = parse("let RegExp = 1; RegExp;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "RegExp");
        let id = bound.resolve(use_span).expect("RegExp should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_map_and_set() {
        let program = parse("Map; Set;").unwrap();
        let bound = bind(program).unwrap();
        for name in ["Map", "Set"] {
            let use_span = find_ident_use(&bound.program, name);
            let id = bound.resolve(use_span).expect(name);
            assert_eq!(bound.symbol(id).name, name);
            assert_eq!(bound.symbol(id).kind, BindingKind::Const);
        }
    }

    #[test]
    fn check_map_and_set_are_functions() {
        let program = parse("let tm = typeof Map; let ts = typeof Set;").unwrap();
        let checked = check(program).unwrap();
        for name in ["Map", "Set"] {
            let sym = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.name == name && s.span == Span::dummy())
                .expect(name);
            assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
        }
    }

    #[test]
    fn check_map_set_new_and_methods_ok() {
        let program = parse(
            "let m = new Map(); m.set(1, 2); let g = m.get(1); let h = m.has(1); let n = m.size; let s = new Set(); s.add(3); let sh = s.has(3); let sn = s.size;",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_map_shadows_builtin() {
        let program = parse("let Map = 1; Map;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "Map");
        let id = bound.resolve(use_span).expect("Map should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_weak_map_and_weak_set() {
        let program = parse("WeakMap; WeakSet;").unwrap();
        let bound = bind(program).unwrap();
        for name in ["WeakMap", "WeakSet"] {
            let use_span = find_ident_use(&bound.program, name);
            let id = bound.resolve(use_span).expect(name);
            assert_eq!(bound.symbol(id).name, name);
            assert_eq!(bound.symbol(id).kind, BindingKind::Const);
        }
    }

    #[test]
    fn check_weak_map_and_weak_set_are_functions() {
        let program = parse("let twm = typeof WeakMap; let tws = typeof WeakSet;").unwrap();
        let checked = check(program).unwrap();
        for name in ["WeakMap", "WeakSet"] {
            let sym = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.name == name && s.span == Span::dummy())
                .expect(name);
            assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
        }
    }

    #[test]
    fn check_weak_map_set_new_and_methods_ok() {
        let program = parse(
            "let k = {}; let wm = new WeakMap(); wm.set(k, 1); let g = wm.get(k); let h = wm.has(k); let d = wm.delete(k); let ws = new WeakSet(); ws.add(k); let sh = ws.has(k); let sd = ws.delete(k);",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_weak_map_shadows_builtin() {
        let program = parse("let WeakMap = 1; WeakMap;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "WeakMap");
        let id = bound.resolve(use_span).expect("WeakMap should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_global_arraybuffer_dataview_typedarrays() {
        let program = parse(
            "ArrayBuffer; DataView; Uint8Array; Int32Array; Float64Array;",
        )
        .unwrap();
        let bound = bind(program).unwrap();
        for name in [
            "ArrayBuffer",
            "DataView",
            "Uint8Array",
            "Int32Array",
            "Float64Array",
        ] {
            let use_span = find_ident_use(&bound.program, name);
            let id = bound.resolve(use_span).expect(name);
            assert_eq!(bound.symbol(id).name, name);
            assert_eq!(bound.symbol(id).kind, BindingKind::Const);
        }
    }

    #[test]
    fn check_arraybuffer_dataview_typedarrays_are_functions() {
        let program = parse(
            "let tab = typeof ArrayBuffer; let tdv = typeof DataView; let tu8 = typeof Uint8Array;",
        )
        .unwrap();
        let checked = check(program).unwrap();
        for name in ["ArrayBuffer", "DataView", "Uint8Array", "Int32Array", "Float64Array"] {
            let sym = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.name == name && s.span == Span::dummy())
                .expect(name);
            assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
        }
    }

    #[test]
    fn check_arraybuffer_typedarrays_new_and_ops_ok() {
        let program = parse(
            "let buf = new ArrayBuffer(8); let bl = buf.byteLength; let u8 = new Uint8Array(buf); u8[0] = 1; let x = u8[0]; let i32 = new Int32Array(2); i32[0] = 42; let f64 = new Float64Array([1.5]); let dv = new DataView(buf); dv.setUint8(0, 1); let g = dv.getUint8(0);",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_arraybuffer_shadows_builtin() {
        let program = parse("let ArrayBuffer = 1; ArrayBuffer;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "ArrayBuffer");
        let id = bound.resolve(use_span).expect("ArrayBuffer should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn bind_resolves_eval() {
        let program = parse("eval;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "eval");
        let id = bound.resolve(use_span).expect("eval should resolve");
        assert_eq!(bound.symbol(id).name, "eval");
        assert_eq!(bound.symbol(id).kind, BindingKind::Const);
        assert_eq!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn check_eval_is_function() {
        let program = parse("let t = typeof eval;").unwrap();
        let checked = check(program).unwrap();
        let sym = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "eval" && s.span == Span::dummy())
            .expect("eval builtin");
        assert_eq!(checked.type_of_symbol(sym.id), Type::Function);
    }

    #[test]
    fn check_eval_call_ok() {
        let program = parse("let a = eval(\"1 + 2\"); let b = eval(\"typeof undefined\");").unwrap();
        check(program).unwrap();
    }

    #[test]
    fn bind_let_eval_shadows_builtin() {
        let program = parse("let eval = 1; eval;").unwrap();
        let bound = bind(program).unwrap();
        let use_span = find_ident_use(&bound.program, "eval");
        let id = bound.resolve(use_span).expect("eval should resolve");
        assert_ne!(bound.symbol(id).span, Span::dummy());
    }

    #[test]
    fn check_new_function_is_function() {
        let program = parse("let f = new Function(\"return 1\");").unwrap();
        let checked = check(program).unwrap();
        let f = checked
            .bound
            .symbols()
            .iter()
            .find(|s| s.name == "f")
            .expect("f");
        assert_eq!(checked.type_of_symbol(f.id), Type::Function);
    }

    #[test]
    fn check_new_function_call_ok() {
        let program = parse(
            "let f = new Function(\"a\", \"b\", \"return a + b\"); let r = f(1, 2); let g = Function(\"x\", \"return x\"); let s = g(3);",
        )
        .unwrap();
        check(program).unwrap();
    }

    #[test]
    fn check_function_call_construct_ok() {
        let program = parse("let f = Function(\"return 7\"); let r = f();").unwrap();
        check(program).unwrap();
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
    fn bind_catch_var_same_name_allowed_annex_b() {
        let program = parse(
            r#"function f() {
                try { throw 1; } catch (e) { var e = 2; return e; }
            }"#,
        )
        .unwrap();
        bind(program).expect("Annex B.3.4 allows var same name as catch param");
    }

    #[test]
    fn bind_catch_let_same_name_errors() {
        let program = parse("try { throw 1; } catch (e) { let e = 2; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("e"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_catch_function_same_name_errors() {
        let program = parse("try { throw 1; } catch (e) { function e() {} }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("e"),
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
    fn bind_resolves_arguments_in_function() {
        let program = parse("function f(a) { return arguments.length + arguments[0]; }").unwrap();
        let bound = bind(program).unwrap();
        let args_span = find_ident_use(&bound.program, "arguments");
        let sym = bound.symbol(bound.resolve(args_span).unwrap());
        assert_eq!(sym.name, "arguments");
        assert_eq!(sym.kind, BindingKind::Var);
    }

    #[test]
    fn bind_arguments_unresolved_in_arrow_at_top_level() {
        let program = parse("let f = () => arguments.length;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("unresolved") && err.message.contains("arguments"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn check_arguments_in_function_ok() {
        let program =
            parse("function f(a, b) { return arguments.length + arguments[0]; } let r = f(1, 2);")
                .unwrap();
        check(program).unwrap();
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
    fn check_unary_plus_coerces_to_number() {
        let program = parse(r#"let a = +"42"; let b = +true; let c = +null; let d = +"";"#).unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Number);
        assert_eq!(sym_type(&checked, "b"), Type::Number);
        assert_eq!(sym_type(&checked, "c"), Type::Number);
        assert_eq!(sym_type(&checked, "d"), Type::Number);
    }

    #[test]
    fn check_unary_plus_rejects_bigint() {
        let program = parse("let a = +1n;").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("unary") && err.message.contains("bigint"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn check_add_coercion_types() {
        let program = parse(
            r#"let a = "a" + true; let b = true + 1; let c = null + 1; let d = false + true;"#,
        )
        .unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::String);
        assert_eq!(sym_type(&checked, "b"), Type::Number);
        assert_eq!(sym_type(&checked, "c"), Type::Number);
        assert_eq!(sym_type(&checked, "d"), Type::Number);
    }

    #[test]
    fn check_abstract_eq_mixed_types() {
        let program = parse(r#"let a = 1 == "1"; let b = null == 0; let c = true != "1";"#).unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Boolean);
        assert_eq!(sym_type(&checked, "b"), Type::Boolean);
        assert_eq!(sym_type(&checked, "c"), Type::Boolean);
    }

    #[test]
    fn check_to_primitive_object_ops() {
        // valueOf/toString run at runtime; static type of object + primitive is Any.
        let program = parse(
            r#"
            let o = { valueOf: function () { return 1; } };
            let a = o + 2;
            let b = "x" + o;
            let c = o == 1;
            let d = o != "1";
            "#,
        )
        .unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Any);
        assert_eq!(sym_type(&checked, "b"), Type::String);
        assert_eq!(sym_type(&checked, "c"), Type::Boolean);
        assert_eq!(sym_type(&checked, "d"), Type::Boolean);
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
                | Expr::BigInt(_)
                | Expr::String(_)
                | Expr::RegExp { .. }
                | Expr::Boolean { .. }
                | Expr::Null { .. }
                | Expr::This { .. }
                | Expr::Super { .. } => {}
                Expr::TemplateLiteral { expressions, .. } => {
                    for e in expressions {
                        walk_expr(e, name, out);
                    }
                }
                Expr::TaggedTemplate {
                    tag, expressions, ..
                } => {
                    walk_expr(tag, name, out);
                    for e in expressions {
                        walk_expr(e, name, out);
                    }
                }
                Expr::Unary { arg, .. }
                | Expr::Paren { expr: arg, .. }
                | Expr::As { expr: arg, .. } => {
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
                Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
                    walk_expr(callee, name, out);
                    for a in args {
                        match a {
                            Arg::Expr(expr) | Arg::Spread(expr) => walk_expr(expr, name, out),
                        }
                    }
                }
                Expr::ObjectExpression { properties, .. } => {
                    for prop in properties {
                        match prop {
                            ObjectProp::Property { key, value, .. } => {
                                if let ObjectKey::Computed(expr) = key {
                                    walk_expr(expr, name, out);
                                }
                                walk_expr(value, name, out);
                            }
                            ObjectProp::Accessor { key, body, .. } => {
                                if let ObjectKey::Computed(expr) = key {
                                    walk_expr(expr, name, out);
                                }
                                walk_stmt(body, name, out);
                            }
                            ObjectProp::Spread { expr, .. } => walk_expr(expr, name, out),
                        }
                    }
                }
                Expr::ArrayExpression { elements, .. } => {
                    for el in elements {
                        match el {
                            ArrayElement::Expr(expr) | ArrayElement::Spread(expr) => {
                                walk_expr(expr, name, out);
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
                    walk_expr(object, name, out);
                    if *computed {
                        walk_expr(property, name, out);
                    }
                }
                // Function bodies walked via Stmt::FunctionDeclaration path when needed.
                Expr::FunctionExpression { .. } | Expr::ArrowFunction { .. } => {}
                Expr::ArrayPattern { elements, .. } => {
                    for el in elements {
                        match el {
                            ArrayPatternElement::Pattern {
                                binding: BindingPattern::Ident(id),
                                default,
                            } if id.name == name => {
                                *out = Some(id.span);
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ArrayPatternElement::Pattern {
                                binding: BindingPattern::Ident(_),
                                default,
                            } => {
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ArrayPatternElement::Pattern {
                                binding:
                                    BindingPattern::Array {
                                        elements: nested, ..
                                    },
                                default,
                            } => {
                                walk_expr(
                                    &Expr::ArrayPattern {
                                        elements: nested.clone(),
                                        span: Span::dummy(),
                                    },
                                    name,
                                    out,
                                );
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ArrayPatternElement::Pattern {
                                binding: BindingPattern::Object { properties, .. },
                                default,
                            } => {
                                walk_expr(
                                    &Expr::ObjectPattern {
                                        properties: properties.clone(),
                                        span: Span::dummy(),
                                    },
                                    name,
                                    out,
                                );
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ArrayPatternElement::Rest(id) if id.name == name => {
                                *out = Some(id.span);
                            }
                            ArrayPatternElement::Rest(_) => {}
                        }
                    }
                }
                Expr::ObjectPattern { properties, .. } => {
                    for p in properties {
                        match p {
                            ObjectPatternProp::Prop {
                                binding: BindingPattern::Ident(id),
                                default,
                                ..
                            } if id.name == name => {
                                *out = Some(id.span);
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ObjectPatternProp::Prop {
                                binding: BindingPattern::Ident(_),
                                default,
                                ..
                            } => {
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ObjectPatternProp::Prop {
                                binding: BindingPattern::Array { elements, .. },
                                default,
                                ..
                            } => {
                                walk_expr(
                                    &Expr::ArrayPattern {
                                        elements: elements.clone(),
                                        span: Span::dummy(),
                                    },
                                    name,
                                    out,
                                );
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ObjectPatternProp::Prop {
                                binding:
                                    BindingPattern::Object {
                                        properties: nested,
                                        ..
                                    },
                                default,
                                ..
                            } => {
                                walk_expr(
                                    &Expr::ObjectPattern {
                                        properties: nested.clone(),
                                        span: Span::dummy(),
                                    },
                                    name,
                                    out,
                                );
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ObjectPatternProp::Rest(id) if id.name == name => {
                                *out = Some(id.span);
                            }
                            ObjectPatternProp::Rest(_) => {}
                        }
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
                Stmt::Let { init: None, .. }
                | Stmt::Empty { .. }
                | Stmt::TypeAlias { .. } => {}
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
                Stmt::ClassDeclaration {
                    super_class,
                    body,
                    ..
                } => {
                    if let Some(sc) = super_class {
                        walk_expr(sc, name, out);
                    }
                    for el in body {
                        match el {
                            ClassElement::Constructor { body, .. }
                            | ClassElement::Method { body, .. }
                            | ClassElement::Accessor { body, .. } => {
                                walk_stmt(body, name, out);
                            }
                        }
                    }
                }
                Stmt::Return {
                    argument: Some(arg),
                    ..
                } => walk_expr(arg, name, out),
                Stmt::Return {
                    argument: None, ..
                } => {}
                Stmt::Throw { argument, .. } => walk_expr(argument, name, out),
                Stmt::Try {
                    block,
                    handler,
                    finalizer,
                    ..
                } => {
                    walk_stmt(block, name, out);
                    if let Some(handler) = handler {
                        walk_stmt(handler, name, out);
                    }
                    if let Some(finalizer) = finalizer {
                        walk_stmt(finalizer, name, out);
                    }
                }
                Stmt::With { object, body, .. } => {
                    walk_expr(object, name, out);
                    walk_stmt(body, name, out);
                }
                Stmt::ImportDeclaration { .. }
                | Stmt::ExportNamedDeclaration { .. }
                | Stmt::ExportDefaultDeclaration { .. } => {}
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
                | Expr::Update { arg, .. }
                | Expr::As { expr: arg, .. } => walk(arg, op, out),
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
                        match a {
                            Arg::Expr(expr) | Arg::Spread(expr) => walk(expr, op, out),
                        }
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
