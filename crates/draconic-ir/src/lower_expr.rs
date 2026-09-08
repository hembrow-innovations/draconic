use draconic_ast::{
    Arg as AstArg, ArrayPatternElement, AssignOp, BindingPattern, Expr as AstExpr,
    ObjectPatternProp, UnaryOp,
};
use draconic_check::{CheckedProgram, Type};
use draconic_diagnostics::Span;

use crate::lower::LowerCtx;
use crate::{
    Arg, ArrayPatternEl, AssignTarget, Expr, ObjectPatternEl, ObjectPropKey, Param, Pattern,
    UpdateTarget,
};

use crate::lower_class_element::{assert_derived_this, lower_super_prop_assign};
use crate::lower_expr_object::lower_expr_object;
use crate::lower_pattern::{
    array_pattern_has_private, lower_array_pattern_assign, lower_object_pattern_assign,
    object_pattern_has_private,
};
use crate::lower_private::{
    iife_bind_arg, lower_private_assign, lower_private_update, private_in_check,
    resolve_private_brand,
};

pub(crate) fn lower_arg(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    arg: &AstArg,
    super_class: Option<&AstExpr>,
) -> Arg {
    match arg {
        AstArg::Expr(e) => Arg::Expr(lower_expr(checked, ctx, e, super_class)),
        AstArg::Spread(e) => Arg::Spread(lower_expr(checked, ctx, e, super_class)),
    }
}

pub(crate) fn lower_expr(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    expr: &AstExpr,
    super_class: Option<&AstExpr>,
) -> Expr {
    lower_expr_hint(checked, ctx, expr, super_class, None)
}

