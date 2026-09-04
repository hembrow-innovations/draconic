//! Binder (scopes + symbol resolution) and Checker (TypeScript-inspired).
//! Binder: ROADMAP B04. Checker: ROADMAP B05. Host API registry: H00.01.

mod host_api;

pub use host_api::{
    host_apis, is_available as host_api_is_available, is_host_api, lookup as lookup_host_api,
    unsupported_diagnostic as host_api_unsupported_diagnostic, CompileTarget, HostApiEntry,
    HostAvailability,
};

use draconic_ast::{
    Arg, ArrayElement, ArrayPatternElement, ArrowBody, BinaryOp, BindingKind, BindingPattern,
    ClassElement, Expr, ObjectKey, ObjectPatternProp, ObjectProp, Param, Program, Stmt, TypeAnn,
    UnaryOp,
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

/// Resolved signature for a non-generic annotated function (T07.01).
/// Recorded for call-site argument checking: arity and per-param assignability.
/// Only built when the function has at least one annotated parameter, so
/// untyped (E19-era) JS stays fully permissive.
#[derive(Debug, Clone)]
struct FnSig {
    /// Resolved type per parameter; `None` = unannotated (permissive).
    param_types: Vec<Option<Type>>,
    /// Number of annotated parameters without a default or rest that must be supplied.
    required: usize,
    /// Whether the final parameter is a rest param (extra args allowed).
    has_rest: bool,
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
    let mut binder = Binder::new();
    binder.bind_program(program, false)
}

/// Bind under Module goal (E19.67): top-level functions are lexical, not var-like.
pub fn bind_module(program: Program) -> Result<BoundProgram, Diagnostic> {
    let mut binder = Binder::new();
    binder.strict = true;
    binder.bind_program(program, true)
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
    let bound = if module_goal {
        bind_module(program)?
    } else {
        bind(program)?
    };
    let mut checker = Checker::new(&bound);
    // Module evaluation may be async when the body uses top-level await.
    checker.in_async = module_goal;
    checker.host_target = target;
    checker.check_program()?;
    let symbol_types = checker.symbol_types;
    let expr_types = checker.expr_types;
    let shapes = checker.shapes;
    let unions = checker.unions;
    let intersections = checker.intersections;
    let generic_fns = checker.generic_fns;
    let type_aliases = checker.type_aliases;
    Ok(CheckedProgram {
        bound,
        symbol_types,
        expr_types,
        shapes,
        unions,
        intersections,
        generic_fns,
        type_aliases,
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
    /// Current strict-mode code (directive prologue / nested function body).
    strict: bool,
    /// SuperProperty allowed (class/object method or constructor). Arrows inherit; plain
    /// functions clear it. SuperCall is never allowed in arrows (E19.58).
    super_allowed: bool,
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
            strict: false,
            super_allowed: false,
        };
        binder.install_builtin("Math", BindingKind::Const);
        binder.install_builtin("Number", BindingKind::Const);
        binder.install_builtin("NaN", BindingKind::Const);
        binder.install_builtin("Infinity", BindingKind::Const);
        binder.install_builtin("Symbol", BindingKind::Const);
        binder.install_builtin("Promise", BindingKind::Const);
        binder.install_builtin("Proxy", BindingKind::Const);
        binder.install_builtin("Reflect", BindingKind::Const);
        // E19.73: ShadowRealm constructor (host Node --harmony-shadow-realm)
        binder.install_builtin("ShadowRealm", BindingKind::Const);
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
        // L01.01: UTF-8 TextEncoder / TextDecoder (WHATWG Encoding; portable)
        binder.install_builtin("TextEncoder", BindingKind::Const);
        binder.install_builtin("TextDecoder", BindingKind::Const);
        // E16.01: direct eval
        binder.install_builtin("eval", BindingKind::Const);
        // E18.01: Annex B escape / unescape
        binder.install_builtin("escape", BindingKind::Const);
        binder.install_builtin("unescape", BindingKind::Const);
        // L08.01: stdlib URL parse
        binder.install_builtin("parseUrl", BindingKind::Const);
        // L08.02: query parse/serialize
        binder.install_builtin("parseQuery", BindingKind::Const);
        binder.install_builtin("serializeQuery", BindingKind::Const);
        // L03.01: SHA-256 digest over bytes
        binder.install_builtin("sha256", BindingKind::Const);
        // L03.02: OS CSPRNG bytes
        binder.install_builtin("randomBytes", BindingKind::Const);
        // L06.01: leveled logger factory
        binder.install_builtin("createLogger", BindingKind::Const);
        // L02.01: designed collections helpers (not Object.groupBy / Map.groupBy)
        binder.install_builtin("groupBy", BindingKind::Const);
        binder.install_builtin("chunk", BindingKind::Const);
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

    fn bind_program(
        &mut self,
        program: Program,
        module_goal: bool,
    ) -> Result<BoundProgram, Diagnostic> {
        if stmt_list_has_use_strict(&program.body) {
            self.strict = true;
        }
        // E19.67: Module top-level uses Block-like LexicallyDeclaredNames (functions are
        // lexical). Script/FunctionBody use TopLevel*DeclaredNames (functions are var-like).
        let top_level = !module_goal;
        self.bind_stmt_list(&program.body, top_level)?;

        Ok(BoundProgram {
            program,
            symbols: std::mem::take(&mut self.symbols),
            resolutions: std::mem::take(&mut self.resolutions),
        })
    }

    /// Two-pass list bind: declare lexical bindings in this scope, then bind each statement.
    ///
    /// `top_level`: Script or FunctionBody (TopLevel*DeclaredNames early errors).
    /// Module program body passes `false` so functions are lexical (E19.67).
    fn bind_stmt_list(&mut self, stmts: &[Stmt], top_level: bool) -> Result<(), Diagnostic> {
        // E19.24: LexicallyDeclaredNames / VarDeclaredNames early errors.
        check_statement_list_early_errors(stmts, self.strict, top_level)?;
        for stmt in stmts {
            self.declare_list_item(stmt)?;
        }
        for stmt in stmts {
            self.bind_stmt(stmt)?;
        }
        Ok(())
    }

    /// Bind a function/method/arrow block body with FunctionBody (top-level) early errors.
    fn bind_function_body(&mut self, body: &Stmt) -> Result<(), Diagnostic> {
        // Body gets its own block scope (named FE / `let arguments` / nested `function`
        // can shadow). Param∩body-lexical conflicts are checked separately.
        match body {
            Stmt::Block { body, .. } => {
                self.push_scope();
                self.bind_stmt_list(body, true)?;
                self.pop_scope();
                Ok(())
            }
            other => self.bind_stmt(other),
        }
    }

    /// E19.39: LexicallyDeclaredNames of FunctionBody must not intersect BoundNames of formals.
    fn check_params_body_lexical_conflict(
        &self,
        params: &[Param],
        body: &Stmt,
    ) -> Result<(), Diagnostic> {
        let mut param_names = std::collections::HashSet::new();
        for p in params {
            p.binding.for_each_ident(&mut |id| {
                param_names.insert(id.name.clone());
            });
        }
        let stmts: &[Stmt] = match body {
            Stmt::Block { body, .. } => body.as_slice(),
            _ => return Ok(()),
        };
        // FunctionBody uses TopLevelLexicallyDeclaredNames (not hoistable functions).
        let lexical = collect_lexically_declared_names(stmts.iter(), true);
        for (name, span, _) in lexical {
            if param_names.contains(&name) {
                return Err(Diagnostic::new(
                    format!("duplicate declaration of `{name}`"),
                    span,
                ));
            }
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
            Stmt::FunctionDeclaration {
                name,
                is_async,
                is_generator,
                ..
            } => {
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
                // Annex B / E19.24: sloppy duplicate plain FunctionDeclarations share binding.
                let scope = self.scopes.last().expect("scope stack non-empty");
                if let Some(&existing) = scope.get(&name.name) {
                    let existing_kind = self.symbols[existing.0 as usize].kind;
                    if existing_kind == BindingKind::Var
                        || (!self.strict
                            && !*is_async
                            && !*is_generator
                            && existing_kind == BindingKind::Function)
                    {
                        self.declare_annex_b_function_span(name);
                        return Ok(());
                    }
                }
                self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                Ok(())
            }
            // F06.02: `extern "C" function` binds like a function declaration (no body).
            Stmt::ExternFunctionDeclaration { name, .. } => {
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
        // E19.49: strict BindingIdentifier cannot be `eval`/`arguments`.
        if self.strict && (name == "eval" || name == "arguments") {
            return Err(Diagnostic::new(
                format!("binding `{name}` is invalid in strict mode"),
                span,
            ));
        }
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
                BindingKind::Let
                | BindingKind::Const
                | BindingKind::Using
                | BindingKind::AwaitUsing => {
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
        // E19.49: strict BindingIdentifier cannot be `eval`/`arguments`.
        if self.strict && (name == "eval" || name == "arguments") {
            return Err(Diagnostic::new(
                format!("binding `{name}` is invalid in strict mode"),
                span,
            ));
        }
        let scope = self.scopes.last_mut().expect("scope stack non-empty");
        if let Some(&existing) = scope.get(&name) {
            // `var` then `let`/`const` in the same var environment is a conflict.
            let existing_kind = self.symbols[existing.0 as usize].kind;
            if existing_kind == BindingKind::Var
                && matches!(
                    kind,
                    BindingKind::Let
                        | BindingKind::Const
                        | BindingKind::Using
                        | BindingKind::AwaitUsing
                )
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
    ///
    /// Free identifiers outside `with` also stay unresolved (E19.05): ECMA-262
    /// global object / unresolvable references are runtime GetValue/PutValue
    /// (ReferenceError on read; non-strict assign creates a global property;
    /// `typeof` unresolvable → `"undefined"`). IR emits `IdentName` /
    /// `AssignTarget::Name` for the JS backend.
    fn bind_ident_use(&mut self, id: &draconic_ast::Ident) -> Result<(), Diagnostic> {
        if let Some(sym) = self.resolve_name(&id.name) {
            let decl_depth = self.symbols[sym.0 as usize].with_depth;
            if self.with_depth == 0 || decl_depth >= self.with_depth {
                self.resolutions.insert(id.span, sym);
            }
            return Ok(());
        }
        Ok(())
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
            // F06.02: name declared in list pass; no body or param scope to bind.
            Stmt::ExternFunctionDeclaration { .. } => Ok(()),
            Stmt::Empty { .. } => Ok(()),
            Stmt::Block { body, .. } => {
                self.push_scope();
                self.bind_stmt_list(body, false)?;
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
                    if matches!(
                        kind,
                        BindingKind::Let
                            | BindingKind::Const
                            | BindingKind::Using
                            | BindingKind::AwaitUsing
                    ) {
                        // E19.67: ForDeclaration BoundNames ∩ VarDeclaredNames(Statement) empty.
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
                        self.push_scope();
                        self.declare_binding(binding, *kind)?;
                        // Pattern defaults (`[cls = class {}]`) bind free refs + class expr locals.
                        self.bind_pattern_defaults(binding)?;
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
                // E19.24: CaseBlock LexicallyDeclaredNames / VarDeclaredNames early errors.
                check_statement_list_early_errors(all_stmts.iter().copied(), self.strict, false)?;
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
            Stmt::FunctionDeclaration {
                name,
                params,
                body,
                is_async,
                is_generator,
                span,
                ..
            } => {
                // Name already declared in the enclosing list's first pass.
                // Function scope is a var environment.
                let prev_strict = self.strict;
                if body_has_use_strict(body) {
                    // E19.39: ContainsUseStrict && !IsSimpleParameterList → SyntaxError.
                    if !is_simple_parameter_list(params) {
                        return Err(Diagnostic::new(
                            "\"use strict\" not allowed in function with non-simple parameter list"
                                .to_string(),
                            *span,
                        ));
                    }
                    self.strict = true;
                }
                // E19.49: BindingIdentifier of FunctionDeclaration in strict function code.
                if self.strict && (name.name == "eval" || name.name == "arguments") {
                    return Err(Diagnostic::new(
                        format!("binding `{}` is invalid in strict mode", name.name),
                        name.span,
                    ));
                }
                // Plain/async/generator functions cannot contain SuperCall/SuperProperty.
                if params_contain_super(params) || stmt_contains_super(body) {
                    return Err(Diagnostic::new(
                        "function cannot contain super".to_string(),
                        *span,
                    ));
                }
                let prev_super = self.super_allowed;
                self.super_allowed = false;
                self.push_scope_kind(true);
                // E17.02.04: only plain (non-async/generator) functions allow sloppy dups.
                let allow_sloppy_dups = !*is_async && !*is_generator;
                self.bind_params(params, allow_sloppy_dups)?;
                self.install_arguments_object()?;
                // FunctionBody uses TopLevel*DeclaredNames early errors (E19.24).
                self.check_params_body_lexical_conflict(params, body)?;
                self.bind_function_body(body)?;
                self.pop_scope();
                self.super_allowed = prev_super;
                self.strict = prev_strict;
                Ok(())
            }
            Stmt::ClassDeclaration {
                super_class, body, ..
            } => {
                // Name already declared in the enclosing list's first pass.
                if let Some(sc) = super_class {
                    self.bind_expr(sc)?;
                }
                for el in body {
                    match el {
                        ClassElement::Constructor { params, body, .. } => {
                            // Class bodies are always strict mode code.
                            let prev_strict = self.strict;
                            let prev_super = self.super_allowed;
                            self.strict = true;
                            self.super_allowed = true;
                            self.push_scope_kind(true);
                            self.bind_params(params, false)?;
                            self.install_arguments_object()?;
                            self.check_params_body_lexical_conflict(params, body)?;
                            self.bind_function_body(body)?;
                            self.pop_scope();
                            self.super_allowed = prev_super;
                            self.strict = prev_strict;
                        }
                        ClassElement::Method {
                            key,
                            params,
                            body,
                            span,
                            ..
                        }
                        | ClassElement::Accessor {
                            key,
                            params,
                            body,
                            span,
                            ..
                        } => {
                            self.bind_object_key(key)?;
                            if body_has_use_strict(body) && !is_simple_parameter_list(params) {
                                return Err(Diagnostic::new(
                                    "\"use strict\" not allowed in function with non-simple parameter list".to_string(),
                                    *span,
                                ));
                            }
                            let prev_strict = self.strict;
                            let prev_super = self.super_allowed;
                            self.strict = true;
                            self.super_allowed = true;
                            self.push_scope_kind(true);
                            self.bind_params(params, false)?;
                            self.install_arguments_object()?;
                            self.check_params_body_lexical_conflict(params, body)?;
                            self.bind_function_body(body)?;
                            self.pop_scope();
                            self.super_allowed = prev_super;
                            self.strict = prev_strict;
                        }
                        ClassElement::Field { key, value, .. } => {
                            self.bind_object_key(key)?;
                            if let Some(v) = value {
                                // E19.82.05: field inits allow SuperProperty (lexical home object).
                                let prev_super = self.super_allowed;
                                self.super_allowed = true;
                                self.bind_expr(v)?;
                                self.super_allowed = prev_super;
                            }
                        }
                        ClassElement::StaticBlock { body, .. } => {
                            // No `arguments`; block body provides its own scope.
                            let prev_strict = self.strict;
                            self.strict = true;
                            self.bind_stmt(body)?;
                            self.strict = prev_strict;
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
                        if let Some((name, span)) = catch_lexical_conflict(param, handler) {
                            return Err(Diagnostic::new(
                                format!("duplicate declaration of `{name}`"),
                                span,
                            ));
                        }
                    }
                    self.push_scope();
                    if let Some(param) = handler_param {
                        // CatchParameter is a lexical binding (like `let`).
                        if matches!(param, BindingPattern::Member(_)) {
                            return Err(Diagnostic::new(
                                "member expression is not a valid catch binding".to_string(),
                                param.span(),
                            ));
                        }
                        self.declare_binding(param, BindingKind::Let)?;
                        self.bind_pattern_defaults(param)?;
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
            | Stmt::ExportDefaultDeclaration { span, .. }
            | Stmt::ExportAllDeclaration { span, .. } => Err(Diagnostic::new(
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
        // `for (let/const binding in/of right)` — loop-scoped bindings.
        // `for (var binding in/of right)` — function-scoped (already hoisted).
        // Annex B.3.5: `for (var name = init in right)` only.
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
            // ForDeclaration BoundNames ∩ VarDeclaredNames(Statement) must be empty.
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
            if matches!(
                kind,
                BindingKind::Let
                    | BindingKind::Const
                    | BindingKind::Using
                    | BindingKind::AwaitUsing
            ) {
                self.push_scope();
                self.declare_binding(binding, *kind)?;
                self.bind_pattern_defaults(binding)?;
                if let Some(e) = init {
                    self.bind_expr(e)?;
                }
                self.bind_expr(right)?;
                self.bind_stmt(body)?;
                self.pop_scope();
                Ok(())
            } else {
                // var: already hoisted into the enclosing var environment.
                self.bind_pattern_defaults(binding)?;
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
            | Expr::Super { .. }
            | Expr::NewTarget { .. }
            | Expr::ImportMeta { .. } => Ok(()),
            Expr::ImportCall {
                source, options, ..
            } => {
                self.bind_expr(source)?;
                if let Some(opts) = options {
                    self.bind_expr(opts)?;
                }
                Ok(())
            }
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
            Expr::Unary { op, arg, span } => {
                // E19.39: `delete IdentifierReference` is early SyntaxError in strict mode
                // (including parenthesized forms: `delete ((id))`).
                if matches!(op, UnaryOp::Delete) && self.strict {
                    let mut inner = arg.as_ref();
                    while let Expr::Paren { expr, .. } = inner {
                        inner = expr.as_ref();
                    }
                    if matches!(inner, Expr::Ident(_)) {
                        return Err(Diagnostic::new(
                            "cannot delete unqualified identifier in strict mode".to_string(),
                            *span,
                        ));
                    }
                }
                self.bind_expr(arg)
            }
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
                // E19.49: strict mode — `eval`/`arguments` are not valid simple assignment targets.
                if self.strict {
                    if let Some((name, span)) = strict_forbidden_assign_target(target) {
                        return Err(Diagnostic::new(
                            format!("cannot assign to `{name}` in strict mode"),
                            span,
                        ));
                    }
                }
                self.bind_expr(target)?;
                self.bind_expr(value)
            }
            Expr::Update { arg, .. } => {
                // E19.49: strict mode — `eval`/`arguments` are not valid update targets.
                if self.strict {
                    if let Some((name, span)) = strict_forbidden_assign_target(arg) {
                        return Err(Diagnostic::new(
                            format!("cannot assign to `{name}` in strict mode"),
                            span,
                        ));
                    }
                }
                self.bind_expr(arg)
            }
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
                name,
                params,
                body,
                is_async,
                is_generator,
                is_method,
                span,
                ..
            } => {
                // Name (if any) is local to the function body only (ES named FE).
                let prev_strict = self.strict;
                if body_has_use_strict(body) {
                    // E19.39: ContainsUseStrict && !IsSimpleParameterList → SyntaxError.
                    if !is_simple_parameter_list(params) {
                        return Err(Diagnostic::new(
                            "\"use strict\" not allowed in function with non-simple parameter list"
                                .to_string(),
                            *span,
                        ));
                    }
                    self.strict = true;
                }
                // E19.39: object/class methods (is_method) cannot contain SuperCall.
                if *is_method
                    && (params_contain_super_call(params) || stmt_contains_super_call(body))
                {
                    return Err(Diagnostic::new(
                        "method cannot contain super call".to_string(),
                        *span,
                    ));
                }
                // Non-method functions cannot contain SuperCall/SuperProperty at all.
                if !*is_method && (params_contain_super(params) || stmt_contains_super(body)) {
                    return Err(Diagnostic::new(
                        "function cannot contain super".to_string(),
                        *span,
                    ));
                }
                let prev_super = self.super_allowed;
                // Methods allow SuperProperty (and nested arrows inherit); plain FE clears it.
                self.super_allowed = *is_method;
                // Named FE: name lives in an outer env so params may shadow it
                // (e.g. `function await(await) {}` — E19.52).
                let named = name.is_some();
                if named {
                    self.push_scope_kind(true);
                    if let Some(name) = name {
                        self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                    }
                }
                self.push_scope_kind(true);
                // E17.02.04: methods / async / generators use UniqueFormalParameters.
                let allow_sloppy_dups = !*is_async && !*is_generator && !*is_method;
                self.bind_params(params, allow_sloppy_dups)?;
                self.install_arguments_object()?;
                self.check_params_body_lexical_conflict(params, body)?;
                self.bind_function_body(body)?;
                self.super_allowed = prev_super;
                self.pop_scope();
                if named {
                    self.pop_scope();
                }
                self.strict = prev_strict;
                Ok(())
            }
            Expr::ClassExpression {
                name,
                super_class,
                body,
                span,
                ..
            } => {
                // Name (if any) is local to the class body only (ES named class expression).
                // Anonymous class expressions get a synthetic binding keyed by the expr span.
                self.push_scope_kind(true);
                if let Some(name) = name {
                    self.declare(name.name.clone(), name.span, BindingKind::Function)?;
                } else {
                    self.declare("__class".into(), *span, BindingKind::Function)?;
                }
                if let Some(sc) = super_class {
                    self.bind_expr(sc)?;
                }
                for el in body {
                    match el {
                        ClassElement::Constructor { params, body, .. } => {
                            let prev_strict = self.strict;
                            let prev_super = self.super_allowed;
                            self.strict = true;
                            self.super_allowed = true;
                            self.push_scope_kind(true);
                            self.bind_params(params, false)?;
                            self.install_arguments_object()?;
                            self.check_params_body_lexical_conflict(params, body)?;
                            self.bind_function_body(body)?;
                            self.pop_scope();
                            self.super_allowed = prev_super;
                            self.strict = prev_strict;
                        }
                        ClassElement::Method {
                            key,
                            params,
                            body,
                            span,
                            ..
                        }
                        | ClassElement::Accessor {
                            key,
                            params,
                            body,
                            span,
                            ..
                        } => {
                            self.bind_object_key(key)?;
                            if body_has_use_strict(body) && !is_simple_parameter_list(params) {
                                return Err(Diagnostic::new(
                                    "\"use strict\" not allowed in function with non-simple parameter list".to_string(),
                                    *span,
                                ));
                            }
                            let prev_strict = self.strict;
                            let prev_super = self.super_allowed;
                            self.strict = true;
                            self.super_allowed = true;
                            self.push_scope_kind(true);
                            self.bind_params(params, false)?;
                            self.install_arguments_object()?;
                            self.check_params_body_lexical_conflict(params, body)?;
                            self.bind_function_body(body)?;
                            self.pop_scope();
                            self.super_allowed = prev_super;
                            self.strict = prev_strict;
                        }
                        ClassElement::Field { key, value, .. } => {
                            self.bind_object_key(key)?;
                            if let Some(v) = value {
                                // E19.82.05: field inits allow SuperProperty (lexical home object).
                                let prev_super = self.super_allowed;
                                self.super_allowed = true;
                                self.bind_expr(v)?;
                                self.super_allowed = prev_super;
                            }
                        }
                        ClassElement::StaticBlock { body, .. } => {
                            let prev_strict = self.strict;
                            self.strict = true;
                            self.bind_stmt(body)?;
                            self.strict = prev_strict;
                        }
                    }
                }
                self.pop_scope();
                Ok(())
            }
            Expr::ArrowFunction {
                params, body, span, ..
            } => {
                let prev_strict = self.strict;
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
                    self.strict = true;
                }
                // SuperCall/SuperProperty in arrows: lexical — allowed when nested in a
                // Super-enabled context (ctor/method/field). Outer early errors reject
                // SuperCall in methods/fields/static-blocks via Contains SuperCall (E19.82.05).
                // SuperProperty only when nested in method/constructor/field (lexical super).
                let body_super = match body {
                    ArrowBody::Expr(e) => expr_contains_super(e),
                    ArrowBody::Block(s) => stmt_contains_super(s),
                };
                if !self.super_allowed && (params_contain_super(params) || body_super) {
                    return Err(Diagnostic::new(
                        "arrow function cannot contain super".to_string(),
                        *span,
                    ));
                }
                self.push_scope_kind(true);
                self.bind_params(params, false)?;
                match body {
                    ArrowBody::Expr(expr) => self.bind_expr(expr)?,
                    ArrowBody::Block(stmt) => {
                        self.check_params_body_lexical_conflict(params, stmt)?;
                        self.bind_function_body(stmt)?;
                    }
                }
                self.pop_scope();
                self.strict = prev_strict;
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
                            key,
                            params,
                            body,
                            span,
                            ..
                        } => {
                            match key {
                                ObjectKey::Ident(_) | ObjectKey::String(_) => {}
                                ObjectKey::Computed(expr) => self.bind_expr(expr)?,
                            }
                            let prev_strict = self.strict;
                            if body_has_use_strict(body) {
                                if !is_simple_parameter_list(params) {
                                    return Err(Diagnostic::new(
                                        "\"use strict\" not allowed in function with non-simple parameter list".to_string(),
                                        *span,
                                    ));
                                }
                                self.strict = true;
                            }
                            if params_contain_super_call(params) || stmt_contains_super_call(body) {
                                return Err(Diagnostic::new(
                                    "method cannot contain super call".to_string(),
                                    *span,
                                ));
                            }
                            self.push_scope_kind(true);
                            self.bind_params(params, false)?;
                            self.install_arguments_object()?;
                            self.check_params_body_lexical_conflict(params, body)?;
                            self.bind_function_body(body)?;
                            self.pop_scope();
                            self.strict = prev_strict;
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
                        ArrayElement::Elision => {}
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
            Expr::PrivateIn { object, .. } => self.bind_expr(object),
            Expr::Paren { expr, .. } => self.bind_expr(expr),
            Expr::As { expr, .. } => self.bind_expr(expr),
            Expr::ArrayPattern { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Elision => {}
                        ArrayPatternElement::Pattern { binding, default } => {
                            self.bind_assign_pattern(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ArrayPatternElement::Rest(binding) => {
                            self.bind_assign_pattern(binding)?;
                        }
                    }
                }
                Ok(())
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
                            self.bind_object_key(key)?;
                            self.bind_assign_pattern(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ObjectPatternProp::Rest(binding) => {
                            self.bind_assign_pattern(binding)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    fn bind_assign_pattern(&mut self, pat: &BindingPattern) -> Result<(), Diagnostic> {
        match pat {
            BindingPattern::Ident(id) => {
                // E19.39: strict mode — `eval`/`arguments` are not valid simple assignment targets.
                if self.strict && (id.name == "eval" || id.name == "arguments") {
                    return Err(Diagnostic::new(
                        format!("cannot assign to `{}` in strict mode", id.name),
                        id.span,
                    ));
                }
                self.bind_expr(&Expr::Ident(id.clone()))
            }
            BindingPattern::Member(expr) => self.bind_expr(expr),
            BindingPattern::Array { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Elision => {}
                        ArrayPatternElement::Pattern { binding, default } => {
                            self.bind_assign_pattern(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ArrayPatternElement::Rest(binding) => {
                            self.bind_assign_pattern(binding)?;
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
                            self.bind_object_key(key)?;
                            self.bind_assign_pattern(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ObjectPatternProp::Rest(binding) => {
                            self.bind_assign_pattern(binding)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    /// Bind free references in pattern default initializers (`pat = expr`)
    /// and computed property names in object patterns.
    fn bind_pattern_defaults(&mut self, pat: &BindingPattern) -> Result<(), Diagnostic> {
        match pat {
            BindingPattern::Ident(_) | BindingPattern::Member(_) => Ok(()),
            BindingPattern::Array { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Elision => {}
                        ArrayPatternElement::Pattern { binding, default } => {
                            self.bind_pattern_defaults(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ArrayPatternElement::Rest(binding) => {
                            self.bind_pattern_defaults(binding)?;
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
                            self.bind_object_key(key)?;
                            self.bind_pattern_defaults(binding)?;
                            if let Some(def) = default {
                                self.bind_expr(def)?;
                            }
                        }
                        ObjectPatternProp::Rest(binding) => {
                            self.bind_pattern_defaults(binding)?;
                        }
                    }
                }
                Ok(())
            }
        }
    }

    fn bind_object_key(&mut self, key: &ObjectKey) -> Result<(), Diagnostic> {
        match key {
            ObjectKey::Ident(_) | ObjectKey::String(_) => Ok(()),
            ObjectKey::Computed(expr) => self.bind_expr(expr),
        }
    }

    fn bind_params(&mut self, params: &[Param], allow_sloppy_dups: bool) -> Result<(), Diagnostic> {
        // E19.24: strict FormalParameters / ArrowParameters cannot bind `eval` or `arguments`.
        if self.strict {
            for p in params {
                let mut err = None;
                p.binding.for_each_ident(&mut |id| {
                    if err.is_some() {
                        return;
                    }
                    if id.name == "eval" || id.name == "arguments" {
                        err = Some(Diagnostic::new(
                            format!("binding `{}` is invalid in strict mode", id.name),
                            id.span,
                        ));
                    }
                });
                if let Some(e) = err {
                    return Err(e);
                }
            }
        }
        // E17.02.04: non-strict simple FormalParameters on plain `function` may
        // repeat BoundNames (last wins). Strict, non-simple, arrows, methods,
        // async, and generators require unique names.
        let simple = is_simple_parameter_list(params);
        let allow_dups = allow_sloppy_dups && !self.strict && simple;
        if !allow_dups {
            let mut seen: HashMap<String, Span> = HashMap::new();
            for p in params {
                let mut err = None;
                p.binding.for_each_ident(&mut |id| {
                    if err.is_some() {
                        return;
                    }
                    if seen.contains_key(&id.name) {
                        err = Some(Diagnostic::new(
                            format!("duplicate declaration of `{}`", id.name),
                            id.span,
                        ));
                    } else {
                        seen.insert(id.name.clone(), id.span);
                    }
                });
                if let Some(e) = err {
                    return Err(e);
                }
            }
        }
        for p in params {
            if allow_dups {
                // Simple list: each param is a single Ident.
                if let BindingPattern::Ident(id) = &p.binding {
                    self.declare_param_allow_dup(id.name.clone(), id.span)?;
                } else {
                    self.declare_binding(&p.binding, BindingKind::Let)?;
                }
            } else {
                self.declare_binding(&p.binding, BindingKind::Let)?;
            }
        }
        for p in params {
            self.bind_pattern_defaults(&p.binding)?;
            if let Some(default) = &p.default {
                self.bind_expr(default)?;
            }
        }
        Ok(())
    }

    /// Declare a formal binding, allowing a later same-name formal to replace
    /// the scope entry (E17.02.04). Each declaration span keeps its own symbol
    /// so IR can lower every parameter pattern.
    fn declare_param_allow_dup(
        &mut self,
        name: String,
        span: Span,
    ) -> Result<SymbolId, Diagnostic> {
        let scope = self.scopes.last_mut().expect("scope stack non-empty");
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: name.clone(),
            span,
            kind: BindingKind::Let,
            with_depth: self.with_depth,
        });
        scope.insert(name, id);
        Ok(id)
    }

    /// Implicit `arguments` binding for non-arrow functions (E18.24).
    /// Skipped when a param already shadows the name. Arrows inherit lexically.
    fn install_arguments_object(&mut self) -> Result<(), Diagnostic> {
        let scope = self.scopes.last_mut().expect("scope stack non-empty");
        if scope.contains_key("arguments") {
            return Ok(());
        }
        // Implicit Arguments object — not a user BindingIdentifier (ok in strict).
        let id = SymbolId(self.symbols.len() as u32);
        self.symbols.push(Symbol {
            id,
            name: "arguments".into(),
            span: Span::dummy(),
            kind: BindingKind::Var,
            with_depth: self.with_depth,
        });
        scope.insert("arguments".into(), id);
        Ok(())
    }
}

struct Checker<'a> {
    bound: &'a BoundProgram,
    symbol_types: Vec<Type>,
    /// True when the binding's type came from a type annotation (not inference).
    /// Untyped JS assignment may widen inferred bindings (E19.12 / E19.48).
    symbol_annotated: Vec<bool>,
    /// Resolved call signatures for annotated functions (T07.01), parallel to `symbol_types`.
    fn_sigs: Vec<Option<FnSig>>,
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
    /// When set, free host API references are checked against this target (H00.01).
    host_target: Option<CompileTarget>,
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
        let n = bound.symbols().len();
        Self {
            bound,
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
                self.generic_aliases
                    .insert(name, GenericAlias { params, body: ty });
            }
        }
        let mut labels = Vec::new();
        for stmt in &self.bound.program.body {
            self.check_stmt(stmt, 0, 0, 0, &mut labels)?;
        }
        Ok(())
    }

    /// Left side of `for-in` / `for-of`: `let`/`const`/`var` binding or assignable LHS.
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
                // Iteration values are JS values; leave bindings as Any until finer types.
                self.check_binding_pattern(binding, Type::Any)
            }
            Stmt::Expression {
                expr: Expr::Ident(id),
                ..
            } => {
                // E17.02.09 / E19.05: free IdentifierReference is runtime PutValue
                // (non-strict creates a global; strict → ReferenceError), not a check error.
                if let Some(sym) = self.bound.resolve(id.span) {
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

    fn check_binding_pattern(
        &mut self,
        binding: &BindingPattern,
        ty: Type,
    ) -> Result<(), Diagnostic> {
        self.check_binding_pattern_annotated(binding, ty, false)
    }

    fn check_binding_pattern_annotated(
        &mut self,
        binding: &BindingPattern,
        ty: Type,
        annotated: bool,
    ) -> Result<(), Diagnostic> {
        match binding {
            BindingPattern::Ident(name) => {
                let id = self
                    .bound
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

    fn check_assign_pattern(
        &mut self,
        binding: &BindingPattern,
        span: Span,
    ) -> Result<(), Diagnostic> {
        match binding {
            BindingPattern::Ident(id) => {
                let Some(sym) = self.bound.resolve(id.span) else {
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
    fn check_missing_return(&self, body: &Stmt, ret_ty: Type) -> Result<(), Diagnostic> {
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
            // F06.02: bind as function; params/return must be native ABI types.
            Stmt::ExternFunctionDeclaration {
                name,
                params,
                return_type,
                span,
                ..
            } => self.check_extern_function_declaration(name, params, return_type, *span),
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
                if *kind == BindingKind::AwaitUsing && !self.in_async {
                    return Err(Diagnostic::new(
                        "await using is only valid in async functions and modules".to_string(),
                        *span,
                    ));
                }
                let ann_ty = match type_ann {
                    Some(ann) => Some(self.resolve_type_ann(ann)?),
                    None => None,
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
                            if let Some(sym) =
                                self.bound.symbols().iter().find(|s| s.span == name.span)
                            {
                                self.fn_sigs[sym.id.0 as usize] = Some(sig);
                            }
                        }
                    }
                }
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
            } => {
                self.check_for_in_of_left(left)?;
                self.check_expr(right)?;
                self.check_stmt(body, loop_depth + 1, switch_depth, fn_depth, labels)
            }
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
                span,
            } => {
                if *is_await && !self.in_async {
                    return Err(Diagnostic::new(
                        "for await is only valid in async functions and modules".to_string(),
                        *span,
                    ));
                }
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
                span,
                ..
            } => {
                // E19.49: undeclared function name (e.g. with-body before parse reject) → diagnostic.
                let Some(id) = self
                    .bound
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
                // FunctionDeclaration formals: +Await only for async generators.
                self.check_params_await_yield(params, *is_async && *is_generator, *is_generator)?;
                // T07.01: record a call signature for non-generic annotated functions so
                // call sites can check arity and argument types (generics use instantiate_generic_call).
                if type_params.is_empty() {
                    if let Some(sig) = self.fn_sig_from_params(params) {
                        self.fn_sigs[id.0 as usize] = Some(sig);
                    }
                }
                // Fresh label set inside functions (labels do not cross function boundaries).
                let mut inner_labels = Vec::new();
                let prev_async = self.in_async;
                let prev_generator = self.in_generator;
                let prev_ret = self.expected_return;
                self.in_async = *is_async;
                self.in_generator = *is_generator;
                let ret_ty = match return_type {
                    Some(ann) => Some(self.resolve_type_ann(ann)?),
                    None => None,
                };
                self.expected_return = ret_ty;
                let result = (|| {
                    self.check_stmt(body, 0, 0, fn_depth + 1, &mut inner_labels)?;
                    if let Some(ty) = ret_ty {
                        self.check_missing_return(body, ty)?;
                    }
                    Ok(())
                })();
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
                for el in body {
                    match el {
                        ClassElement::Constructor { params, body, .. } => {
                            self.check_params_await_yield(params, false, false)?;
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
                            key,
                            params,
                            body,
                            is_async,
                            is_generator,
                            ..
                        } => {
                            self.check_object_key(key)?;
                            self.check_params_await_yield(
                                params,
                                *is_async && *is_generator,
                                *is_generator,
                            )?;
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = *is_async;
                            self.in_generator = *is_generator;
                            self.check_stmt(body, 0, 0, fn_depth + 1, &mut inner_labels)?;
                            self.in_async = prev_async;
                            self.in_generator = prev_generator;
                        }
                        ClassElement::Accessor {
                            key, params, body, ..
                        } => {
                            self.check_object_key(key)?;
                            self.check_params_await_yield(params, false, false)?;
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = false;
                            self.in_generator = false;
                            self.check_stmt(body, 0, 0, fn_depth + 1, &mut inner_labels)?;
                            self.in_async = prev_async;
                            self.in_generator = prev_generator;
                        }
                        ClassElement::Field { key, value, .. } => {
                            self.check_object_key(key)?;
                            if let Some(v) = value {
                                self.check_expr(v)?;
                            }
                        }
                        ClassElement::StaticBlock { body, .. } => {
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = false;
                            self.in_generator = false;
                            // Static blocks are not nested functions for return; treat as fn-like.
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
                        self.check_binding_pattern(param, Type::Any)?;
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
            | Stmt::ExportDefaultDeclaration { span, .. }
            | Stmt::ExportAllDeclaration { span, .. } => Err(Diagnostic::new(
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
                if let Some(sym) = self.bound.resolve(id.span) {
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
                        "await is only valid in async functions and modules".to_string(),
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
                // E19.60: peel cover parentheses so `(id) = v` is a simple assignment target.
                match peel_parens(target.as_ref()) {
                    Expr::Ident(id) => {
                        let Some(sym) = self.bound.resolve(id.span) else {
                            // Free / with-chain assign target.
                            self.record(id.span, Type::Any);
                            self.record(*span, value_ty);
                            return Ok(value_ty);
                        };
                        // E19.57 / E19.60: const/using/function-name PutValue is runtime
                        // TypeError (or silent); do not compile-reject.
                        let kind = self.bound.symbol(sym).kind;
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
                                    let native = matches!(left_ty, Type::Native(_) | Type::Ptr(_))
                                        || matches!(result_ty, Type::Native(_) | Type::Ptr(_));
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
                // E19.60: peel cover parentheses so `(id)++` is a valid update target.
                match peel_parens(arg.as_ref()) {
                    Expr::Ident(id) => {
                        let Some(sym) = self.bound.resolve(id.span) else {
                            self.record(id.span, Type::Any);
                            self.record(*span, Type::Number);
                            return Ok(Type::Number);
                        };
                        // E19.57 / E19.60: const/using/function-name PutValue is runtime
                        // TypeError (or silent); do not compile-reject.
                        let kind = self.bound.symbol(sym).kind;
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
                    if let Some(sym_id) = self.bound.resolve(id.span) {
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
                let result_ty = match callee_ty {
                    Type::GenericFn(gid) => self.instantiate_generic_call(gid, &arg_tys, *span)?,
                    // Native/ptr have no JS [[Call]].
                    Type::Native(_) | Type::Ptr(_) => {
                        return Err(Diagnostic::new(
                            format!("type `{callee_ty}` is not callable"),
                            *span,
                        )
                        .with_code(codes::NOT_CALLABLE)
                        .with_help(
                            "only functions (and values with a call signature) can be called",
                        ));
                    }
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
                if matches!(callee_ty, Type::Native(_) | Type::Ptr(_)) {
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
                    if let Some(sym_id) = self.bound.resolve(id.span) {
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
                span,
                ..
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
                // FunctionExpression formals: +Await only for async generators.
                self.check_params_await_yield(params, *is_async && *is_generator, *is_generator)?;
                // New function boundary (return allowed; labels do not escape).
                let mut inner_labels = Vec::new();
                let prev_async = self.in_async;
                let prev_generator = self.in_generator;
                let prev_ret = self.expected_return;
                self.in_async = *is_async;
                self.in_generator = *is_generator;
                let ret_ty = match return_type {
                    Some(ann) => Some(self.resolve_type_ann(ann)?),
                    None => None,
                };
                self.expected_return = ret_ty;
                let result = (|| {
                    self.check_stmt(body, 0, 0, 1, &mut inner_labels)?;
                    if let Some(ty) = ret_ty {
                        self.check_missing_return(body, ty)?;
                    }
                    Ok(())
                })();
                self.in_async = prev_async;
                self.in_generator = prev_generator;
                self.expected_return = prev_ret;
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
                let class_span = name.as_ref().map(|n| n.span).unwrap_or(*span);
                let id = self
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == class_span)
                    .map(|s| s.id)
                    .expect("class expression binding must be declared");
                self.symbol_types[id.0 as usize] = Type::Function;
                if let Some(sc) = super_class {
                    self.check_expr(sc)?;
                }
                for el in body {
                    match el {
                        ClassElement::Constructor { params, body, .. } => {
                            self.check_params_await_yield(params, false, false)?;
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = false;
                            self.in_generator = false;
                            self.check_stmt(body, 0, 0, 1, &mut inner_labels)?;
                            self.in_async = prev_async;
                            self.in_generator = prev_generator;
                        }
                        ClassElement::Method {
                            key,
                            params,
                            body,
                            is_async,
                            is_generator,
                            ..
                        } => {
                            self.check_object_key(key)?;
                            self.check_params_await_yield(
                                params,
                                *is_async && *is_generator,
                                *is_generator,
                            )?;
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = *is_async;
                            self.in_generator = *is_generator;
                            self.check_stmt(body, 0, 0, 1, &mut inner_labels)?;
                            self.in_async = prev_async;
                            self.in_generator = prev_generator;
                        }
                        ClassElement::Accessor {
                            key, params, body, ..
                        } => {
                            self.check_object_key(key)?;
                            self.check_params_await_yield(params, false, false)?;
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = false;
                            self.in_generator = false;
                            self.check_stmt(body, 0, 0, 1, &mut inner_labels)?;
                            self.in_async = prev_async;
                            self.in_generator = prev_generator;
                        }
                        ClassElement::Field { key, value, .. } => {
                            self.check_object_key(key)?;
                            if let Some(v) = value {
                                self.check_expr(v)?;
                            }
                        }
                        ClassElement::StaticBlock { body, .. } => {
                            let mut inner_labels = Vec::new();
                            let prev_async = self.in_async;
                            let prev_generator = self.in_generator;
                            self.in_async = false;
                            self.in_generator = false;
                            self.check_stmt(body, 0, 0, 1, &mut inner_labels)?;
                            self.in_async = prev_async;
                            self.in_generator = prev_generator;
                        }
                    }
                }
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
                // Async arrows: UniqueFormalParameters[~Yield, +Await].
                self.check_params_await_yield(params, *is_async, false)?;
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
                        if let Some(ty) = ret_ty {
                            self.check_missing_return(stmt, ty)?;
                        }
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
                            self.check_params_await_yield(params, false, false)?;
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

    fn check_object_key(&mut self, key: &ObjectKey) -> Result<(), Diagnostic> {
        match key {
            ObjectKey::Ident(_) | ObjectKey::String(_) => Ok(()),
            ObjectKey::Computed(expr) => {
                self.check_expr(expr)?;
                Ok(())
            }
        }
    }

    fn check_params(&mut self, params: &[Param]) -> Result<(), Diagnostic> {
        for (i, p) in params.iter().enumerate() {
            if p.rest {
                if i != params.len() - 1 {
                    return Err(Diagnostic::new(
                        "rest parameter must be last formal parameter".to_string(),
                        p.binding.span(),
                    ));
                }
                if p.default.is_some() {
                    return Err(Diagnostic::new(
                        "rest parameter cannot have a default".to_string(),
                        p.binding.span(),
                    ));
                }
            }
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
            let annotated = ann_ty.is_some();
            self.check_binding_pattern_annotated(
                &p.binding,
                ann_ty.unwrap_or(Type::Any),
                annotated,
            )?;
        }
        Ok(())
    }

    /// Check formals under the correct Await/Yield grammar flags (E19.28).
    ///
    /// Module top-level `+Await` must not leak into `FormalParameters[~Await]`
    /// (ordinary / async function / method params). Async generators and async
    /// arrows use `+Await` in parameter lists.
    fn check_params_await_yield(
        &mut self,
        params: &[Param],
        await_ok: bool,
        yield_ok: bool,
    ) -> Result<(), Diagnostic> {
        let prev_async = self.in_async;
        let prev_generator = self.in_generator;
        self.in_async = await_ok;
        self.in_generator = yield_ok;
        let result = self.check_params(params);
        self.in_async = prev_async;
        self.in_generator = prev_generator;
        result
    }

    fn intern_shape(&mut self, props: Vec<(String, Type)>, strict: bool) -> Type {
        let id = self.shapes.len() as u32;
        self.shapes.push(ObjectShape { props, strict });
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
        // A merged shape is strict only when every contributing shape is strict,
        // so inferred (permissive) shapes never make an annotated one reject.
        let mut shape_strict = true;
        let mut rest = Vec::new();
        for m in out {
            match m {
                Type::Shape(id) => {
                    if let Some(shape) = self.shapes.get(id as usize) {
                        shape_strict = shape_strict && shape.strict;
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
            rest.push(self.intern_shape(props, shape_strict));
        }
        if rest.is_empty() {
            return Type::Any;
        }
        if rest.len() == 1 {
            return rest[0];
        }
        let id = self.intersections.len() as u32;
        self.intersections.push(IntersectionType { members: rest });
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

    /// T07.03: type of `obj.name`, rejecting access to a property absent from a
    /// strict (annotated) shape. Untyped (`Any`/`Object`), inferred object-literal,
    /// and tuple shapes stay dynamic (permissive), as do intersections with any
    /// non-strict member.
    fn member_prop_type(&self, obj: Type, name: &str, span: Span) -> Result<Type, Diagnostic> {
        match obj {
            Type::Shape(id) => {
                let shape = self.shapes.get(id as usize);
                if let Some((_, t)) = shape.and_then(|s| s.props.iter().find(|(n, _)| n == name)) {
                    return Ok(*t);
                }
                if shape.is_some_and(|s| s.strict) {
                    return Err(Self::unknown_property_diagnostic(obj, name, span, self));
                }
                Ok(Type::Any)
            }
            Type::Intersection(id) => {
                let i = self.intersections.get(id as usize);
                if let Some(t) =
                    i.and_then(|i| i.members.iter().find_map(|m| self.prop_type(*m, name)))
                {
                    return Ok(t);
                }
                if i.is_some_and(|i| i.members.iter().all(|m| self.is_strict_member(*m))) {
                    return Err(Self::unknown_property_diagnostic(obj, name, span, self));
                }
                Ok(Type::Any)
            }
            _ => Ok(Type::Any),
        }
    }

    fn unknown_property_diagnostic(
        obj: Type,
        name: &str,
        span: Span,
        checker: &Self,
    ) -> Diagnostic {
        let obj_s = format_type_full(
            obj,
            &checker.shapes,
            &checker.unions,
            &checker.intersections,
        );
        Diagnostic::new(format!("unknown property `{name}` on type `{obj_s}`"), span)
            .with_code(codes::UNKNOWN_PROPERTY)
            .with_help("check the property name, or extend the type annotation to include it")
    }

    /// Whether a type is entirely strict (annotated) shapes, recursing intersections.
    fn is_strict_member(&self, ty: Type) -> bool {
        match ty {
            Type::Shape(id) => self.shapes.get(id as usize).is_some_and(|s| s.strict),
            Type::Intersection(id) => self
                .intersections
                .get(id as usize)
                .is_some_and(|i| i.members.iter().all(|m| self.is_strict_member(*m))),
            _ => false,
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
                                format!("generic type `{other}` requires type arguments"),
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
                let mut shape_props = Vec::new();
                for p in props {
                    let ty = self.resolve_type_ann(&p.ty)?;
                    shape_props.push((p.name.clone(), ty));
                }
                Ok(self.intern_shape(shape_props, true))
            }
            TypeAnn::Pointer { inner, span } => {
                let pointee = self.resolve_type_ann(inner)?;
                match pointee {
                    Type::Native(n) => Ok(Type::Ptr(n)),
                    other => Err(Diagnostic::new(
                        format!("pointer pointee must be a native scalar type, got `{other}`"),
                        *span,
                    )),
                }
            }
            TypeAnn::Tuple { elements, .. } => {
                let mut shape_props = Vec::new();
                for (i, el) in elements.iter().enumerate() {
                    let ty = self.resolve_type_ann(el)?;
                    shape_props.push((i.to_string(), ty));
                }
                Ok(self.intern_shape(shape_props, false))
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
            self.type_param_env.insert(p.clone(), Type::TypeParam(id));
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
            Expr::Paren { expr, .. } | Expr::As { expr, .. } => Self::is_number_literal_expr(expr),
            _ => false,
        }
    }

    /// Non-negative integer index key from a constant number literal (`0` → `"0"`).
    fn const_index_key(expr: &Expr) -> Option<String> {
        let raw = match expr {
            Expr::Number(n) => n.raw.as_str(),
            Expr::Paren { expr, .. } => return Self::const_index_key(expr),
            _ => return None,
        };
        // Decimal integer only (no float/hex/bin for tuple index keys this Loop).
        if raw.is_empty() || !raw.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        // Normalize leading zeros: "00" → "0" via parse.
        let n: u64 = raw.parse().ok()?;
        Some(n.to_string())
    }

    fn number_literal_ok_for_native(to: Type) -> bool {
        matches!(to, Type::Native(n) if !n.is_bool())
    }

    /// Explicit dual-worlds boundary (`as`): JS `number` ↔ unboxed native numeric (T06).
    fn is_dual_world_boundary(from: Type, to: Type) -> bool {
        matches!(
            (from, to),
            (
                Type::Number,
                Type::Native(n)
            ) if !n.is_bool()
        ) || matches!(
            (from, to),
            (
                Type::Native(n),
                Type::Number
            ) if !n.is_bool()
        )
    }

    /// Assignability with contextual typing of numeric literals to native types (T05),
    /// object literals to native-layout shapes (N03.01), and array literals to tuple
    /// layouts (N03.02).
    fn require_assignable_expr(
        &self,
        from: Type,
        to: Type,
        from_expr: &Expr,
    ) -> Result<(), Diagnostic> {
        // T07.05: a fresh object literal must not name properties absent from an
        // annotated (strict) shape.
        if let Some(diag) = self.excess_prop_diag(from_expr, to) {
            return Err(diag);
        }
        if self.is_assignable(from, to) {
            return Ok(());
        }
        if Self::is_number_literal_expr(from_expr) && Self::number_literal_ok_for_native(to) {
            return Ok(());
        }
        if let (Expr::ObjectExpression { properties, .. }, Type::Shape(to_id)) = (from_expr, to) {
            if let Some(to_shape) = self.shapes.get(to_id as usize) {
                if self.object_literal_contextually_assignable(properties, to_shape) {
                    return Ok(());
                }
            }
        }
        if let (Expr::ArrayExpression { elements, .. }, Type::Shape(to_id)) = (from_expr, to) {
            if let Some(to_shape) = self.shapes.get(to_id as usize) {
                if self.array_literal_contextually_assignable(elements, to_shape) {
                    return Ok(());
                }
            }
        }
        self.require_assignable(from, to, expr_span_of(from_expr))
    }

    /// Object literal may assign to a shape when each required property is present and
    /// assignable, allowing number/boolean literals to fill native scalar fields.
    fn object_literal_contextually_assignable(
        &self,
        properties: &[ObjectProp],
        to_shape: &ObjectShape,
    ) -> bool {
        let mut by_name: HashMap<String, &Expr> = HashMap::new();
        for prop in properties {
            match prop {
                ObjectProp::Property { key, value, .. } => {
                    let name = match key {
                        ObjectKey::Ident(id) => id.name.clone(),
                        ObjectKey::String(s) => s.value.to_string_lossy(),
                        ObjectKey::Computed(_) => return false,
                    };
                    by_name.insert(name, value);
                }
                ObjectProp::Accessor { .. } | ObjectProp::Spread { .. } => return false,
            }
        }
        to_shape.props.iter().all(|(name, want)| {
            let Some(val) = by_name.get(name) else {
                return false;
            };
            self.expr_contextually_assignable_to(val, *want)
        })
    }

    /// Entry point for the T07.05 excess-property check: returns a diagnostic when
    /// `from_expr` is a fresh object literal assigned to an annotated (strict) shape.
    fn excess_prop_diag(&self, from_expr: &Expr, to: Type) -> Option<Diagnostic> {
        if let (Expr::ObjectExpression { properties, .. }, Type::Shape(to_id)) = (from_expr, to) {
            if let Some(to_shape) = self.shapes.get(to_id as usize) {
                return self.object_literal_excess_diag(properties, to_shape);
            }
        }
        None
    }

    /// Excess-property check (T07.05): a fresh object literal assigned to an annotated
    /// (strict) shape must not name properties absent from that shape. Recurses into
    /// nested object literals against nested strict shapes. Computed keys, spreads, and
    /// accessors make the literal permissive (keys not statically known).
    fn object_literal_excess_diag(
        &self,
        properties: &[ObjectProp],
        to_shape: &ObjectShape,
    ) -> Option<Diagnostic> {
        if !to_shape.strict {
            return None;
        }
        for prop in properties {
            let ObjectProp::Property {
                key, value, span, ..
            } = prop
            else {
                return None;
            };
            let name = match key {
                ObjectKey::Ident(id) => id.name.clone(),
                ObjectKey::String(s) => s.value.to_string_lossy(),
                ObjectKey::Computed(_) => return None,
            };
            let Some(want) = to_shape
                .props
                .iter()
                .find(|(n, _)| n == &name)
                .map(|(_, t)| *t)
            else {
                return Some(
                    Diagnostic::new(
                        format!(
                            "object literal has excess property `{name}` not in annotated shape"
                        ),
                        *span,
                    )
                    .with_code(codes::EXCESS_PROPERTY)
                    .with_help("remove the extra property, or add it to the annotated shape"),
                );
            };
            if let (
                Expr::ObjectExpression {
                    properties: inner, ..
                },
                Type::Shape(inner_id),
            ) = (value, want)
            {
                if let Some(inner_shape) = self.shapes.get(inner_id as usize) {
                    if let Some(diag) = self.object_literal_excess_diag(inner, inner_shape) {
                        return Some(diag);
                    }
                }
            }
        }
        None
    }

    /// Array literal may assign to a tuple shape (`"0"`, `"1"`, …) by position (N03.02).
    fn array_literal_contextually_assignable(
        &self,
        elements: &[ArrayElement],
        to_shape: &ObjectShape,
    ) -> bool {
        if elements.len() != to_shape.props.len() {
            return false;
        }
        for (i, (name, want)) in to_shape.props.iter().enumerate() {
            if name != &i.to_string() {
                return false;
            }
            let ArrayElement::Expr(val) = &elements[i] else {
                return false;
            };
            if !self.expr_contextually_assignable_to(val, *want) {
                return false;
            }
        }
        true
    }

    fn expr_contextually_assignable_to(&self, val: &Expr, want: Type) -> bool {
        let got = self
            .expr_types
            .get(&expr_span_of(val))
            .copied()
            .unwrap_or(Type::Any);
        if self.is_assignable(got, want) {
            return true;
        }
        if Self::is_number_literal_expr(val) && Self::number_literal_ok_for_native(want) {
            return true;
        }
        if matches!(val, Expr::Boolean { .. }) && matches!(want, Type::Native(NativeType::Bool)) {
            return true;
        }
        false
    }

    /// Whether `from` is assignable to `to` (exact, `any`, structural, union/intersection).
    fn is_assignable(&self, from: Type, to: Type) -> bool {
        if from == to || from == Type::Any || to == Type::Any {
            return true;
        }
        // F01.03: JS `null` → native null pointer (`*T`).
        if from == Type::Null && matches!(to, Type::Ptr(_)) {
            return true;
        }
        // JS `boolean` (literals, comparisons) → native `bool` (N02).
        if from == Type::Boolean && matches!(to, Type::Native(NativeType::Bool)) {
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

    fn require_assignable(&self, from: Type, to: Type, span: Span) -> Result<(), Diagnostic> {
        if self.is_assignable(from, to) {
            Ok(())
        } else {
            let from_s = format_type_full(from, &self.shapes, &self.unions, &self.intersections);
            let to_s = format_type_full(to, &self.shapes, &self.unions, &self.intersections);
            Err(Diagnostic::new(
                format!("type `{from_s}` is not assignable to type `{to_s}`"),
                span,
            )
            .with_code(codes::NOT_ASSIGNABLE)
            .with_help("change the value to match the expected type, or widen the annotation"))
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
                matches!(ty, Type::Object | Type::Shape(_) | Type::Null | Type::Any)
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

    /// F06.02: check an `extern "C" function` declaration.
    ///
    /// - Binds the name as a callable `function`.
    /// - Every parameter must be annotated with a native scalar, pointer, `function`, or native layout.
    /// - Return type is optional / `void`, or native scalar / pointer / `function` / native layout.
    /// - JS-only types (`string`, `number`, `any`, non-layout shapes, …) are rejected.
    /// - Records a full `FnSig` so later call sites get arity/arg checking.
    /// - F08.01: when the compile target is js, hard-error (native-only FFI).
    fn check_extern_function_declaration(
        &mut self,
        name: &draconic_ast::Ident,
        params: &[Param],
        return_type: &Option<TypeAnn>,
        span: Span,
    ) -> Result<(), Diagnostic> {
        // F08.01: `extern "C"` is native-only FFI — reject before signature detail on js.
        if self.host_target == Some(CompileTarget::Js) {
            return Err(extern_unsupported_on_js_diagnostic(&name.name, span));
        }

        let Some(id) = self
            .bound
            .symbols()
            .iter()
            .find(|s| s.span == name.span)
            .map(|s| s.id)
        else {
            return Err(Diagnostic::new(
                format!("extern function binding `{}` must be declared", name.name),
                span,
            ));
        };
        self.symbol_types[id.0 as usize] = Type::Function;

        let mut param_types = Vec::with_capacity(params.len());
        for p in params {
            if p.rest {
                return Err(Diagnostic::new(
                    "extern parameter cannot be a rest parameter".to_string(),
                    p.binding.span(),
                )
                .with_code(codes::INVALID_EXTERN_TYPE)
                .with_help("C ABI parameters are fixed arity; remove `...`"));
            }
            if p.default.is_some() {
                return Err(Diagnostic::new(
                    "extern parameter cannot have a default value".to_string(),
                    p.binding.span(),
                )
                .with_code(codes::INVALID_EXTERN_TYPE)
                .with_help("C ABI parameters have no defaults; remove the initializer"));
            }
            if !matches!(p.binding, BindingPattern::Ident(_)) {
                return Err(Diagnostic::new(
                    "extern parameter must be a simple identifier".to_string(),
                    p.binding.span(),
                )
                .with_code(codes::INVALID_EXTERN_TYPE)
                .with_help("destructuring is not valid in an extern \"C\" signature"));
            }
            let Some(ann) = &p.type_ann else {
                return Err(Diagnostic::new(
                    "extern parameter must have a type annotation".to_string(),
                    p.binding.span(),
                )
                .with_code(codes::INVALID_EXTERN_TYPE)
                .with_help(
                    "annotate with a native scalar, pointer, function, or native layout struct",
                ));
            };
            if is_void_type_ann(ann) {
                return Err(Diagnostic::new(
                    "extern parameter type cannot be `void`".to_string(),
                    ann.span(),
                )
                .with_code(codes::INVALID_EXTERN_TYPE)
                .with_help("`void` is only valid as an extern return type"));
            }
            let ty = self.resolve_extern_abi_type(ann, "parameter")?;
            param_types.push(Some(ty));
        }

        if let Some(ann) = return_type {
            if !is_void_type_ann(ann) {
                let _ = self.resolve_extern_abi_type(ann, "return")?;
            }
        }

        // Always record a signature (including zero-param) so call sites check arity.
        self.fn_sigs[id.0 as usize] = Some(FnSig {
            param_types,
            required: params.len(),
            has_rest: false,
        });
        Ok(())
    }

    /// Resolve a type annotation for an extern ABI position: native scalar, `*T`, `function`, or native layout.
    fn resolve_extern_abi_type(&mut self, ann: &TypeAnn, role: &str) -> Result<Type, Diagnostic> {
        let ty = self.resolve_type_ann(ann)?;
        if matches!(ty, Type::Native(_) | Type::Ptr(_) | Type::Function) {
            return Ok(ty);
        }
        if self.is_native_layout(ty) {
            return Ok(ty);
        }
        let pretty = format_type_full(ty, &self.shapes, &self.unions, &self.intersections);
        Err(Diagnostic::new(
            format!("extern {role} type must be a native scalar, pointer, function, or native layout, got `{pretty}`"),
            ann.span(),
        )
        .with_code(codes::INVALID_EXTERN_TYPE)
        .with_help(
            "use a native type such as `i32`, `i64`, `f64`, `bool`, a pointer like `*u8`, `function`, or a native-field struct",
        ))
    }

    /// Build a resolved call signature from a parameter list (T07.01).
    /// Returns `None` when the function has no annotated parameters, so untyped
    /// (E19-era) functions keep permissive call-site behavior.
    fn fn_sig_from_params(&mut self, params: &[Param]) -> Option<FnSig> {
        let mut param_types = Vec::with_capacity(params.len());
        let mut required = 0usize;
        let mut has_rest = false;
        let mut any_annotated = false;
        for p in params {
            let ann = match &p.type_ann {
                Some(ann) => {
                    any_annotated = true;
                    Some(self.resolve_type_ann(ann).ok()?)
                }
                None => None,
            };
            if p.rest {
                has_rest = true;
            } else if p.default.is_none() && ann.is_some() {
                required += 1;
            }
            param_types.push(ann);
        }
        if !any_annotated {
            return None;
        }
        Some(FnSig {
            param_types,
            required,
            has_rest,
        })
    }

    /// Check a call against a recorded annotated signature (T07.01): reject wrong
    /// arity and non-assignable arguments. Unannotated params are skipped.
    fn check_call_sig(
        &self,
        sig: &FnSig,
        args: &[Arg],
        arg_tys: &[Type],
        span: Span,
    ) -> Result<(), Diagnostic> {
        // Spreads make arity unknowable statically; still type-check the leading args.
        let has_spread = args.iter().any(|a| matches!(a, Arg::Spread(_)));
        if !has_spread {
            if arg_tys.len() < sig.required {
                return Err(Diagnostic::new(
                    format!(
                        "expected at least {} argument(s), got {}",
                        sig.required,
                        arg_tys.len()
                    ),
                    span,
                )
                .with_code(codes::WRONG_ARITY)
                .with_help("pass the required number of arguments for this function"));
            }
            if !sig.has_rest && arg_tys.len() > sig.param_types.len() {
                return Err(Diagnostic::new(
                    format!(
                        "expected at most {} argument(s), got {}",
                        sig.param_types.len(),
                        arg_tys.len()
                    ),
                    span,
                )
                .with_code(codes::WRONG_ARITY)
                .with_help("pass the required number of arguments for this function"));
            }
        }
        for (i, want) in sig.param_types.iter().enumerate() {
            let Some(want) = want else { continue };
            let Some(Arg::Expr(expr)) = args.get(i) else {
                continue;
            };
            let got = arg_tys.get(i).copied().unwrap_or(Type::Any);
            self.require_assignable_expr(got, *want, expr)?;
        }
        Ok(())
    }

    /// T07.04: whether a JS value type has a call/construct surface. Only called for
    /// annotated bindings; `Any`, un-annotated `Object`, and inferred (non-strict)
    /// object-literal/tuple shapes stay permissive (runtime TypeError, E19.13/E19.59).
    fn type_is_callable(&self, ty: Type) -> bool {
        match ty {
            Type::Function | Type::GenericFn(_) | Type::Any => true,
            // Strict (annotated) shapes have no call signatures; inferred shapes stay permissive.
            Type::Shape(id) => !self.shapes.get(id as usize).is_some_and(|s| s.strict),
            _ => false,
        }
    }

    /// True when `ty` is a non-empty shape of native scalar fields (N03 / F03.01).
    fn is_native_layout(&self, ty: Type) -> bool {
        let Type::Shape(id) = ty else {
            return false;
        };
        let Some(shape) = self.shapes.get(id as usize) else {
            return false;
        };
        !shape.props.is_empty()
            && shape
                .props
                .iter()
                .all(|(_, t)| matches!(t, Type::Native(_)))
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
                if arg == Type::BigInt {
                    Ok(Type::BigInt)
                } else if let Type::Native(n) = arg {
                    if n.is_float() && matches!(op, UnaryOp::BitNot) {
                        Err(Diagnostic::new(
                            format!("unary `{op}` cannot be applied to type `{arg}`"),
                            span,
                        ))
                    } else if n.is_float() && matches!(op, UnaryOp::Minus) {
                        Ok(Type::Native(n))
                    } else if n.is_int() {
                        Ok(Type::Native(n))
                    } else {
                        Err(Diagnostic::new(
                            format!("unary `{op}` cannot be applied to type `{arg}`"),
                            span,
                        ))
                    }
                } else if self.is_js_to_number_operand(arg) {
                    // E19.04: ToNumber / ToInt32 on JS values (string, boolean, object, …).
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
            // Await yields the fulfillment value; keep coarse `any` for now.
            UnaryOp::Await => Ok(Type::Any),
            // Yield expression value is the next `.next(arg)` resume value; coarse `any`.
            // `yield*` completion is the inner iterator's final value; coarse `any`.
            UnaryOp::Yield | UnaryOp::YieldStar => Ok(Type::Any),
            // N03.03: `&x` → `*T` when x is native scalar T.
            // F03.01: `&layout` → `*u8` (byte pointer to C ABI struct).
            UnaryOp::Ref => match arg {
                Type::Native(n) => Ok(Type::Ptr(n)),
                Type::Shape(_) if self.is_native_layout(arg) => Ok(Type::Ptr(NativeType::U8)),
                other => Err(Diagnostic::new(
                    format!("cannot take address of type `{other}` (native scalar required)"),
                    span,
                )),
            },
            // N03.03: `*p` → T when p is `*T`.
            UnaryOp::Deref => match arg {
                Type::Ptr(n) => Ok(Type::Native(n)),
                other => Err(Diagnostic::new(
                    format!("cannot dereference type `{other}` (pointer required)"),
                    span,
                )),
            },
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
                if left == Type::BigInt && right == Type::BigInt {
                    Ok(Type::BigInt)
                } else if left == Type::String || right == Type::String {
                    // Including BigInt + string → ToString concat (ECMA-262).
                    if self.is_js_add_side(left) && self.is_js_add_side(right) {
                        Ok(Type::String)
                    } else {
                        Err(Diagnostic::new(
                            format!(
                                "operator `+` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    }
                } else if left == Type::BigInt || right == Type::BigInt {
                    // E19.07: mixed bigint×number/object/any — TypeError (or ToPrimitive) at runtime.
                    if self.is_js_bigint_mixed_operand(left)
                        && self.is_js_bigint_mixed_operand(right)
                    {
                        Ok(Type::Any)
                    } else {
                        Err(Diagnostic::new(
                            format!(
                                "operator `+` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    }
                } else if let Some(n) = self.native_arith_result(left, right, left_expr, right_expr)
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
                if left == Type::BigInt || right == Type::BigInt {
                    // Same-type BigInt (except `>>>`, which always TypeErrors on BigInt).
                    // E19.07: mixed bigint×number/object/any — TypeError is runtime, not compile.
                    if left == Type::BigInt
                        && right == Type::BigInt
                        && !matches!(op, BinaryOp::UShr)
                    {
                        Ok(Type::BigInt)
                    } else if self.is_js_bigint_mixed_operand(left)
                        && self.is_js_bigint_mixed_operand(right)
                    {
                        Ok(Type::Any)
                    } else {
                        Err(Diagnostic::new(
                            format!(
                                "operator `{op}` cannot be applied to types `{left}` and `{right}`"
                            ),
                            span,
                        ))
                    }
                } else if let Some(n) = self.native_arith_result(left, right, left_expr, right_expr)
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
                } else if self.is_js_to_number_operand(left) && self.is_js_to_number_operand(right)
                {
                    // E19.04: ToNumber both sides (string/boolean/null/object/…).
                    Ok(Type::Number)
                } else {
                    Err(Diagnostic::new(
                        format!(
                            "operator `{op}` cannot be applied to types `{left}` and `{right}`"
                        ),
                        span,
                    ))
                }
            }
            BinaryOp::Lt | BinaryOp::LtEq | BinaryOp::Gt | BinaryOp::GtEq => {
                if self
                    .native_arith_result(left, right, left_expr, right_expr)
                    .is_some()
                {
                    Ok(Type::Boolean)
                } else if self.is_js_relational_operand(left)
                    && self.is_js_relational_operand(right)
                {
                    // E19.04: ToPrimitive; mixed primitives/objects/BigInt+Number ok.
                    Ok(Type::Boolean)
                } else {
                    Err(Diagnostic::new(
                        format!(
                            "operator `{op}` cannot be applied to types `{left}` and `{right}`"
                        ),
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

    /// JS values that numeric operators coerce via ToNumber (not BigInt, not native/ptr).
    fn is_js_to_number_operand(&self, ty: Type) -> bool {
        matches!(
            ty,
            Type::Number
                | Type::String
                | Type::Boolean
                | Type::Null
                | Type::Object
                | Type::Shape(_)
                | Type::Function
                | Type::GenericFn(_)
                | Type::Union(_)
                | Type::Intersection(_)
                | Type::TypeParam(_)
                | Type::Any
        )
    }

    /// E19.04 / E19.13: `++`/`--` apply ToNumber (objects via valueOf/toString); BigInt stays BigInt.
    fn check_update_operand(&self, left_ty: Type, span: Span) -> Result<Type, Diagnostic> {
        let ok = left_ty == Type::BigInt
            || matches!(left_ty, Type::Native(n) if n.is_int())
            || self.is_js_to_number_operand(left_ty);
        if !ok {
            return Err(Diagnostic::new(
                format!("update operator cannot be applied to type `{left_ty}`"),
                span,
            ));
        }
        Ok(if matches!(left_ty, Type::Native(_)) {
            left_ty
        } else if left_ty == Type::BigInt {
            Type::BigInt
        } else {
            Type::Number
        })
    }

    /// E19.07: BigInt or JS value that may mix with BigInt at runtime (TypeError / ToPrimitive).
    fn is_js_bigint_mixed_operand(&self, ty: Type) -> bool {
        ty == Type::BigInt || self.is_js_to_number_operand(ty)
    }

    /// Sides legal for binary `+` string/numeric paths (JS values + BigInt; not native/ptr).
    fn is_js_add_side(&self, ty: Type) -> bool {
        self.is_js_bigint_mixed_operand(ty)
    }

    /// Relational comparison operands after ToPrimitive (includes BigInt same-type path separately).
    fn is_js_relational_operand(&self, ty: Type) -> bool {
        self.is_js_to_number_operand(ty) || ty == Type::BigInt
    }

    /// Same native numeric type on both sides, or native + number-literal (contextual).
    fn native_arith_result(
        &self,
        left: Type,
        right: Type,
        left_expr: &Expr,
        right_expr: &Expr,
    ) -> Option<NativeType> {
        match (left, right) {
            (Type::Native(a), Type::Native(b)) if a == b && !a.is_bool() => Some(a),
            (Type::Native(a), Type::Number)
                if !a.is_bool() && Self::is_number_literal_expr(right_expr) =>
            {
                Some(a)
            }
            (Type::Number, Type::Native(b))
                if !b.is_bool() && Self::is_number_literal_expr(left_expr) =>
            {
                Some(b)
            }
            _ => None,
        }
    }

    /// Primitives ToNumber accepts for binary `+` when neither side is string/BigInt/object.
    fn is_primitive_numeric_coercible(&self, ty: Type) -> bool {
        matches!(ty, Type::Number | Type::Boolean | Type::Null)
    }

    fn is_add_operand(&self, ty: Type) -> bool {
        !matches!(ty, Type::Native(_) | Type::Ptr(_))
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
