use draconic_ast::{
    ArrayPatternElement, AssignOp, BinaryOp, BindingPattern, Expr as AstExpr, ObjectPatternProp,
    UnaryOp,
};
use draconic_check::{CheckedProgram, Type};

use crate::lower::LowerCtx;
use crate::{Arg, AssignTarget, Expr, LocalId, Stmt};

use crate::lower_class_element::local_expr;
use crate::lower_expr::lower_expr;
use crate::lower_private::private_member_set;

pub(crate) fn binding_pattern_has_private(pat: &BindingPattern) -> bool {
    match pat {
        BindingPattern::Ident(_) => false,
        BindingPattern::Member(expr) => matches!(
            expr.as_ref(),
            AstExpr::MemberExpression { private: true, .. }
        ),
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Elision => false,
            ArrayPatternElement::Pattern { binding, .. } | ArrayPatternElement::Rest(binding) => {
                binding_pattern_has_private(binding)
            }
        }),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop { binding, .. } | ObjectPatternProp::Rest(binding) => {
                binding_pattern_has_private(binding)
            }
        }),
    }
}

pub(crate) fn array_pattern_has_private(elements: &[ArrayPatternElement]) -> bool {
    elements.iter().any(|el| match el {
        ArrayPatternElement::Elision => false,
        ArrayPatternElement::Pattern { binding, .. } | ArrayPatternElement::Rest(binding) => {
            binding_pattern_has_private(binding)
        }
    })
}

pub(crate) fn object_pattern_has_private(properties: &[ObjectPatternProp]) -> bool {
    properties.iter().any(|p| match p {
        ObjectPatternProp::Prop { binding, .. } | ObjectPatternProp::Rest(binding) => {
            binding_pattern_has_private(binding)
        }
    })
}

pub(crate) fn comma_seq(exprs: Vec<Expr>) -> Expr {
    let mut it = exprs.into_iter();
    let first = it.next().expect("comma_seq non-empty");
    it.fold(first, |left, right| Expr::Binary {
        left: Box::new(left),
        op: BinaryOp::Comma,
        right: Box::new(right),
        ty: Type::Any,
    })
}

pub(crate) fn bind_local(id: LocalId, value: Expr) -> Expr {
    Expr::Assign {
        target: AssignTarget::Local(id),
        op: AssignOp::Eq,
        value: Box::new(value),
        ty: Type::Any,
    }
}

pub(crate) fn undefined_expr() -> Expr {
    Expr::Unary {
        op: UnaryOp::Void,
        arg: Box::new(Expr::Number {
            raw: "0".into(),
            ty: Type::Number,
        }),
        ty: Type::Any,
    }
}

/// PutValue into a binding (private leaves use PrivateFieldSet).
pub(crate) fn lower_put_binding(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    binding: &BindingPattern,
    value: Expr,
    super_class: Option<&AstExpr>,
) -> Expr {
    match binding {
        BindingPattern::Ident(id) => {
            let target = if let Some(local) = checked.bound.resolve(id.span) {
                AssignTarget::Local(ctx.map_class_name(local))
            } else {
                AssignTarget::Name(id.name.clone())
            };
            Expr::Assign {
                target,
                op: AssignOp::Eq,
                value: Box::new(value),
                ty: Type::Any,
            }
        }
        BindingPattern::Member(expr) => match expr.as_ref() {
            AstExpr::MemberExpression {
                object,
                property,
                private: true,
                ..
            } => {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.as_str(),
                    _ => panic!("private member property must be ident"),
                };
                let obj = lower_expr(checked, ctx, object, super_class);
                private_member_set(ctx, fname, obj, value)
            }
            AstExpr::MemberExpression {
                object,
                property,
                computed,
                private: false,
                ..
            } => {
                let prop = if *computed {
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
                Expr::Assign {
                    target: AssignTarget::Member {
                        object: Box::new(lower_expr(checked, ctx, object, super_class)),
                        property: Box::new(prop),
                        computed: *computed,
                    },
                    op: AssignOp::Eq,
                    value: Box::new(value),
                    ty: Type::Any,
                }
            }
            _ => panic!("BindingPattern::Member must wrap MemberExpression"),
        },
        BindingPattern::Array { elements, .. } => {
            lower_array_pattern_assign(checked, ctx, elements, value, super_class)
        }
        BindingPattern::Object { properties, .. } => {
            lower_object_pattern_assign(checked, ctx, properties, value, super_class)
        }
    }
}