/// Like `lower_expr`, but `name_hint` drives NamedEvaluation for anonymous
/// class expressions (`let cls = class {}` → `.name === "cls"`) (E19.31).
pub(crate) fn lower_expr_hint(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    expr: &AstExpr,
    super_class: Option<&AstExpr>,
    name_hint: Option<&str>,
) -> Expr {
    match expr {
        AstExpr::Paren { expr: inner, .. } => {
            lower_expr_hint(checked, ctx, inner, super_class, name_hint)
        }
        // Dual-worlds `as` is a type-level boundary only (T06); erase at IR.
        AstExpr::As { expr: inner, .. } => {
            lower_expr_hint(checked, ctx, inner, super_class, name_hint)
        }
        AstExpr::ArrayPattern { .. } => {
            panic!("array pattern must only appear as assignment target")
        }
        AstExpr::ObjectPattern { .. } => {
            panic!("object pattern must only appear as assignment target")
        }
        AstExpr::Ident(id) => {
            let ty = expr_ty(checked, id.span);
            if let Some(sym) = checked.bound.resolve(id.span) {
                Expr::Local {
                    id: ctx.map_class_name(sym),
                    ty,
                }
            } else {
                Expr::IdentName {
                    name: id.name.clone(),
                    ty,
                }
            }
        }
        AstExpr::Number(n) => Expr::Number {
            raw: n.raw.clone(),
            ty: expr_ty(checked, n.span),
        },
        AstExpr::BigInt(n) => Expr::BigInt {
            raw: n.raw.clone(),
            ty: expr_ty(checked, n.span),
        },
        AstExpr::String(s) => Expr::String {
            value: s.value.clone(),
            ty: expr_ty(checked, s.span),
        },
        AstExpr::RegExp {
            pattern,
            flags,
            span,
        } => Expr::RegExp {
            pattern: pattern.clone(),
            flags: flags.clone(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::TemplateLiteral {
            quasis,
            expressions,
            span,
        } => Expr::Template {
            quasis: quasis.iter().map(|q| q.cooked.clone()).collect(),
            expressions: expressions
                .iter()
                .map(|e| lower_expr(checked, ctx, e, super_class))
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            span,
        } => Expr::TaggedTemplate {
            tag: Box::new(lower_expr(checked, ctx, tag, super_class)),
            quasis: quasis.iter().map(|q| q.cooked.clone()).collect(),
            expressions: expressions
                .iter()
                .map(|e| lower_expr(checked, ctx, e, super_class))
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Boolean { value, span } => Expr::Boolean {
            value: *value,
            ty: expr_ty(checked, *span),
        },
        AstExpr::Null { span } => Expr::Null {
            ty: expr_ty(checked, *span),
        },
        AstExpr::This { span } => {
            if let Some(this_id) = ctx.derived_this {
                // Derived ctor: ES this TDZ until super() (E19.82.03).
                let _ = span;
                assert_derived_this(this_id)
            } else {
                Expr::This {
                    ty: expr_ty(checked, *span),
                }
            }
        }
        AstExpr::NewTarget { span } => Expr::NewTarget {
            ty: expr_ty(checked, *span),
        },
        AstExpr::ImportMeta { span } => Expr::ImportMeta {
            ty: expr_ty(checked, *span),
        },
        AstExpr::ImportCall {
            phase,
            source,
            options,
            span,
        } => Expr::ImportCall {
            phase: *phase,
            source: Box::new(lower_expr(checked, ctx, source, super_class)),
            options: options
                .as_ref()
                .map(|o| Box::new(lower_expr(checked, ctx, o, super_class))),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Super { span } => {
            // Keep bare `super` for JS home-object emit; never panic (E19.34).
            // Invalid SuperCall/SuperProperty sites are early errors in parser/check.
            Expr::Super {
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::Unary { op, arg, span } => Expr::Unary {
            op: *op,
            arg: Box::new(lower_expr(checked, ctx, arg, super_class)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Binary {
            left,
            op,
            right,
            span,
        } => Expr::Binary {
            left: Box::new(lower_expr(checked, ctx, left, super_class)),
            op: *op,
            right: Box::new(lower_expr(checked, ctx, right, super_class)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::PrivateIn { name, object, span } => {
            let brand = resolve_private_brand(ctx, &name.name);
            let obj = lower_expr(checked, ctx, object, super_class);
            let _ = span;
            iife_bind_arg(ctx, obj, |o| private_in_check(brand, o))
        }
        AstExpr::Conditional {
            test,
            consequent,
            alternate,
            span,
        } => Expr::Conditional {
            test: Box::new(lower_expr(checked, ctx, test, super_class)),
            consequent: Box::new(lower_expr(checked, ctx, consequent, super_class)),
            alternate: Box::new(lower_expr(checked, ctx, alternate, super_class)),
            ty: expr_ty(checked, *span),
        },
        AstExpr::Assign {
            target,
            op,
            value,
            span,
        } => {
            // E19.60: peel cover parentheses so `(id) = v` lowers as a simple target.
            let mut core = target.as_ref();
            while let AstExpr::Paren { expr, .. } = core {
                core = expr.as_ref();
            }
            // Private field/accessor assign: `obj.#x = v` / compound / logical (E19.36).
            if let AstExpr::MemberExpression {
                object,
                property,
                private: true,
                ..
            } = core
            {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.as_str(),
                    _ => panic!("private member property must be ident"),
                };
                return lower_private_assign(checked, ctx, fname, object, *op, value, super_class);
            }
            // E19.82.10: destructuring assign into private fields — desugar so
            // lref-before-GetV order and PrivateFieldSet apply (not native `#` emit).
            if matches!(op, AssignOp::Eq) {
                if let AstExpr::ArrayPattern { elements, .. } = core {
                    if array_pattern_has_private(elements) {
                        let rhs = lower_expr(checked, ctx, value, super_class);
                        return lower_array_pattern_assign(
                            checked,
                            ctx,
                            elements,
                            rhs,
                            super_class,
                        );
                    }
                }
                if let AstExpr::ObjectPattern { properties, .. } = core {
                    if object_pattern_has_private(properties) {
                        let rhs = lower_expr(checked, ctx, value, super_class);
                        return lower_object_pattern_assign(
                            checked,
                            ctx,
                            properties,
                            rhs,
                            super_class,
                        );
                    }
                }
            }
            let assign_name_hint = match core {
                AstExpr::Ident(id) if matches!(op, AssignOp::Eq) => Some(id.name.as_str()),
                _ => None,
            };
            let target = match core {
                AstExpr::Ident(id) => {
                    if let Some(local) = checked.bound.resolve(id.span) {
                        AssignTarget::Local(ctx.map_class_name(local))
                    } else {
                        AssignTarget::Name(id.name.clone())
                    }
                }
                AstExpr::MemberExpression {
                    object,
                    property,
                    computed,
                    private: false,
                    ..
                } => {
                    let property = if *computed {
                        lower_expr(checked, ctx, property, super_class)
                    } else {
                        match property.as_ref() {
                            AstExpr::Ident(id) => Expr::String {
                                value: id.name.clone().into(),
                                ty: Type::String,
                            },
                            other => lower_expr(checked, ctx, other, super_class),
                        }
                    };
                    // `super.x = v` in a derived constructor — SetSuperProperty with the
                    // derived `this` as receiver (E19.86); `super` cannot be emitted in a
                    // plain constructor function.
                    if matches!(object.as_ref(), AstExpr::Super { .. }) && ctx.derived_super.is_some()
                    {
                        return lower_super_prop_assign(
                            checked,
                            ctx,
                            property,
                            *op,
                            value,
                            super_class,
                        );
                    }
                    AssignTarget::Member {
                        object: Box::new(lower_expr(checked, ctx, object, super_class)),
                        property: Box::new(property),
                        computed: *computed,
                    }
                }
                AstExpr::Unary {
                    op: UnaryOp::Deref,
                    arg,
                    ..
                } => AssignTarget::Deref(Box::new(lower_expr(checked, ctx, arg, super_class))),
                AstExpr::ArrayPattern { elements, .. } => AssignTarget::ArrayPattern {
                    elements: lower_array_pattern_els(checked, ctx, elements),
                },
                AstExpr::ObjectPattern { properties, .. } => AssignTarget::ObjectPattern {
                    properties: lower_object_pattern_props(checked, ctx, properties),
                },
                _ => panic!(
                    "assign target must be ident, member, deref, array pattern, or object pattern after check"
                ),
            };
            Expr::Assign {
                target,
                op: *op,
                value: Box::new(lower_expr_hint(
                    checked,
                    ctx,
                    value,
                    super_class,
                    assign_name_hint,
                )),
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::Update {
            op,
            arg,
            prefix,
            span,
        } => {
            // E19.60: peel cover parentheses so `(id)++` lowers as a simple target.
            let mut core = arg.as_ref();
            while let AstExpr::Paren { expr, .. } = core {
                core = expr.as_ref();
            }
            if let AstExpr::MemberExpression {
                object,
                property,
                private: true,
                ..
            } = core
            {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.as_str(),
                    _ => panic!("private member property must be ident"),
                };
                return lower_private_update(
                    checked,
                    ctx,
                    fname,
                    object,
                    *op,
                    *prefix,
                    super_class,
                );
            }
            let target = match core {
                AstExpr::Ident(id) => {
                    if let Some(local) = checked.bound.resolve(id.span) {
                        UpdateTarget::Local(ctx.map_class_name(local))
                    } else {
                        UpdateTarget::Name(id.name.clone())
                    }
                }
                AstExpr::MemberExpression {
                    object,
                    property,
                    computed,
                    private: false,
                    ..
                } => {
                    let property = if *computed {
                        lower_expr(checked, ctx, property, super_class)
                    } else {
                        match property.as_ref() {
                            AstExpr::Ident(id) => Expr::String {
                                value: id.name.clone().into(),
                                ty: Type::String,
                            },
                            other => lower_expr(checked, ctx, other, super_class),
                        }
                    };
                    UpdateTarget::Member {
                        object: Box::new(lower_expr(checked, ctx, object, super_class)),
                        property: Box::new(property),
                        computed: *computed,
                    }
                }
                _ => panic!("update target must be ident or member after check"),
            };
            Expr::Update {
                op: *op,
                target,
                prefix: *prefix,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::Call { .. }
        | AstExpr::New { .. }
        | AstExpr::FunctionExpression { .. }
        | AstExpr::ClassExpression { .. }
        | AstExpr::ArrowFunction { .. }
        | AstExpr::ObjectExpression { .. }
        | AstExpr::ArrayExpression { .. }
        | AstExpr::MemberExpression { .. } => {
            lower_expr_object(checked, ctx, expr, super_class, name_hint)
        }
    }
}

pub(crate) fn lower_params(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    params: &[draconic_ast::Param],
    super_class: Option<&AstExpr>,
) -> Vec<Param> {
    let mut out = Vec::with_capacity(params.len());
    for p in params {
        let hint = single_name_binding_hint(&p.binding);
        out.push(Param {
            pattern: lower_binding_pattern(checked, ctx, &p.binding),
            default: p
                .default
                .as_ref()
                .map(|e| lower_expr_hint(checked, ctx, e, super_class, hint)),
            rest: p.rest,
        });
    }
    out
}

/// BindingIdentifier name for NamedEvaluation (SingleNameBinding only).
pub(crate) fn single_name_binding_hint(pat: &BindingPattern) -> Option<&str> {
    match pat {
        BindingPattern::Ident(id) => Some(id.name.as_str()),
        _ => None,
    }
}

pub(crate) fn expr_ty(checked: &CheckedProgram, span: Span) -> Type {
    // Missing types → Any: normal Programs are fully typed; direct-eval fragments
    // inlined for private access (E19.82.08) are parsed outside the CheckedProgram.
    checked.type_of_expr(span).unwrap_or(Type::Any)
}

pub(crate) fn lower_array_pattern_els(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    elements: &[ArrayPatternElement],
) -> Vec<ArrayPatternEl> {
    let mut out = Vec::with_capacity(elements.len());
    for el in elements {
        out.push(match el {
            ArrayPatternElement::Elision => ArrayPatternEl::Elision,
            ArrayPatternElement::Pattern { binding, default } => {
                let hint = single_name_binding_hint(binding);
                ArrayPatternEl::Pattern {
                    binding: lower_binding_pattern(checked, ctx, binding),
                    default: default
                        .as_ref()
                        .map(|d| lower_expr_hint(checked, ctx, d, None, hint)),
                }
            }
            ArrayPatternElement::Rest(binding) => {
                ArrayPatternEl::Rest(lower_binding_pattern(checked, ctx, binding))
            }
        });
    }
    out
}

pub(crate) fn lower_object_pattern_props(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    properties: &[ObjectPatternProp],
) -> Vec<ObjectPatternEl> {
    let mut out = Vec::with_capacity(properties.len());
    for p in properties {
        out.push(match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                shorthand,
                default,
                ..
            } => {
                let hint = single_name_binding_hint(binding);
                ObjectPatternEl::Prop {
                    key: match key {
                        draconic_ast::ObjectKey::Ident(id) => {
                            ObjectPropKey::Static(id.name.clone().into())
                        }
                        draconic_ast::ObjectKey::String(s) => {
                            ObjectPropKey::Static(s.value.clone())
                        }
                        draconic_ast::ObjectKey::Computed(expr) => {
                            ObjectPropKey::Computed(lower_expr(checked, ctx, expr, None))
                        }
                    },
                    binding: lower_binding_pattern(checked, ctx, binding),
                    shorthand: *shorthand,
                    default: default
                        .as_ref()
                        .map(|d| lower_expr_hint(checked, ctx, d, None, hint)),
                }
            }
            ObjectPatternProp::Rest(binding) => {
                ObjectPatternEl::Rest(lower_binding_pattern(checked, ctx, binding))
            }
        });
    }
    out
}

pub(crate) fn lower_binding_pattern(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    pat: &BindingPattern,
) -> Pattern {
    match pat {
        BindingPattern::Ident(id) => {
            if let Some(local) = checked
                .bound
                .symbols()
                .iter()
                .find(|s| s.span == id.span)
                .map(|s| s.id)
                .or_else(|| checked.bound.resolve(id.span))
            {
                Pattern::Local(local)
            } else {
                Pattern::Name(id.name.clone())
            }
        }
        BindingPattern::Member(expr) => match expr.as_ref() {
            AstExpr::MemberExpression {
                object,
                property,
                computed,
                private: false,
                ..
            } => {
                let property = if *computed {
                    lower_expr(checked, ctx, property, None)
                } else {
                    match property.as_ref() {
                        AstExpr::Ident(id) => Expr::String {
                            value: id.name.clone().into(),
                            ty: Type::String,
                        },
                        other => lower_expr(checked, ctx, other, None),
                    }
                };
                Pattern::Member {
                    object: Box::new(lower_expr(checked, ctx, object, None)),
                    property: Box::new(property),
                    computed: *computed,
                }
            }
            _ => panic!("BindingPattern::Member must wrap MemberExpression"),
        },
        BindingPattern::Array { elements, .. } => {
            Pattern::Array(lower_array_pattern_els(checked, ctx, elements))
        }
        BindingPattern::Object { properties, .. } => {
            Pattern::Object(lower_object_pattern_props(checked, ctx, properties))
        }
    }
}
