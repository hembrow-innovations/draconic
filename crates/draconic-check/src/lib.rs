//! Binder (scopes + symbol resolution) and Checker (TypeScript-inspired).
//! Binder: ROADMAP B04. Checker: ROADMAP B05. Host API registry: H00.01.

mod binder;
mod checker;
mod checker_assign;
mod checker_expr;
mod checker_ops;
mod checker_stmt;
mod checker_type;
mod host_api;

use checker::Checker;

pub use host_api::{
    host_apis, is_available as host_api_is_available, is_host_api, lookup as lookup_host_api,
    unsupported_diagnostic as host_api_unsupported_diagnostic, CompileTarget, HostApiEntry,
    HostAvailability,
};

use draconic_ast::{
    Arg, ArrayElement, ArrowBody, BindingKind, BindingPattern, Expr, ObjectProp, Param, Program,
    Stmt, TypeAnn,
};
use draconic_diagnostics::{codes, Diagnostic, Span};
use std::collections::HashMap;
use std::fmt;

/// Hard diagnostic when `extern "C"` / FFI appears on the js target (F08.01).
pub fn extern_unsupported_on_js_diagnostic(name: &str, span: Span) -> Diagnostic {
    Diagnostic::new(
        format!("extern \"C\" function `{name}` is unsupported on js target (native-only FFI)"),
        span,
    )
    .with_code(codes::EXTERN_UNSUPPORTED)
    .with_help("compile with the native backend, or remove the extern declaration")
}

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
    /// Unboxed native boolean (N02); distinct from JS `boolean`.
    Bool,
}

impl NativeType {
    pub fn from_name(name: &str) -> Option<Self> {
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
            "bool" => Self::Bool,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
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
            Self::Bool => "bool",
        }
    }

    pub fn is_float(self) -> bool {
        matches!(self, Self::F32 | Self::F64)
    }

    pub fn is_bool(self) -> bool {
        matches!(self, Self::Bool)
    }

    /// Integer native types only (`i8`–`i64`, `u8`–`u64`).
    pub fn is_int(self) -> bool {
        !self.is_float() && !self.is_bool()
    }

    pub fn is_signed(self) -> bool {
        matches!(self, Self::I8 | Self::I16 | Self::I32 | Self::I64)
    }

    pub fn bit_width(self) -> u32 {
        match self {
            Self::I8 | Self::U8 | Self::Bool => 8,
            Self::I16 | Self::U16 => 16,
            Self::I32 | Self::U32 | Self::F32 => 32,
            Self::I64 | Self::U64 | Self::F64 => 64,
        }
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
    /// Pointer to a native scalar (`*i32`, …); N03.03.
    Ptr(NativeType),
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

/// Property list for a structural object type (`Type::Shape`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectShape {
    pub props: Vec<(String, Type)>,
    /// True when the shape came from an explicit type annotation (`{ x: number }`).
    /// Only strict shapes reject access to unknown properties (T07.03); inferred
    /// object-literal and tuple shapes stay permissive so untyped JS is dynamic.
    pub strict: bool,
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
            Type::Ptr(n) => {
                return write!(f, "*{}", n.as_str());
            }
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

    /// Smallest use-site identifier span that contains `offset` (UTF-8 bytes),
    /// with the symbol it resolves to. Used by LSP hover / go-to-definition.
    pub fn use_at_offset(&self, offset: u32) -> Option<(Span, SymbolId)> {
        self.resolutions
            .iter()
            .filter(|(span, _)| span_contains_offset(**span, offset))
            .min_by_key(|(span, _)| span.len())
            .map(|(span, id)| (*span, *id))
    }

    /// Declaration symbol whose binding-name span contains `offset`, if any
    /// (smallest span wins).
    pub fn decl_at_offset(&self, offset: u32) -> Option<&Symbol> {
        self.symbols
            .iter()
            .filter(|s| span_contains_offset(s.span, offset))
            .min_by_key(|s| s.span.len())
    }
}

fn span_contains_offset(span: Span, offset: u32) -> bool {
    if span.is_dummy() {
        return false;
    }
    // Half-open [start, end), plus the caret resting on the end of a non-empty span.
    if span.start.0 == span.end.0 {
        return offset == span.start.0;
    }
    offset >= span.start.0 && offset <= span.end.0
}