/// Keyed/Iterator destructuring: evaluate private lref base before GetV/IteratorStep (E19.82.10).
pub(crate) fn lower_dstr_element_assign<F>(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    binding: &BindingPattern,
    value_after_lref: F,
    super_class: Option<&AstExpr>,
) -> Expr
where
    F: FnOnce(&mut LowerCtx) -> Expr,
{
    match binding {
        BindingPattern::Member(expr) => match expr.as_ref() {
            AstExpr::MemberExpression {
                object,
                property,
                private: true,
                ..
            } => {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.as_str(),
                    _ => panic!("private member property must be ident"),
                };
                let obj_id =
                    ctx.alloc_synthetic_local(format!("__drac_dstr_lref_{fname}"), Type::Any);
                let val_id = ctx.alloc_synthetic_local(format!("__drac_dstr_v_{fname}"), Type::Any);
                let bind_obj = bind_local(obj_id, lower_expr(checked, ctx, object, super_class));
                let bind_val = bind_local(val_id, value_after_lref(ctx));
                let set = private_member_set(ctx, fname, local_expr(obj_id), local_expr(val_id));
                comma_seq(vec![bind_obj, bind_val, set])
            }
            AstExpr::MemberExpression {
                object,
                property,
                computed,
                private: false,
                ..
            } => {
                let obj_id = ctx.alloc_synthetic_local("__drac_dstr_lref_m".into(), Type::Any);
                let mut steps = vec![bind_local(
                    obj_id,
                    lower_expr(checked, ctx, object, super_class),
                )];
                let prop_expr = if *computed {
                    let prop_id = ctx.alloc_synthetic_local("__drac_dstr_pkey".into(), Type::Any);
                    steps.push(bind_local(
                        prop_id,
                        lower_expr(checked, ctx, property, super_class),
                    ));
                    local_expr(prop_id)
                } else {
                    match property.as_ref() {
                        AstExpr::Ident(id) => Expr::String {
                            value: id.name.clone().into(),
                            ty: Type::String,
                        },
                        other => lower_expr(checked, ctx, other, super_class),
                    }
                };
                let val_id = ctx.alloc_synthetic_local("__drac_dstr_vm".into(), Type::Any);
                steps.push(bind_local(val_id, value_after_lref(ctx)));
                steps.push(Expr::Assign {
                    target: AssignTarget::Member {
                        object: Box::new(local_expr(obj_id)),
                        property: Box::new(prop_expr),
                        computed: *computed,
                    },
                    op: AssignOp::Eq,
                    value: Box::new(local_expr(val_id)),
                    ty: Type::Any,
                });
                comma_seq(steps)
            }
            _ => panic!("BindingPattern::Member must wrap MemberExpression"),
        },
        other => {
            let val_id = ctx.alloc_synthetic_local("__drac_dstr_vo".into(), Type::Any);
            comma_seq(vec![
                bind_local(val_id, value_after_lref(ctx)),
                lower_put_binding(checked, ctx, other, local_expr(val_id), super_class),
            ])
        }
    }
}

