use draconic_ast::BindingKind;
use draconic_check::{ObjectShape, SymbolId as LocalId, Type};
use draconic_diagnostics::Span;

use crate::Stmt;

/// Entry named export after flatten: public name plus the local that holds the value.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NamedExport {
    pub public_name: String,
    pub local_name: String,
}

/// Top-level IR unit both backends consume.
#[derive(Debug, Clone, PartialEq)]
pub struct Module {
    pub locals: Vec<Local>,
    pub body: Vec<Stmt>,
    /// Original source span for each top-level `body` entry (same length as `body`).
    /// Expanded lowerings (e.g. class → several stmts) share the originating AST span.
    pub body_spans: Vec<Span>,
    /// Structural object shapes referenced by `Type::Shape` (N03 native layouts).
    pub shapes: Vec<ObjectShape>,
    /// Program declared `extern "C"` (native-only FFI). JS backend must hard-error (F08.01).
    /// When true, `body` contains one or more `Stmt::ExternFunction` ABI decls (F06.03).
    pub has_extern_ffi: bool,
    /// Entry named exports (public name → local after flatten). Empty for Scripts.
    /// LLVM ignores this. Default JS emit does not print `export`.
    pub named_exports: Vec<NamedExport>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Local {
    pub id: LocalId,
    pub name: String,
    pub ty: Type,
    pub kind: BindingKind,
}
