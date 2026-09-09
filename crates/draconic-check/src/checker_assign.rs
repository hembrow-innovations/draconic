use std::collections::HashMap;

use draconic_ast::{ArrayElement, BinaryOp, Expr, ObjectKey, ObjectProp, UnaryOp};
use draconic_diagnostics::{codes, Diagnostic, Span};

use super::checker::{expr_span_of, Checker};
use super::{format_type_full, NativeType, ObjectShape, SymbolId, Type};

impl Checker {
    /// Assignability with contextual typing of numeric literals to native types (T05),
    /// object literals to native-layout shapes (N03.01), and array literals to tuple
    /// layouts (N03.02).
    pub(crate) fn require_assignable_expr(
        &self,
        from: Type,
        to: Type,
        from_expr: &Expr,
    ) -> Result<(), Diagnostic> {
        if !self.typecheck {
            return Ok(());
        }
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
    pub(crate) fn object_literal_contextually_assignable(
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
    pub(crate) fn excess_prop_diag(&self, from_expr: &Expr, to: Type) -> Option<Diagnostic> {
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
    pub(crate) fn object_literal_excess_diag(
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
    pub(crate) fn array_literal_contextually_assignable(
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

    pub(crate) fn expr_contextually_assignable_to(&self, val: &Expr, want: Type) -> bool {
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
    pub(crate) fn is_assignable(&self, from: Type, to: Type) -> bool {
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

    pub(crate) fn require_assignable(
        &self,
        from: Type,
        to: Type,
        span: Span,
    ) -> Result<(), Diagnostic> {
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
    pub(crate) fn typeof_tag_type(tag: &str) -> Option<Type> {
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
    pub(crate) fn matches_typeof_tag(&self, ty: Type, tag: &str) -> bool {
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
    pub(crate) fn filter_by_typeof(&mut self, ty: Type, tag: &str, positive: bool) -> Type {
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
    pub(crate) fn typeof_narrow_facts(
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
        let Some(sym) = self.resolve_span(ident.span) else {
            return empty;
        };
        let cur = self.symbol_types[sym.0 as usize];
        let then_ty = self.filter_by_typeof(cur, tag.as_str(), positive);
        let else_ty = self.filter_by_typeof(cur, tag.as_str(), !positive);
        (vec![(sym, then_ty)], vec![(sym, else_ty)])
    }

    pub(crate) fn with_narrows<R>(
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

    pub(crate) fn record(&mut self, span: Span, ty: Type) {
        if self.typecheck {
            self.expr_types.insert(span, ty);
        }
    }

    /// True when `expr` is an identifier resolving to the host global `name`.
    pub(crate) fn is_global_ident(&self, expr: &Expr, name: &str) -> bool {
        let Expr::Ident(id) = expr else {
            return false;
        };
        let Some(sym_id) = self.resolve_span(id.span) else {
            return false;
        };
        let sym = self.symbol(sym_id);
        sym.name == name && sym.span == Span::dummy()
    }
}