/// `void` type annotation (keyword parsed as Named "void"); valid only as extern return (F06.02).
fn is_void_type_ann(ann: &TypeAnn) -> bool {
    matches!(ann, TypeAnn::Named { name, .. } if name == "void")
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
    /// Named type aliases (`type Pair = { … }`) for ABI re-resolution (F03.02).
    type_aliases: HashMap<String, Type>,
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

    /// Smallest typed expression span containing `offset` (UTF-8 bytes).
    pub fn expr_type_at_offset(&self, offset: u32) -> Option<(Span, Type)> {
        self.expr_types
            .iter()
            .filter(|(span, _)| span_contains_offset(**span, offset))
            .min_by_key(|(span, _)| span.len())
            .map(|(span, ty)| (*span, *ty))
    }

    pub fn shapes(&self) -> &[ObjectShape] {
        &self.shapes
    }

    /// Resolved type for a named alias, if one was declared.
    pub fn type_alias(&self, name: &str) -> Option<Type> {
        self.type_aliases.get(name).copied()
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
                    format!(
                        "{n}: {}",
                        format_type_full(*t, shapes, unions, intersections)
                    )
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
    let mut checker = Checker::new();
    checker.typecheck = false;
    checker.analyze(&program, false)?;
    Ok(checker.into_bound(program))
}

/// Bind under Module goal (E19.67): top-level functions are lexical, not var-like.
pub fn bind_module(program: Program) -> Result<BoundProgram, Diagnostic> {
    let mut checker = Checker::new();
    checker.typecheck = false;
    checker.binder.strict = true;
    checker.analyze(&program, true)?;
    Ok(checker.into_bound(program))
}

pub fn check(program: Program) -> Result<CheckedProgram, Diagnostic> {
    // Script goal: top-level `await` / `for await` rejected.
    // No host-target policy (call [`check_for_target`] when the backend is known).
    check_with_module_goal(program, false, None)
}

/// Check a Program under the Module goal (E19.28): top-level `await` and
/// `for await` are allowed (async module). Nested non-async functions still
/// reject `await`.
pub fn check_module(program: Program) -> Result<CheckedProgram, Diagnostic> {
    check_with_module_goal(program, true, None)
}

/// Check a Script-goal Program for a specific compile target (H00.01).
///
/// Free references to registered host APIs that are unavailable on `target`
/// produce a hard diagnostic ([`codes::HOST_API_UNSUPPORTED`]).
pub fn check_for_target(
    program: Program,
    target: CompileTarget,
) -> Result<CheckedProgram, Diagnostic> {
    check_with_module_goal(program, false, Some(target))
}

/// Check a Module-goal Program for a specific compile target (H00.01).
pub fn check_module_for_target(
    program: Program,
    target: CompileTarget,
) -> Result<CheckedProgram, Diagnostic> {
    check_with_module_goal(program, true, Some(target))
}

fn check_with_module_goal(
    program: Program,
    module_goal: bool,
    target: Option<CompileTarget>,
) -> Result<CheckedProgram, Diagnostic> {
    let mut checker = Checker::new();
    checker.typecheck = true;
    // Module evaluation may be async when the body uses top-level await.
    checker.in_async = module_goal;
    checker.host_target = target;
    if module_goal {
        checker.binder.strict = true;
    }
    checker.analyze(&program, module_goal)?;
    Ok(checker.into_checked(program))
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

/// If any catch-parameter bound name appears in LexicallyDeclaredNames of the
/// catch Block, return the conflicting name and its declaration span. Annex
/// B.3.4 allows the same name in VarDeclaredNames (`var`); only lexical
/// `let`/`const`/`class`/`function` at the top level of the catch block are rejected.
fn catch_lexical_conflict(param: &BindingPattern, handler: &Stmt) -> Option<(String, Span)> {
    let body: &[Stmt] = match handler {
        Stmt::Block { body, .. } => body.as_slice(),
        other => std::slice::from_ref(other),
    };
    let mut conflict = None;
    param.for_each_ident(&mut |id| {
        if conflict.is_some() {
            return;
        }
        for stmt in body {
            if let Some(span) = catch_stmt_lexical_name(stmt, &id.name) {
                conflict = Some((id.name.clone(), span));
                return;
            }
        }
    });
    conflict
}

fn catch_stmt_lexical_name(stmt: &Stmt, param: &str) -> Option<Span> {
    let mut s = stmt;
    while let Stmt::Labeled { body, .. } = s {
        s = body;
    }
    match s {
        Stmt::Let {
            kind:
                BindingKind::Let | BindingKind::Const | BindingKind::Using | BindingKind::AwaitUsing,
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

/// Lexical binding kind for statement-list early errors (E19.24 / Annex B.3.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LexNameKind {
    /// Plain `function` (not async/generator): sloppy mode may allow duplicates among these only.
    PlainFunction,
    Other,
}

fn peel_labels(stmt: &Stmt) -> &Stmt {
    let mut s = stmt;
    while let Stmt::Labeled { body, .. } = s {
        s = body;
    }
    s
}

/// ECMA-262 IsSimpleParameterList: only BindingIdentifiers, no rest/defaults.
fn is_simple_parameter_list(params: &[Param]) -> bool {
    params
        .iter()
        .all(|p| !p.rest && p.default.is_none() && matches!(p.binding, BindingPattern::Ident(_)))
}

/// SuperCall in parameter defaults (E19.39 method early error).
fn params_contain_super_call(params: &[Param]) -> bool {
    params
        .iter()
        .any(|p| p.default.as_ref().is_some_and(expr_contains_super_call))
}

/// SuperCall or SuperProperty in formals (plain / async / generator functions).
fn params_contain_super(params: &[Param]) -> bool {
    params
        .iter()
        .any(|p| p.default.as_ref().is_some_and(expr_contains_super))
}

/// True when `expr` is (or chains from) an OptionalExpression (`?.`).
fn expr_has_optional_chain(expr: &Expr) -> bool {
    match expr {
        Expr::MemberExpression {
            object, optional, ..
        } => *optional || expr_has_optional_chain(object),
        Expr::Call {
            callee, optional, ..
        } => *optional || expr_has_optional_chain(callee),
        Expr::Paren { expr: inner, .. } => expr_has_optional_chain(inner),
        _ => false,
    }
}

fn expr_contains_super_call(expr: &Expr) -> bool {
    match expr {
        Expr::Call { callee, args, .. } => {
            matches!(callee.as_ref(), Expr::Super { .. })
                || expr_contains_super_call(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_super_call(e),
                })
        }
        Expr::ArrowFunction { body, params, .. } => {
            params_contain_super_call(params)
                || match body {
                    ArrowBody::Expr(e) => expr_contains_super_call(e),
                    ArrowBody::Block(s) => stmt_contains_super_call(s),
                }
        }
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_super_call(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_super_call(left) || expr_contains_super_call(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super_call(test)
                || expr_contains_super_call(consequent)
                || expr_contains_super_call(alternate)
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_super_call(object) || expr_contains_super_call(property),
        Expr::New { callee, args, .. } => {
            expr_contains_super_call(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_super_call(e),
                })
        }
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_super_call(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } => expr_contains_super_call(value),
            ObjectProp::Spread { expr, .. } => expr_contains_super_call(expr),
            ObjectProp::Accessor { .. } => false,
        }),
        _ => false,
    }
}

/// SuperCall or SuperProperty (not nested in inner functions/classes).
fn expr_contains_super(expr: &Expr) -> bool {
    match expr {
        Expr::Super { .. } => true,
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_super(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_super(e),
                })
        }
        Expr::ArrowFunction { body, params, .. } => {
            params_contain_super(params)
                || match body {
                    ArrowBody::Expr(e) => expr_contains_super(e),
                    ArrowBody::Block(s) => stmt_contains_super(s),
                }
        }
        // Nested function/class bodies are their own ContainsSuper roots.
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_super(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_super(left) || expr_contains_super(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super(test)
                || expr_contains_super(consequent)
                || expr_contains_super(alternate)
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_super(object) || expr_contains_super(property),
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_super(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } => expr_contains_super(value),
            ObjectProp::Spread { expr, .. } => expr_contains_super(expr),
            ObjectProp::Accessor { .. } => false,
        }),
        _ => false,
    }
}

fn stmt_contains_super(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_super),
        Stmt::Expression { expr, .. } => expr_contains_super(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_super),
        Stmt::Throw { argument, .. } => expr_contains_super(argument),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super(test)
                || stmt_contains_super(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_contains_super(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_contains_super(test) || stmt_contains_super(body)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_super),
        // Nested function/class declarations are separate ContainsSuper roots.
        Stmt::FunctionDeclaration { .. } | Stmt::ClassDeclaration { .. } => false,
        _ => false,
    }
}

fn stmt_contains_super_call(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_super_call),
        Stmt::Expression { expr, .. } => expr_contains_super_call(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_super_call),
        Stmt::Throw { argument, .. } => expr_contains_super_call(argument),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super_call(test)
                || stmt_contains_super_call(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_contains_super_call(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_contains_super_call(test) || stmt_contains_super_call(body)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_super_call),
        _ => false,
    }
}

/// `true` when `stmts` begins with a `"use strict"` directive prologue.
fn stmt_list_has_use_strict(stmts: &[Stmt]) -> bool {
    for stmt in stmts {
        match stmt {
            Stmt::Expression {
                expr: Expr::String(s),
                ..
            } => {
                if s.value.to_string_lossy() == "use strict" {
                    return true;
                }
            }
            _ => break,
        }
    }
    false
}

fn body_has_use_strict(body: &Stmt) -> bool {
    match body {
        Stmt::Block { body, .. } => stmt_list_has_use_strict(body),
        _ => false,
    }
}

/// `true` for the literal `true` expression (used by T07.02 loop reachability).
fn is_literal_true(expr: &Expr) -> bool {
    matches!(expr, Expr::Boolean { value: true, .. })
}

/// Whether `stmt` — the body of a loop — contains an unlabeled `break` that would
/// exit that loop, i.e. a `break` not shadowed by an inner loop or switch.
fn loop_body_has_escaping_break(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(loop_body_has_escaping_break),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            loop_body_has_escaping_break(consequent)
                || alternate
                    .as_deref()
                    .is_some_and(loop_body_has_escaping_break)
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            loop_body_has_escaping_break(block)
                || handler.as_deref().is_some_and(loop_body_has_escaping_break)
                || finalizer
                    .as_deref()
                    .is_some_and(loop_body_has_escaping_break)
        }
        Stmt::Labeled { body, .. } => loop_body_has_escaping_break(body),
        // A `break` inside these targets the inner construct, not the outer loop.
        Stmt::While { .. }
        | Stmt::DoWhile { .. }
        | Stmt::For { .. }
        | Stmt::ForIn { .. }
        | Stmt::ForOf { .. }
        | Stmt::Switch { .. } => false,
        Stmt::Break { label, .. } => label.is_none(),
        _ => false,
    }
}

/// Whether control flow can never reach the end of `stmt` (always returns, throws,
/// or loops forever). Conservative toward "terminates" to avoid false positives on
/// valid code; used by the T07.02 missing-return check.
fn stmt_cannot_fall_through(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } | Stmt::Throw { .. } => true,
        Stmt::Block { body, .. } => body.iter().any(stmt_cannot_fall_through),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => match alternate {
            Some(alt) => stmt_cannot_fall_through(consequent) && stmt_cannot_fall_through(alt),
            None => false,
        },
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            is_literal_true(test) && !loop_body_has_escaping_break(body)
        }
        Stmt::For { test, body, .. } => {
            test.as_ref().is_none_or(is_literal_true) && !loop_body_has_escaping_break(body)
        }
        Stmt::Switch { cases, .. } => {
            // A `default` must exist (a non-matching discriminant otherwise exits the
            // switch) and the concatenated case bodies, in source order, must reach a
            // terminating statement (case bodies fall through to the next case).
            if !cases.iter().any(|c| c.test.is_none()) {
                return false;
            }
            cases
                .iter()
                .any(|c| c.body.iter().any(stmt_cannot_fall_through))
        }
        _ => false,
    }
}

