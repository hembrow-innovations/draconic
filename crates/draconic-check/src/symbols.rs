use std::collections::HashMap;

use draconic_ast::{BindingKind, Program};
use draconic_diagnostics::Span;

use super::types::{
    format_type_full, GenericFnSig, IntersectionType, ObjectShape, Type, UnionType,
};

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

/// Program after scope analysis and identifier resolution.
#[derive(Debug)]
pub struct BoundProgram {
    pub program: Program,
    pub(crate) symbols: Vec<Symbol>,
    /// Use-site identifier span → declared symbol.
    pub(crate) resolutions: HashMap<Span, SymbolId>,
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

/// Bound program with inferred / checked types.
#[derive(Debug)]
pub struct CheckedProgram {
    pub bound: BoundProgram,
    /// Declaration symbol → type.
    pub(crate) symbol_types: Vec<Type>,
    /// Expression span → type.
    pub(crate) expr_types: HashMap<Span, Type>,
    /// Structural object shapes referenced by `Type::Shape`.
    pub(crate) shapes: Vec<ObjectShape>,
    /// Named type aliases (`type Pair = { … }`) for ABI re-resolution (F03.02).
    pub(crate) type_aliases: HashMap<String, Type>,
    /// Union members referenced by `Type::Union`.
    pub(crate) unions: Vec<UnionType>,
    /// Intersection members referenced by `Type::Intersection`.
    pub(crate) intersections: Vec<IntersectionType>,
    /// Generic function signatures referenced by `Type::GenericFn`.
    pub(crate) generic_fns: Vec<GenericFnSig>,
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

#[cfg(test)]
mod tests {
    use super::{BoundProgram, Symbol};
    use crate::bind;
    use draconic_ast::BindingKind;
    use draconic_diagnostics::Span;
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
}
