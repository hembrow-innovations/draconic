use std::collections::HashMap;

use draconic_ast::{Expr, ObjectKey, Param, TypeAnn, UnaryOp};
use draconic_diagnostics::{codes, Diagnostic, Span};

use super::Checker;
use crate::{format_type_full, IntersectionType, NativeType, ObjectShape, Type, UnionType};

impl Checker {
    pub(crate) fn check_object_key(&mut self, key: &ObjectKey) -> Result<(), Diagnostic> {
        match key {
            ObjectKey::Ident(_) | ObjectKey::String(_) => Ok(()),
            ObjectKey::Computed(expr) => {
                self.check_expr(expr)?;
                Ok(())
            }
        }
    }

    pub(crate) fn check_params(&mut self, params: &[Param]) -> Result<(), Diagnostic> {
        for (i, p) in params.iter().enumerate() {
            if self.typecheck && p.rest {
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
            let ann_ty = if self.typecheck {
                match &p.type_ann {
                    Some(ann) => Some(self.resolve_type_ann(ann)?),
                    None => None,
                }
            } else {
                None
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
    pub(crate) fn check_params_await_yield(
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

    pub(crate) fn intern_shape(&mut self, props: Vec<(String, Type)>, strict: bool) -> Type {
        let id = self.shapes.len() as u32;
        self.shapes.push(ObjectShape { props, strict });
        Type::Shape(id)
    }

    pub(crate) fn intern_union(&mut self, mut members: Vec<Type>) -> Type {
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

    pub(crate) fn intern_intersection(&mut self, mut members: Vec<Type>) -> Type {
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

    pub(crate) fn collect_union_members(&self, ty: Type, out: &mut Vec<Type>) {
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

    pub(crate) fn collect_intersection_members(&self, ty: Type, out: &mut Vec<Type>) {
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

    pub(crate) fn union_members(&self, ty: Type) -> Vec<Type> {
        let mut out = Vec::new();
        self.collect_union_members(ty, &mut out);
        out
    }

    pub(crate) fn prop_type(&self, obj: Type, name: &str) -> Option<Type> {
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
    pub(crate) fn member_prop_type(
        &self,
        obj: Type,
        name: &str,
        span: Span,
    ) -> Result<Type, Diagnostic> {
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

    pub(crate) fn unknown_property_diagnostic(
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
    pub(crate) fn is_strict_member(&self, ty: Type) -> bool {
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
    pub(crate) fn resolve_type_ann(&mut self, ann: &TypeAnn) -> Result<Type, Diagnostic> {
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
    pub(crate) fn instantiate_generic_call(
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
    pub(crate) fn unify_infer(
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

    pub(crate) fn apply_subst(&self, ty: Type, subst: &HashMap<u32, Type>) -> Type {
        match ty {
            Type::TypeParam(id) => subst.get(&id).copied().unwrap_or(ty),
            other => other,
        }
    }

    /// Number (or ±number) literal expression — may contextually type as a native numeric.
    pub(crate) fn is_number_literal_expr(expr: &Expr) -> bool {
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
    pub(crate) fn const_index_key(expr: &Expr) -> Option<String> {
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

    pub(crate) fn number_literal_ok_for_native(to: Type) -> bool {
        matches!(to, Type::Native(n) if !n.is_bool())
    }

    /// Explicit dual-worlds boundary (`as`): JS `number` ↔ unboxed native numeric (T06).
    pub(crate) fn is_dual_world_boundary(from: Type, to: Type) -> bool {
        from.is_dual_world_boundary(to)
    }
}