fn stmt_span(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Expression { span, .. }
        | Stmt::Let { span, .. }
        | Stmt::Empty { span }
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
        | Stmt::ImportDeclaration { span, .. }
        | Stmt::ExportNamedDeclaration { span, .. }
        | Stmt::ExportDefaultDeclaration { span, .. }
        | Stmt::ExportAllDeclaration { span, .. }
        | Stmt::TypeAlias { span, .. }
        | Stmt::ExternFunctionDeclaration { span, .. } => *span,
    }
}

/// Peel covering parentheses (E19.60 cover IdentifierReference).
fn peel_parens(expr: &Expr) -> &Expr {
    let mut inner = expr;
    while let Expr::Paren { expr, .. } = inner {
        inner = expr.as_ref();
    }
    inner
}

/// The formal parameters of a function expression / arrow value (peeling parens),
/// if `expr` denotes one. Used to record call signatures for function bindings (T07.01).
fn fn_params_of_expr(expr: &Expr) -> Option<&[Param]> {
    match peel_parens(expr) {
        Expr::FunctionExpression { params, .. } | Expr::ArrowFunction { params, .. } => {
            Some(params)
        }
        _ => None,
    }
}

/// Peel parens; if the core is Ident `eval`/`arguments`, return (name, span). E19.49.
fn strict_forbidden_assign_target(expr: &Expr) -> Option<(String, Span)> {
    match peel_parens(expr) {
        Expr::Ident(id) if id.name == "eval" || id.name == "arguments" => {
            Some((id.name.clone(), id.span))
        }
        _ => None,
    }
}

/// LexicallyDeclaredNames of a StatementList (not nested blocks).
///
/// When `top_level` (Script / FunctionBody), hoistable `function`/`async`/`generator`
/// declarations are **not** lexical (TopLevelLexicallyDeclaredNames); they are var-like.
fn collect_lexically_declared_names<'a, I>(
    stmts: I,
    top_level: bool,
) -> Vec<(String, Span, LexNameKind)>
where
    I: IntoIterator<Item = &'a Stmt>,
{
    let mut out = Vec::new();
    for stmt in stmts {
        let s = peel_labels(stmt);
        match s {
            Stmt::Let {
                kind:
                    BindingKind::Let | BindingKind::Const | BindingKind::Using | BindingKind::AwaitUsing,
                binding,
                ..
            } => {
                binding.for_each_ident(&mut |id| {
                    out.push((id.name.clone(), id.span, LexNameKind::Other));
                });
            }
            Stmt::ClassDeclaration { name, .. } => {
                out.push((name.name.clone(), name.span, LexNameKind::Other));
            }
            Stmt::FunctionDeclaration {
                name,
                is_async,
                is_generator,
                ..
            } => {
                // Script/FunctionBody: hoistables are TopLevelVarDeclaredNames only.
                if top_level {
                    continue;
                }
                let kind = if *is_async || *is_generator {
                    LexNameKind::Other
                } else {
                    LexNameKind::PlainFunction
                };
                out.push((name.name.clone(), name.span, kind));
            }
            _ => {}
        }
    }
    out
}

/// VarDeclaredNames of a StatementList (walks nested statements; not function/class bodies).
///
/// When `top_level`, direct hoistable function declarations are included
/// (TopLevelVarDeclaredNames).
fn collect_var_declared_names<'a, I>(stmts: I, top_level: bool) -> Vec<(String, Span)>
where
    I: IntoIterator<Item = &'a Stmt>,
{
    let mut out = Vec::new();
    for stmt in stmts {
        if top_level {
            let s = peel_labels(stmt);
            match s {
                Stmt::FunctionDeclaration { name, .. }
                | Stmt::ExternFunctionDeclaration { name, .. } => {
                    out.push((name.name.clone(), name.span));
                }
                _ => {}
            }
        }
        collect_var_declared_names_stmt(stmt, &mut out);
    }
    out
}

fn collect_var_declared_names_stmt(stmt: &Stmt, out: &mut Vec<(String, Span)>) {
    match stmt {
        Stmt::Labeled { body, .. } => collect_var_declared_names_stmt(body, out),
        Stmt::Let {
            kind: BindingKind::Var,
            binding,
            ..
        } => {
            binding.for_each_ident(&mut |id| {
                out.push((id.name.clone(), id.span));
            });
        }
        Stmt::Block { body, .. } => {
            for child in body {
                collect_var_declared_names_stmt(child, out);
            }
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_var_declared_names_stmt(consequent, out);
            if let Some(alt) = alternate {
                collect_var_declared_names_stmt(alt, out);
            }
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } | Stmt::With { body, .. } => {
            collect_var_declared_names_stmt(body, out);
        }
        Stmt::For { init, body, .. } => {
            if let Some(init) = init {
                collect_var_declared_names_stmt(init, out);
            }
            collect_var_declared_names_stmt(body, out);
        }
        Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
            collect_var_declared_names_stmt(left, out);
            collect_var_declared_names_stmt(body, out);
        }
        Stmt::Switch { cases, .. } => {
            for case in cases {
                for child in &case.body {
                    collect_var_declared_names_stmt(child, out);
                }
            }
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            collect_var_declared_names_stmt(block, out);
            if let Some(handler) = handler {
                collect_var_declared_names_stmt(handler, out);
            }
            if let Some(finalizer) = finalizer {
                collect_var_declared_names_stmt(finalizer, out);
            }
        }
        Stmt::FunctionDeclaration { .. } | Stmt::ClassDeclaration { .. } => {}
        _ => {}
    }
}