pub(crate) fn iterator_next_value(ctx: &mut LowerCtx, iter_id: LocalId) -> Expr {
    let n_id = ctx.alloc_synthetic_local("__drac_dstr_n".into(), Type::Any);
    let v_id = ctx.alloc_synthetic_local("__drac_dstr_nv".into(), Type::Any);
    comma_seq(vec![
        bind_local(
            n_id,
            Expr::Call {
                callee: Box::new(Expr::Member {
                    object: Box::new(local_expr(iter_id)),
                    property: Box::new(Expr::String {
                        value: "next".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Function,
                }),
                args: Vec::new(),
                optional: false,
                ty: Type::Any,
            },
        ),
        bind_local(
            v_id,
            Expr::Conditional {
                test: Box::new(Expr::Member {
                    object: Box::new(local_expr(n_id)),
                    property: Box::new(Expr::String {
                        value: "done".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Any,
                }),
                consequent: Box::new(undefined_expr()),
                alternate: Box::new(Expr::Member {
                    object: Box::new(local_expr(n_id)),
                    property: Box::new(Expr::String {
                        value: "value".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Any,
                }),
                ty: Type::Any,
            },
        ),
        local_expr(v_id),
    ])
}

pub(crate) fn with_default(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    raw: Expr,
    default: Option<&AstExpr>,
    super_class: Option<&AstExpr>,
) -> Expr {
    match default {
        None => raw,
        Some(def) => {
            let raw_id = ctx.alloc_synthetic_local("__drac_dstr_raw".into(), Type::Any);
            comma_seq(vec![
                bind_local(raw_id, raw),
                Expr::Conditional {
                    test: Box::new(Expr::Binary {
                        left: Box::new(local_expr(raw_id)),
                        op: BinaryOp::EqEqEq,
                        right: Box::new(undefined_expr()),
                        ty: Type::Boolean,
                    }),
                    consequent: Box::new(lower_expr(checked, ctx, def, super_class)),
                    alternate: Box::new(local_expr(raw_id)),
                    ty: Type::Any,
                },
            ])
        }
    }
}

/// Desugar array destructuring assignment (used when private targets present).
pub(crate) fn lower_array_pattern_assign(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    elements: &[ArrayPatternElement],
    rhs: Expr,
    super_class: Option<&AstExpr>,
) -> Expr {
    let rhs_id = ctx.alloc_synthetic_local("__drac_dstr_rhs".into(), Type::Any);
    let iter_id = ctx.alloc_synthetic_local("__drac_dstr_it".into(), Type::Any);
    let mut steps = vec![
        bind_local(rhs_id, rhs),
        bind_local(
            iter_id,
            Expr::Call {
                callee: Box::new(Expr::Member {
                    object: Box::new(local_expr(rhs_id)),
                    property: Box::new(Expr::Member {
                        object: Box::new(Expr::IdentName {
                            name: "Symbol".into(),
                            ty: Type::Any,
                        }),
                        property: Box::new(Expr::String {
                            value: "iterator".into(),
                            ty: Type::String,
                        }),
                        computed: false,
                        optional: false,
                        ty: Type::Any,
                    }),
                    computed: true,
                    optional: false,
                    ty: Type::Function,
                }),
                args: Vec::new(),
                optional: false,
                ty: Type::Any,
            },
        ),
    ];

    for el in elements {
        match el {
            ArrayPatternElement::Elision => {
                steps.push(iterator_next_value(ctx, iter_id));
            }
            ArrayPatternElement::Pattern { binding, default } => {
                let def = default.as_ref();
                let step = lower_dstr_element_assign(
                    checked,
                    ctx,
                    binding,
                    |ctx| {
                        let raw = iterator_next_value(ctx, iter_id);
                        with_default(checked, ctx, raw, def, super_class)
                    },
                    super_class,
                );
                steps.push(step);
            }
            ArrayPatternElement::Rest(binding) => {
                let rest_id = ctx.alloc_synthetic_local("__drac_dstr_rest".into(), Type::Any);
                steps.push(bind_local(
                    rest_id,
                    Expr::Array {
                        elements: Vec::new(),
                        ty: Type::Any,
                    },
                ));
                let n_id = ctx.alloc_synthetic_local("__drac_dstr_rn".into(), Type::Any);
                let drain = Expr::Call {
                    callee: Box::new(Expr::Function {
                        name: None,
                        params: Vec::new(),
                        body: vec![Stmt::While {
                            test: Expr::Boolean {
                                value: true,
                                ty: Type::Boolean,
                            },
                            body: Box::new(Stmt::Block {
                                body: vec![
                                    Stmt::Expr {
                                        expr: bind_local(
                                            n_id,
                                            Expr::Call {
                                                callee: Box::new(Expr::Member {
                                                    object: Box::new(local_expr(iter_id)),
                                                    property: Box::new(Expr::String {
                                                        value: "next".into(),
                                                        ty: Type::String,
                                                    }),
                                                    computed: false,
                                                    optional: false,
                                                    ty: Type::Function,
                                                }),
                                                args: Vec::new(),
                                                optional: false,
                                                ty: Type::Any,
                                            },
                                        ),
                                    },
                                    Stmt::If {
                                        test: Expr::Member {
                                            object: Box::new(local_expr(n_id)),
                                            property: Box::new(Expr::String {
                                                value: "done".into(),
                                                ty: Type::String,
                                            }),
                                            computed: false,
                                            optional: false,
                                            ty: Type::Any,
                                        },
                                        consequent: Box::new(Stmt::Break { label: None }),
                                        alternate: None,
                                    },
                                    Stmt::Expr {
                                        expr: Expr::Call {
                                            callee: Box::new(Expr::Member {
                                                object: Box::new(local_expr(rest_id)),
                                                property: Box::new(Expr::String {
                                                    value: "push".into(),
                                                    ty: Type::String,
                                                }),
                                                computed: false,
                                                optional: false,
                                                ty: Type::Function,
                                            }),
                                            args: vec![Arg::Expr(Expr::Member {
                                                object: Box::new(local_expr(n_id)),
                                                property: Box::new(Expr::String {
                                                    value: "value".into(),
                                                    ty: Type::String,
                                                }),
                                                computed: false,
                                                optional: false,
                                                ty: Type::Any,
                                            })],
                                            optional: false,
                                            ty: Type::Any,
                                        },
                                    },
                                ],
                            }),
                        }],
                        is_async: false,
                        is_generator: false,
                        is_arrow: true,
                        is_method: false,
                        ty: Type::Function,
                    }),
                    args: Vec::new(),
                    optional: false,
                    ty: Type::Any,
                };
                // lref before collecting rest values for private targets.
                let step = lower_dstr_element_assign(
                    checked,
                    ctx,
                    binding,
                    move |_ctx| comma_seq(vec![drain, local_expr(rest_id)]),
                    super_class,
                );
                steps.push(step);
            }
        }
    }
    steps.push(local_expr(rhs_id));
    comma_seq(steps)
}

pub(crate) fn object_key_to_get_prop(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    key: &draconic_ast::ObjectKey,
    super_class: Option<&AstExpr>,
) -> Expr {
    match key {
        draconic_ast::ObjectKey::Ident(id) => Expr::String {
            value: id.name.clone().into(),
            ty: Type::String,
        },
        draconic_ast::ObjectKey::String(s) => Expr::String {
            value: s.value.clone(),
            ty: Type::String,
        },
        draconic_ast::ObjectKey::Computed(expr) => lower_expr(checked, ctx, expr, super_class),
    }
}

/// Desugar object destructuring assignment (used when private targets present).
pub(crate) fn lower_object_pattern_assign(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    properties: &[ObjectPatternProp],
    rhs: Expr,
    super_class: Option<&AstExpr>,
) -> Expr {
    let rhs_id = ctx.alloc_synthetic_local("__drac_dstr_rhs".into(), Type::Any);
    let mut steps = vec![bind_local(rhs_id, rhs)];
    let mut excluded_keys: Vec<LocalId> = Vec::new();

    for p in properties {
        match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                let key_id = ctx.alloc_synthetic_local("__drac_dstr_key".into(), Type::Any);
                steps.push(bind_local(
                    key_id,
                    object_key_to_get_prop(checked, ctx, key, super_class),
                ));
                excluded_keys.push(key_id);
                let def = default.as_ref();
                let step = lower_dstr_element_assign(
                    checked,
                    ctx,
                    binding,
                    |ctx| {
                        let get = Expr::Member {
                            object: Box::new(local_expr(rhs_id)),
                            property: Box::new(local_expr(key_id)),
                            computed: true,
                            optional: false,
                            ty: Type::Any,
                        };
                        with_default(checked, ctx, get, def, super_class)
                    },
                    super_class,
                );
                steps.push(step);
            }
            ObjectPatternProp::Rest(binding) => {
                let rest_id = ctx.alloc_synthetic_local("__drac_dstr_orest".into(), Type::Any);
                // Build rest via Object.assign after lref for private targets.
                let step = lower_dstr_element_assign(
                    checked,
                    ctx,
                    binding,
                    |_ctx| {
                        let mut rest_steps = vec![bind_local(
                            rest_id,
                            Expr::Object {
                                properties: Vec::new(),
                                ty: Type::Any,
                            },
                        )];
                        rest_steps.push(Expr::Call {
                            callee: Box::new(Expr::Member {
                                object: Box::new(Expr::IdentName {
                                    name: "Object".into(),
                                    ty: Type::Any,
                                }),
                                property: Box::new(Expr::String {
                                    value: "assign".into(),
                                    ty: Type::String,
                                }),
                                computed: false,
                                optional: false,
                                ty: Type::Function,
                            }),
                            args: vec![
                                Arg::Expr(local_expr(rest_id)),
                                Arg::Expr(local_expr(rhs_id)),
                            ],
                            optional: false,
                            ty: Type::Any,
                        });
                        for kid in &excluded_keys {
                            rest_steps.push(Expr::Unary {
                                op: UnaryOp::Delete,
                                arg: Box::new(Expr::Member {
                                    object: Box::new(local_expr(rest_id)),
                                    property: Box::new(local_expr(*kid)),
                                    computed: true,
                                    optional: false,
                                    ty: Type::Any,
                                }),
                                ty: Type::Boolean,
                            });
                        }
                        rest_steps.push(local_expr(rest_id));
                        comma_seq(rest_steps)
                    },
                    super_class,
                );
                steps.push(step);
            }
        }
    }
    steps.push(local_expr(rhs_id));
    comma_seq(steps)
}