/// Statement-list early errors (E19.24).
///
/// - LexicallyDeclaredNames must not contain duplicates (Annex B: sloppy plain
///   `function` duplicates only are allowed — block/switch only).
/// - LexicallyDeclaredNames ∩ VarDeclaredNames must be empty.
///
/// `top_level`: Script or FunctionBody (TopLevel*DeclaredNames); otherwise Block/CaseBlock.
fn check_statement_list_early_errors<'a, I>(
    stmts: I,
    strict: bool,
    top_level: bool,
) -> Result<(), Diagnostic>
where
    I: IntoIterator<Item = &'a Stmt> + Clone,
{
    let lexical = collect_lexically_declared_names(stmts.clone(), top_level);
    let mut seen: HashMap<String, LexNameKind> = HashMap::new();
    for (name, span, kind) in &lexical {
        if let Some(prev) = seen.get(name) {
            let allow_sloppy_fn = !strict
                && !top_level
                && *prev == LexNameKind::PlainFunction
                && *kind == LexNameKind::PlainFunction;
            if !allow_sloppy_fn {
                return Err(Diagnostic::new(
                    format!("duplicate declaration of `{name}`"),
                    *span,
                ));
            }
        } else {
            seen.insert(name.clone(), *kind);
        }
    }
    let vars = collect_var_declared_names(stmts, top_level);
    let mut var_names = HashMap::new();
    for (name, span) in vars {
        var_names.entry(name).or_insert(span);
    }
    for (name, span, _) in &lexical {
        if var_names.contains_key(name) {
            return Err(Diagnostic::new(
                format!("duplicate declaration of `{name}`"),
                *span,
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_ast::{ArrayPatternElement, BinaryOp, ClassElement, ObjectKey, ObjectPatternProp};
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

    // E19.60: const PutValue is a runtime TypeError, not a compile reject.
    #[test]
    fn check_const_reassignment_ok() {
        let program = parse("const x = 1; x = 2;").unwrap();
        check(program).expect("const reassignment must typecheck (runtime TypeError)");
    }

    #[test]
    fn check_const_dstr_put_ok() {
        let program = parse("const c = null; [c] = [1];").unwrap();
        check(program).expect("const dstr put must typecheck (runtime TypeError)");
    }

    #[test]
    fn check_const_update_ok() {
        let program = parse("const x = 1; x++;").unwrap();
        check(program).expect("const update must typecheck (runtime TypeError)");
    }

    // E19.60: parenthesized cover IdentifierReference is a valid simple assignment target.
    #[test]
    fn check_parenthesized_assign_target_ok() {
        let program = parse("var x; (x) = 1;").unwrap();
        check(program).expect("(x) = 1 must typecheck");
    }

    #[test]
    fn check_parenthesized_update_target_ok() {
        let program = parse("var y = 1; (y)++; ((y))++;").unwrap();
        check(program).expect("(y)++ must typecheck");
    }

    // E19.60: non-strict eval/arguments are simple assignment targets (not early error).
    #[test]
    fn check_nonstrict_eval_assign_ok() {
        let program = parse("eval = 1;").unwrap();
        check(program).expect("non-strict eval = must typecheck");
    }

    #[test]
    fn check_nonstrict_eval_update_ok() {
        let program = parse("eval++;").unwrap();
        check(program).expect("non-strict eval++ must typecheck");
    }

    // E19.57: named FE / class expr name reassignment is a runtime TypeError (strict)
    // or silent no-op (non-strict FE), not a compile reject.
    #[test]
    fn check_named_function_expression_reassign_ok() {
        let program = parse(
            "let ref = function BindingIdentifier() { BindingIdentifier = 1; return BindingIdentifier; };",
        )
        .unwrap();
        check(program).expect("named FE name reassign must typecheck");
    }

    #[test]
    fn check_named_async_function_expression_reassign_ok() {
        let program = parse(
            "let ref = async function BindingIdentifier() { BindingIdentifier = 1; return BindingIdentifier; };",
        )
        .unwrap();
        check(program).expect("named async FE name reassign must typecheck");
    }

    #[test]
    fn check_named_generator_expression_reassign_ok() {
        let program = parse(
            "let ref = function* BindingIdentifier() { BindingIdentifier = 1; return BindingIdentifier; };",
        )
        .unwrap();
        check(program).expect("named generator FE name reassign must typecheck");
    }

    #[test]
    fn check_named_async_generator_expression_reassign_ok() {
        let program = parse(
            "let ref = async function* BindingIdentifier() { BindingIdentifier = 1; return BindingIdentifier; };",
        )
        .unwrap();
        check(program).expect("named async generator FE name reassign must typecheck");
    }

    #[test]
    fn check_named_class_expression_reassign_ok() {
        let program = parse("let C = class Name { m() { Name = 1; } };").unwrap();
        check(program).expect("named class expression name reassign must typecheck");
    }

    #[test]
    fn check_class_declaration_name_reassign_ok() {
        let program = parse("class C { constructor() { C = 42; } }").unwrap();
        check(program).expect("class declaration name reassign must typecheck (runtime TypeError)");
    }

    #[test]
    fn check_function_declaration_reassign_ok() {
        let program = parse("function f() {} f = 1;").unwrap();
        check(program).expect("function declaration reassign must typecheck");
    }

    // E19.58: OptionalExpression is not a valid AssignmentTarget / update target.
    #[test]
    fn check_optional_chain_assignment_fails() {
        let program = parse("let o = {}; o?.p = 1;").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("invalid assignment target"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_optional_chain_update_fails() {
        let program = parse("let o = {}; o?.p++;").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("invalid update target"),
            "unexpected: {}",
            err.message
        );
    }

    // E19.58 / E19.82.05: Super in arrows only when lexically nested in Super context.
    #[test]
    fn check_arrow_super_call_fails() {
        // E19.67: super outside method is parse-time SyntaxError.
        assert!(
            parse("() => super();").is_err(),
            "top-level arrow SuperCall must fail at parse"
        );
    }

    #[test]
    fn check_async_arrow_super_call_fails() {
        assert!(
            parse("async () => super();").is_err(),
            "top-level async arrow SuperCall must fail at parse"
        );
    }

    #[test]
    fn check_arrow_super_property_outside_method_fails() {
        assert!(
            parse("() => super.x;").is_err(),
            "top-level arrow SuperProperty must fail at parse"
        );
    }

    #[test]
    fn check_async_arrow_super_property_outside_method_fails() {
        assert!(
            parse("async () => super.x;").is_err(),
            "top-level async arrow SuperProperty must fail at parse"
        );
    }

    #[test]
    fn check_arrow_super_property_in_method_ok() {
        let program =
            parse("class B {} class C extends B { m() { return () => super.x; } }").unwrap();
        check(program).expect("arrow SuperProperty in method must typecheck");
    }

    #[test]
    fn check_arrow_super_call_in_derived_ctor_ok() {
        // E19.82.05: SuperCall in arrow nested in derived constructor is valid.
        let program =
            parse("class B {} class C extends B { constructor() { let f = () => super(); f(); } }")
                .unwrap();
        check(program).expect("arrow SuperCall in derived ctor must typecheck");
    }

    #[test]
    fn check_arrow_super_property_in_field_ok() {
        // E19.82.05: SuperProperty in field initializer arrows is valid.
        let program = parse("class C { f = () => { super.x = 1; }; }").unwrap();
        check(program).expect("arrow SuperProperty in field init must typecheck");
    }

    #[test]
    fn check_compound_assignment_to_property_ok() {
        let program = parse("let o = { a: 1 }; o.a += 2; o[\"a\"] *= 3;").unwrap();
        check(program).expect("compound assignment to property should typecheck");
    }

    #[test]
    fn check_compound_assignment_to_computed_property_ok() {
        let program = parse("let o = {}; let k = \"x\"; o[k] = 1; o[k] += 2;").unwrap();
        check(program).expect("compound assignment to computed property should typecheck");
    }

    // E19.12: untyped compound assignment — ToNumber widen; do not reject assign-back.
    #[test]
    fn check_untyped_compound_assignment_boolean() {
        let program = parse("let x = true; x += 1; x *= false;").unwrap();
        check(program).expect("boolean compound assign should typecheck");
    }

    #[test]
    fn check_untyped_compound_assignment_string_numeric() {
        let program = parse(r#"let x = "2"; x *= 3; x -= "1";"#).unwrap();
        check(program).expect("string numeric compound assign should typecheck");
    }

    #[test]
    fn check_untyped_compound_assignment_null() {
        let program = parse("let x = null; x -= 1; x += true;").unwrap();
        check(program).expect("null compound assign should typecheck");
    }

    #[test]
    fn check_untyped_compound_assignment_add_string_concat() {
        let program = parse(r#"let x = 1; x += "a";"#).unwrap();
        check(program).expect("number += string should typecheck (ToString concat)");
    }

    #[test]
    fn check_untyped_compound_assignment_uninitialized_any() {
        let program = parse("let x; x += 1; x *= true;").unwrap();
        check(program).expect("any compound assign should typecheck");
    }

    #[test]
    fn check_untyped_compound_assignment_property_coerced() {
        let program = parse(r#"let o = { a: true }; o.a += 1; o["a"] *= "2";"#).unwrap();
        check(program).expect("property compound assign with coercion should typecheck");
    }

    // E19.48: untyped simple assign residual — after compound widens to number,
    // re-assign null/object/string/boolean must not reject (ECMA-262).
    #[test]
    fn check_untyped_simple_assign_after_number_null() {
        let program = parse(
            r#"
            var x;
            x = null;
            x ^= undefined;
            x = undefined;
            x ^= null;
            x = null;
            x ^= null;
            "#,
        )
        .unwrap();
        check(program).expect("null/undefined simple assign after number should typecheck");
    }

    #[test]
    fn check_untyped_simple_assign_object_string_boolean() {
        let program = parse(
            r#"
            var x;
            x = true;
            x ^= "1";
            x = "1";
            x ^= true;
            x = new Boolean(true);
            x ^= "1";
            x = new String("1");
            x ^= true;
            x = {};
            x = null;
            x = 1;
            "#,
        )
        .unwrap();
        check(program).expect("object/string/boolean simple assign residual should typecheck");
    }

    #[test]
    fn check_untyped_simple_assign_let_number_to_string() {
        let program = parse(r#"let x = 1; x = "a"; x = null; x = {}; x = true;"#).unwrap();
        check(program).expect("inferred number binding accepts JS values without annotation");
    }

    #[test]
    fn check_annotated_number_rejects_string_assign() {
        let program = parse(r#"let x: number = 1; x = "a";"#).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("cannot assign") || err.message.contains("not assignable"),
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
        let program =
            parse("let t = typeof Promise; let p = new Promise(function (r) { r(1); });").unwrap();
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
        let program =
            parse("let t = function (a) { return a; }; let p = new Proxy(t, {}); let r = p(1);")
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
    fn check_proxy_of_object_call_typechecks() {
        // E19.13: Object/Proxy callability is a runtime [[Call]] check, not compile reject.
        let program = parse("let p = new Proxy({}, {}); try { p(); } catch (e) {}").unwrap();
        check(program).expect("calling Proxy of object should typecheck");
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
        let program =
            parse("let a = typeof Error; let b = typeof TypeError; let c = typeof AggregateError;")
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
        let program =
            parse("let t = typeof JSON; let p = JSON.parse; let s = JSON.stringify;").unwrap();
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
        let program =
            parse("ArrayBuffer; DataView; Uint8Array; Int32Array; Float64Array;").unwrap();
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
        for name in [
            "ArrayBuffer",
            "DataView",
            "Uint8Array",
            "Int32Array",
            "Float64Array",
        ] {
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
        let program =
            parse("let a = eval(\"1 + 2\"); let b = eval(\"typeof undefined\");").unwrap();
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
    fn bind_free_identifier_ok_global_object_ref() {
        // E19.05: free idents are runtime global/unresolvable refs, not bind errors.
        let program = parse("y;").unwrap();
        let bound = bind(program).expect("free ident binds");
        let y_span = find_ident_use(&bound.program, "y");
        assert!(
            bound.resolve(y_span).is_none(),
            "free y must stay unresolved for IdentName emit"
        );
    }

    #[test]
    fn bind_free_assign_and_typeof_ok() {
        let src = "x = 1; typeof z;";
        let bound = bind(parse(src).unwrap()).expect("free assign/typeof bind");
        check(parse(src).unwrap()).expect("free assign/typeof check");
        let x_span = find_ident_use(&bound.program, "x");
        let z_span = find_ident_use(&bound.program, "z");
        assert!(bound.resolve(x_span).is_none());
        assert!(bound.resolve(z_span).is_none());
    }

    // E17.02.09: for-in/of left free IdentifierReference is runtime PutValue, not check error.
    #[test]
    fn check_free_for_in_of_left_ok() {
        let src = "for (k in {a: 1}) {} for (v of [2]) {}";
        let bound = bind(parse(src).unwrap()).expect("free for-in/of left binds");
        check(parse(src).unwrap()).expect("free for-in/of left checks");
        let k_span = find_ident_use(&bound.program, "k");
        let v_span = find_ident_use(&bound.program, "v");
        assert!(bound.resolve(k_span).is_none());
        assert!(bound.resolve(v_span).is_none());
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

    // E19.24: early SyntaxError for strict arrow eval/arguments + block/switch redeclarations.
    #[test]
    fn bind_strict_arrow_eval_param_errors() {
        let program = parse("\"use strict\"; let af = eval => 1;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("eval") && err.message.contains("strict"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_strict_arrow_arguments_param_errors() {
        let program = parse("\"use strict\"; let af = (arguments) => 1;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("arguments") && err.message.contains("strict"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_sloppy_arrow_eval_param_ok() {
        let program = parse("let af = eval => eval;").unwrap();
        bind(program).expect("sloppy arrow may bind eval");
    }

    // E19.49: strict eval/arguments bindings + assign targets.
    #[test]
    fn bind_e19_49_strict_var_eval_errors() {
        let program = parse("\"use strict\"; var eval;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("eval") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
        let program = parse("\"use strict\"; var arguments;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("arguments") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_e19_49_strict_catch_eval_errors() {
        let program = parse("\"use strict\"; try {} catch (eval) {}").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("eval") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_e19_49_strict_assign_eval_errors() {
        let program = parse("\"use strict\"; eval = 1;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("eval") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
        let program = parse("\"use strict\"; arguments += 1;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("arguments") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
        let program = parse("\"use strict\"; ++arguments;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("arguments") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
        let program = parse("\"use strict\"; (eval) = 1;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("eval") && err.message.contains("strict"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_e19_49_sloppy_eval_assign_ok() {
        bind(parse("eval = 1;").unwrap()).expect("sloppy eval assign");
        bind(parse("var eval;").unwrap()).expect("sloppy var eval");
    }

    // E19.39: early SyntaxError residuals.
    #[test]
    fn bind_use_strict_non_simple_params_errors() {
        let program = parse("function f(a = 0) { \"use strict\"; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("use strict") || err.message.contains("non-simple"),
            "unexpected: {}",
            err.message
        );
        let program = parse("({ m(a = 0) { \"use strict\"; } });").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("use strict") || err.message.contains("non-simple"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_strict_delete_identifier_errors() {
        let program = parse("\"use strict\"; delete x;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("delete"),
            "unexpected: {}",
            err.message
        );
        let program = parse("\"use strict\"; delete ((x));").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("delete"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_method_param_redecl_errors() {
        let program = parse("({ method(param) { let param; } });").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") || err.message.contains("param"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn bind_object_method_super_call_errors() {
        let program = parse("({ m() { super(); } });").unwrap();
        let err = bind(program).unwrap_err();
        assert!(err.message.contains("super"), "unexpected: {}", err.message);
    }

    // E17.02.04: duplicate formals allowed only for non-strict simple plain `function`.
    #[test]
    fn bind_sloppy_duplicate_params_ok() {
        let program = parse("function f(a, a) { return a; }").unwrap();
        bind(program).expect("sloppy simple duplicate formals");
    }

    #[test]
    fn bind_sloppy_duplicate_params_function_expr_ok() {
        let program = parse("let f = function (a, b, a) { return a; };").unwrap();
        bind(program).expect("sloppy FE simple duplicate formals");
    }

    #[test]
    fn bind_strict_duplicate_params_errors() {
        let program = parse("function f(a, a) { \"use strict\"; return a; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_params_with_default_errors() {
        let program = parse("function f(a, a = 1) { return a; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_params_arrow_errors() {
        let program = parse("let f = (a, a) => a;").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_params_method_errors() {
        let program = parse("let o = { m(a, a) { return a; } };").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_params_async_errors() {
        let program = parse("async function f(a, a) { return a; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_duplicate_params_generator_errors() {
        let program = parse("function* f(a, a) { yield a; }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("a"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_block_function_let_redeclaration_errors() {
        let program = parse("{ function f() {} let f }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("f"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_block_var_let_redeclaration_errors() {
        let program = parse("{ var f; let f }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("f"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_switch_var_let_redeclaration_errors() {
        let program = parse("switch (0) { case 1: var f; default: let f }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("f"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_inner_block_var_outer_let_redeclaration_errors() {
        let program = parse("{ let f; { var f; } }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("f"),
            "unexpected message: {}",
            err.message
        );
    }

    #[test]
    fn bind_sloppy_block_duplicate_function_ok() {
        let program = parse("{ function f() {} function f() {} }").unwrap();
        bind(program).expect("Annex B allows sloppy duplicate plain functions");
    }

    #[test]
    fn bind_strict_block_duplicate_function_errors() {
        let program = parse("\"use strict\"; { function f() {} function f() {} }").unwrap();
        let err = bind(program).unwrap_err();
        assert!(
            err.message.contains("duplicate") && err.message.contains("f"),
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
    fn bind_arguments_free_in_arrow_at_top_level() {
        // Top-level arrow has no `arguments` binding; name stays free (runtime
        // ReferenceError on GetValue / typeof → "undefined").
        let program = parse("let f = () => arguments.length;").unwrap();
        bind(program).expect("free arguments in arrow binds");
        check(parse("let f = () => arguments.length;").unwrap()).expect("check free arguments");
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
        let program =
            parse(r#"let a = +"42"; let b = +true; let c = +null; let d = +"";"#).unwrap();
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
        let program =
            parse(r#"let a = 1 == "1"; let b = null == 0; let c = true != "1";"#).unwrap();
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

    // E19.04: untyped JS operator applicability — ECMA-262 ToNumber/ToPrimitive, not TS-strict.
    #[test]
    fn check_arithmetic_on_string_coerces() {
        let program = parse(r#"let x = "a" - 1; let y = "2" * 3; let z = "8" / "2";"#).unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "x"), Type::Number);
        assert_eq!(sym_type(&checked, "y"), Type::Number);
        assert_eq!(sym_type(&checked, "z"), Type::Number);
    }

    #[test]
    fn check_unary_minus_on_string_coerces() {
        let program = parse(r#"let x = -"a"; let y = ~"1"; let z = -true;"#).unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "x"), Type::Number);
        assert_eq!(sym_type(&checked, "y"), Type::Number);
        assert_eq!(sym_type(&checked, "z"), Type::Number);
    }

    #[test]
    fn check_relational_mixed_primitives() {
        let program =
            parse(r#"let a = "2" < 10; let b = true > 0; let c = null <= 1; let d = "a" < "b";"#)
                .unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Boolean);
        assert_eq!(sym_type(&checked, "b"), Type::Boolean);
        assert_eq!(sym_type(&checked, "c"), Type::Boolean);
        assert_eq!(sym_type(&checked, "d"), Type::Boolean);
    }

    #[test]
    fn check_arithmetic_object_to_primitive() {
        let program = parse(
            r#"
            let o = { valueOf: function () { return 3; } };
            let a = o - 1;
            let b = o * 2;
            let c = o < 10;
            "#,
        )
        .unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Number);
        assert_eq!(sym_type(&checked, "b"), Type::Number);
        assert_eq!(sym_type(&checked, "c"), Type::Boolean);
    }

    // E19.07: mixed BigInt×Number/object/any is ECMA-262-valid; TypeError is runtime.
    #[test]
    fn check_arithmetic_allows_bigint_mixed() {
        let program = parse(
            r#"
            let a = 1n - 1;
            let b = 1 + 1n;
            let c = 1n * true;
            let d = null / 1n;
            let e = 1n + "x";
            let o = { valueOf: function () { return 1n; } };
            let f = o + 1n;
            let g = 1n & 1;
            let h = 1n >>> 1;
            "#,
        )
        .unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::Any);
        assert_eq!(sym_type(&checked, "b"), Type::Any);
        assert_eq!(sym_type(&checked, "c"), Type::Any);
        assert_eq!(sym_type(&checked, "d"), Type::Any);
        assert_eq!(sym_type(&checked, "e"), Type::String);
        assert_eq!(sym_type(&checked, "f"), Type::Any);
        assert_eq!(sym_type(&checked, "g"), Type::Any);
        assert_eq!(sym_type(&checked, "h"), Type::Any);
    }

    #[test]
    fn check_arithmetic_same_type_bigint_still_bigint() {
        let program = parse("let a = 1n + 2n; let b = 3n * 4n; let c = 5n << 1n;").unwrap();
        let checked = check(program).unwrap();
        assert_eq!(sym_type(&checked, "a"), Type::BigInt);
        assert_eq!(sym_type(&checked, "b"), Type::BigInt);
        assert_eq!(sym_type(&checked, "c"), Type::BigInt);
    }

    // E19.59: call/`new` on boolean/number/string/null — TypeError is runtime, not compile.
    #[test]
    fn check_call_on_primitives_typechecks() {
        let program = parse(
            r#"
            let n = 1; try { n(); } catch (e) {}
            let b = true; try { b(); } catch (e) {}
            let s = "x"; try { s(); } catch (e) {}
            let z = null; try { z(); } catch (e) {}
            try { (1)(); } catch (e) {}
            try { (true)(); } catch (e) {}
            try { ("x")(); } catch (e) {}
            try { (null)(); } catch (e) {}
            "#,
        )
        .unwrap();
        check(program).expect("call on primitives should typecheck; [[Call]] is runtime");
    }

    #[test]
    fn check_new_on_primitives_typechecks() {
        let program = parse(
            r#"
            let n = 1; try { new n(); } catch (e) {}
            let b = true; try { new b(); } catch (e) {}
            let s = "x"; try { new s(); } catch (e) {}
            let z = null; try { new z(); } catch (e) {}
            try { new (1)(); } catch (e) {}
            try { new (true)(); } catch (e) {}
            try { new ("x")(); } catch (e) {}
            try { new (null)(); } catch (e) {}
            "#,
        )
        .unwrap();
        check(program).expect("new on primitives should typecheck; [[Construct]] is runtime");
    }

    // E19.13: ++/-- and call on ToPrimitive / object values — runtime ToNumber/[[Call]], not compile reject.
    #[test]
    fn check_update_on_object_to_primitive() {
        let program = parse(
            r#"
            let o = { valueOf: function () { return 1; } };
            o++;
            ++o;
            let f = function () { return 1; };
            f++;
            "#,
        )
        .unwrap();
        check(program).expect("update on object/function should typecheck (ToNumber)");
    }

    #[test]
    fn check_update_on_member_to_primitive() {
        let program = parse(
            r#"
            let o = { x: 1, y: true };
            o.x++;
            ++o["y"];
            let a = [0];
            a[0]++;
            "#,
        )
        .unwrap();
        check(program).expect("update on property should typecheck");
    }

    #[test]
    fn check_call_on_object_typechecks() {
        let program = parse(
            r#"
            let o = {};
            try { o(); } catch (e) {}
            try { Math(); } catch (e) {}
            try { new Boolean(true)(); } catch (e) {}
            let b = new Boolean(true);
            try { b(); } catch (e) {}
            "#,
        )
        .unwrap();
        check(program).expect("calling object should typecheck; [[Call]] is runtime");
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
        fn walk_object_key(key: &ObjectKey, name: &str, out: &mut Option<Span>) {
            if let ObjectKey::Computed(expr) = key {
                walk_expr(expr, name, out);
            }
        }
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
                | Expr::Super { .. }
                | Expr::NewTarget { .. }
                | Expr::ImportMeta { .. } => {}
                Expr::ImportCall {
                    source, options, ..
                } => {
                    walk_expr(source, name, out);
                    if let Some(opts) = options {
                        walk_expr(opts, name, out);
                    }
                }
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
                | Expr::As { expr: arg, .. } => walk_expr(arg, name, out),
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
                            ArrayElement::Elision => {}
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
                Expr::PrivateIn { object, .. } => walk_expr(object, name, out),
                // Function/class bodies walked via declaration paths when needed.
                Expr::FunctionExpression { .. }
                | Expr::ClassExpression { .. }
                | Expr::ArrowFunction { .. } => {}
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
                            ArrayPatternElement::Elision => {}
                            ArrayPatternElement::Pattern {
                                binding: BindingPattern::Member(expr),
                                default,
                            } => {
                                walk_expr(expr, name, out);
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ArrayPatternElement::Rest(BindingPattern::Ident(id))
                                if id.name == name =>
                            {
                                *out = Some(id.span);
                            }
                            ArrayPatternElement::Rest(BindingPattern::Array {
                                elements, ..
                            }) => {
                                walk_expr(
                                    &Expr::ArrayPattern {
                                        elements: elements.clone(),
                                        span: Span::dummy(),
                                    },
                                    name,
                                    out,
                                );
                            }
                            ArrayPatternElement::Rest(BindingPattern::Object {
                                properties,
                                ..
                            }) => {
                                walk_expr(
                                    &Expr::ObjectPattern {
                                        properties: properties.clone(),
                                        span: Span::dummy(),
                                    },
                                    name,
                                    out,
                                );
                            }
                            ArrayPatternElement::Rest(BindingPattern::Member(expr)) => {
                                walk_expr(expr, name, out);
                            }
                            ArrayPatternElement::Rest(_) => {}
                        }
                    }
                }
                Expr::ObjectPattern { properties, .. } => {
                    for p in properties {
                        match p {
                            ObjectPatternProp::Prop {
                                key,
                                binding: BindingPattern::Ident(id),
                                default,
                                ..
                            } if id.name == name => {
                                *out = Some(id.span);
                                if let ObjectKey::Computed(e) = key {
                                    walk_expr(e, name, out);
                                }
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ObjectPatternProp::Prop {
                                key,
                                binding: BindingPattern::Ident(_),
                                default,
                                ..
                            } => {
                                if let ObjectKey::Computed(e) = key {
                                    walk_expr(e, name, out);
                                }
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ObjectPatternProp::Prop {
                                key,
                                binding: BindingPattern::Array { elements, .. },
                                default,
                                ..
                            } => {
                                if let ObjectKey::Computed(e) = key {
                                    walk_expr(e, name, out);
                                }
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
                                key,
                                binding:
                                    BindingPattern::Object {
                                        properties: nested, ..
                                    },
                                default,
                                ..
                            } => {
                                if let ObjectKey::Computed(e) = key {
                                    walk_expr(e, name, out);
                                }
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
                            ObjectPatternProp::Prop {
                                key,
                                binding: BindingPattern::Member(expr),
                                default,
                                ..
                            } => {
                                if let ObjectKey::Computed(e) = key {
                                    walk_expr(e, name, out);
                                }
                                walk_expr(expr, name, out);
                                if let Some(def) = default {
                                    walk_expr(def, name, out);
                                }
                            }
                            ObjectPatternProp::Rest(BindingPattern::Ident(id))
                                if id.name == name =>
                            {
                                *out = Some(id.span);
                            }
                            ObjectPatternProp::Rest(BindingPattern::Array { elements, .. }) => {
                                walk_expr(
                                    &Expr::ArrayPattern {
                                        elements: elements.clone(),
                                        span: Span::dummy(),
                                    },
                                    name,
                                    out,
                                );
                            }
                            ObjectPatternProp::Rest(BindingPattern::Object {
                                properties: nested,
                                ..
                            }) => {
                                walk_expr(
                                    &Expr::ObjectPattern {
                                        properties: nested.clone(),
                                        span: Span::dummy(),
                                    },
                                    name,
                                    out,
                                );
                            }
                            ObjectPatternProp::Rest(BindingPattern::Member(expr)) => {
                                walk_expr(expr, name, out);
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
                | Stmt::TypeAlias { .. }
                | Stmt::ExternFunctionDeclaration { .. } => {}
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
                Stmt::FunctionDeclaration { params, body, .. } => {
                    let _ = params;
                    walk_stmt(body, name, out);
                }
                Stmt::ClassDeclaration {
                    super_class, body, ..
                } => {
                    if let Some(sc) = super_class {
                        walk_expr(sc, name, out);
                    }
                    for el in body {
                        match el {
                            ClassElement::Constructor { body, .. }
                            | ClassElement::StaticBlock { body, .. } => {
                                walk_stmt(body, name, out);
                            }
                            ClassElement::Method { key, body, .. }
                            | ClassElement::Accessor { key, body, .. } => {
                                walk_object_key(key, name, out);
                                walk_stmt(body, name, out);
                            }
                            ClassElement::Field { key, value, .. } => {
                                walk_object_key(key, name, out);
                                if let Some(v) = value {
                                    walk_expr(v, name, out);
                                }
                            }
                        }
                    }
                }
                Stmt::Return {
                    argument: Some(arg),
                    ..
                } => walk_expr(arg, name, out),
                Stmt::Return { argument: None, .. } => {}
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
                | Stmt::ExportDefaultDeclaration { .. }
                | Stmt::ExportAllDeclaration { .. } => {}
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

    // --- T07.02: missing return in annotated non-void function ---

    #[test]
    fn check_missing_return_errors() {
        let program = parse("function f(): number { let x = 1; }").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_missing_return_empty_body_errors() {
        let program = parse("function f(): string {}").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_missing_return_if_without_else_errors() {
        let program = parse("function f(x: boolean): number { if (x) { return 1; } }").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_missing_return_shape_errors() {
        let program = parse("function f(): { x: number } {}").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_function_expression_missing_return_errors() {
        let program = parse("let f = function (): number { let x = 1; };").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_arrow_block_missing_return_errors() {
        let program = parse("let f = (): number => { let x = 1; };").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_return_ends_function_ok() {
        let program = parse("function f(): number { return 1; }").unwrap();
        check(program).expect("trailing return should typecheck");
    }

    #[test]
    fn check_return_in_condition_then_tail_ok() {
        let program =
            parse("function f(x: boolean): number { if (x) { return 1; } return 2; }").unwrap();
        check(program).expect("return in if plus trailing return should typecheck");
    }

    #[test]
    fn check_both_if_branches_return_ok() {
        let program =
            parse("function f(x: boolean): number { if (x) { return 1; } else { return 2; } }")
                .unwrap();
        check(program).expect("both if branches returning should typecheck");
    }

    #[test]
    fn check_infinite_loop_ok() {
        let program = parse("function f(): number { while (true) { let x = 1; } }").unwrap();
        check(program).expect("infinite loop should satisfy return type");
    }

    #[test]
    fn check_infinite_loop_with_inner_break_shadowed_ok() {
        let program =
            parse("function f(): number { while (true) { for (;;) { break; } } }").unwrap();
        check(program).expect("inner break still inside nested loop should satisfy return type");
    }

    #[test]
    fn check_infinite_loop_with_escaping_break_errors() {
        let program = parse("function f(): number { while (true) { break; } }").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_throw_only_ok() {
        let program = parse(r#"function f(): number { throw new Error("x"); }"#).unwrap();
        check(program).expect("throw-only body should satisfy return type");
    }

    #[test]
    fn check_any_return_fall_through_ok() {
        let program = parse("function f(): any { let x = 1; }").unwrap();
        check(program).expect("`any` return type should allow fall-off-end");
    }

    #[test]
    fn check_unannotated_function_fall_through_ok() {
        let program = parse("function f() { let x = 1; }").unwrap();
        check(program).expect("unannotated function should allow fall-off-end");
    }

    #[test]
    fn check_switch_all_cases_return_ok() {
        let program = parse(
            r#"
            function f(x: number): number {
              switch (x) {
                case 1: return 1;
                default: return 0;
              }
            }
            "#,
        )
        .unwrap();
        check(program).expect("switch with all cases returning should typecheck");
    }

    #[test]
    fn check_switch_missing_default_errors() {
        let program = parse(
            r#"
            function f(x: number): number {
              switch (x) {
                case 1: return 1;
                case 2: return 2;
              }
            }
            "#,
        )
        .unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("missing return"),
            "unexpected: {}",
            err.message
        );
    }

    // --- T07.04: call/`new` of an annotated non-callable value ---

    #[test]
    fn check_annotated_number_call_errors() {
        let program = parse("let x: number = 1; x();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("number"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_string_call_errors() {
        let program = parse(r#"let s: string = "a"; s();"#).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("string"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_boolean_call_errors() {
        let program = parse("let b: boolean = true; b();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("boolean"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_bigint_call_errors() {
        let program = parse("let x: bigint = 1n; x();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("bigint"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_shape_call_errors() {
        let program = parse("let p: { x: number } = { x: 1 }; p();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("{ x: number }"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_param_call_errors() {
        let program = parse("function g(x: number) { x(); }").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("number"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_alias_call_errors() {
        let program = parse("type Num = number; let x: Num = 1; x();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable") && err.message.contains("number"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_number_new_errors() {
        let program = parse("let x: number = 1; new x();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not constructable") && err.message.contains("number"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_shape_new_errors() {
        let program = parse("let p: { x: number } = { x: 1 }; new p();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not constructable") && err.message.contains("{ x: number }"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_parenthesized_annotated_call_errors() {
        let program = parse("let x: number = 1; (x)();").unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("not callable"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_annotated_any_call_ok() {
        let program = parse("let x: any = 1; x();").unwrap();
        check(program).expect("`any` stays permissive when called");
    }

    #[test]
    fn check_annotated_callable_declared_fn_ok() {
        let program =
            parse("function g(a: number): number { return a * 2; } let m: number = g(21);")
                .unwrap();
        check(program).expect("annotated declared function is callable");
    }

    #[test]
    fn check_untyped_non_callable_call_ok() {
        let program = parse("let x = 1; x(); let p = { a: 1 }; p();").unwrap();
        check(program).expect("untyped JS stays permissive when calling non-callables");
    }

    #[test]
    fn check_inferred_shape_call_ok() {
        let program = parse("let p = { a: 1 }; p();").unwrap();
        check(program).expect("inferred object-literal shape stays permissive when called");
    }

    // --- F06.02: extern "C" function signature checking ---

    #[test]
    fn bind_extern_function_declares_symbol() {
        let program = parse(r#"extern "C" function add(a: i32, b: i32): i32;"#).unwrap();
        let bound = bind(program).unwrap();
        let add = user_symbol(&bound, "add");
        assert_eq!(add.kind, BindingKind::Function);
    }

    #[test]
    fn check_extern_native_sig_ok() {
        let program = parse(
            r#"
            extern "C" function add(a: i32, b: i32): i32;
            extern "C" function puts(s: *u8): i32;
            extern "C" function free(p: *u8): void;
            extern "C" function quit();
            "#,
        )
        .unwrap();
        let checked = check(program).expect("valid extern signatures must typecheck");
        let add = user_symbol(&checked.bound, "add");
        assert_eq!(checked.type_of_symbol(add.id), Type::Function);
    }

    #[test]
    fn check_extern_string_param_errors() {
        let src = r#"extern "C" function f(s: string): i32;"#;
        let program = parse(src).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("extern parameter") && err.message.contains("string"),
            "unexpected: {}",
            err.message
        );
        assert_eq!(err.code, Some(codes::INVALID_EXTERN_TYPE));
        assert!(
            !err.span.is_dummy(),
            "F08.02: span must point at the bad type"
        );
        let lo = err.span.start.0 as usize;
        let hi = err.span.end.0 as usize;
        assert_eq!(
            &src[lo..hi],
            "string",
            "span should cover the unsupported type"
        );
    }

    #[test]
    fn check_extern_number_param_errors() {
        let program = parse(r#"extern "C" function f(n: number): void;"#).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("extern parameter") && err.message.contains("number"),
            "unexpected: {}",
            err.message
        );
        assert_eq!(err.code, Some(codes::INVALID_EXTERN_TYPE));
    }

    #[test]
    fn check_extern_any_param_errors() {
        let program = parse(r#"extern "C" function f(x: any): void;"#).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("extern parameter") && err.message.contains("any"),
            "unexpected: {}",
            err.message
        );
        assert_eq!(err.code, Some(codes::INVALID_EXTERN_TYPE));
    }

    #[test]
    fn check_extern_native_layout_param_ok() {
        let program = parse(
            r#"
            type Pair = { a: i32; b: i64 };
            extern "C" function take(p: Pair): i32;
            extern "C" function make(a: i32, b: i64): Pair;
            "#,
        )
        .unwrap();
        check(program).expect("native layout struct is a valid extern ABI type (F03.02)");
    }

    #[test]
    fn check_extern_js_shape_param_errors() {
        let program = parse(r#"extern "C" function f(o: { x: string }): void;"#).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("extern parameter"),
            "unexpected: {}",
            err.message
        );
        assert_eq!(err.code, Some(codes::INVALID_EXTERN_TYPE));
    }

    #[test]
    fn check_extern_unannotated_param_errors() {
        let program = parse(r#"extern "C" function f(x): void;"#).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("must have a type annotation"),
            "unexpected: {}",
            err.message
        );
        assert_eq!(err.code, Some(codes::INVALID_EXTERN_TYPE));
    }

    #[test]
    fn check_extern_void_param_errors() {
        let program = parse(r#"extern "C" function f(x: void): void;"#).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("cannot be `void`"),
            "unexpected: {}",
            err.message
        );
        assert_eq!(err.code, Some(codes::INVALID_EXTERN_TYPE));
    }

    #[test]
    fn check_extern_string_return_errors() {
        let src = r#"extern "C" function f(): string;"#;
        let program = parse(src).unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("extern return") && err.message.contains("string"),
            "unexpected: {}",
            err.message
        );
        assert_eq!(err.code, Some(codes::INVALID_EXTERN_TYPE));
        assert!(
            !err.span.is_dummy(),
            "F08.02: span must point at the bad type"
        );
        let lo = err.span.start.0 as usize;
        let hi = err.span.end.0 as usize;
        assert_eq!(
            &src[lo..hi],
            "string",
            "span should cover the unsupported type"
        );
    }

    #[test]
    fn check_extern_call_arity_checked() {
        let program = parse(
            r#"
            extern "C" function add(a: i32, b: i32): i32;
            add(1);
            "#,
        )
        .unwrap();
        let err = check(program).unwrap_err();
        assert!(
            err.message.contains("expected at least 2") || err.message.contains("argument"),
            "unexpected: {}",
            err.message
        );
    }

    #[test]
    fn check_extern_ptr_arg_and_null() {
        let program = parse(
            r#"
            extern "C" function load(p: *i32): i32;
            let x: i32 = 42;
            let p: *i32 = &x;
            let a: i32 = load(p);
            let b: i32 = load(&x);
            let n: *i32 = null;
            let c: i32 = load(n);
            let d: i32 = load(null);
            "#,
        )
        .unwrap();
        check(program).expect("pointer args and null must typecheck for extern *T");
    }

    // --- F08.01: extern / FFI hard-error on js target ---

    #[test]
    fn check_for_target_js_rejects_extern() {
        let program = parse(r#"extern "C" function add(a: i32, b: i32): i32;"#).unwrap();
        let err = check_for_target(program, CompileTarget::Js).expect_err("js hard diagnostic");
        assert_eq!(err.code, Some(codes::EXTERN_UNSUPPORTED));
        assert!(
            err.message.contains("extern")
                && err.message.contains("unsupported on js")
                && err.message.contains("native-only"),
            "got {}",
            err.message
        );
    }

    #[test]
    fn check_for_target_native_allows_extern_sig() {
        let program = parse(r#"extern "C" function add(a: i32, b: i32): i32;"#).unwrap();
        check_for_target(program, CompileTarget::Native)
            .expect("native allows valid extern signatures");
    }

    // --- F02.01: Draconic fn as C function pointer (extern `function` param) ---

    #[test]
    fn check_extern_function_param_ok() {
        let program = parse(
            r#"
            function twice(x: i32): i32 {
              return x + x;
            }
            extern "C" function draconic_rt_fnptr_nonnull(cb: function): i32;
            let ok: i32 = draconic_rt_fnptr_nonnull(twice);
            "#,
        )
        .unwrap();
        check(program).expect("native-ABI fn must pass as extern function-pointer param");
    }

    // --- F03.01: address of native layout is `*u8` for C ABI offset checks ---

    #[test]
    fn check_address_of_native_layout_ok() {
        let program = parse(
            r#"
            type Pair = { a: i32; b: i64 };
            extern "C" function draconic_rt_layout_i32_i64_a(p: *u8): i32;
            let p: Pair = { a: 10, b: 20 };
            let ra: i32 = draconic_rt_layout_i32_i64_a(&p);
            "#,
        )
        .unwrap();
        check(program).expect("address of native layout struct must typecheck as *u8");
    }
}
