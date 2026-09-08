use draconic_ast::{
    Arg as AstArg, ArrayElement as AstArrayElement, BinaryOp, Expr as AstExpr,
    ObjectProp as AstObjectProp,
};
use draconic_check::{CheckedProgram, Type};

use crate::lower::LowerCtx;
use crate::{Arg, ArrayElement, Expr, ObjectProp, ObjectPropKey, Param, Pattern, Stmt};

use crate::lower::lower_fn_body;
use crate::lower_class::lower_class_expression;
use crate::lower_class_element::{assert_derived_this, derived_super_call_expr, local_expr};
use crate::lower_class_prop::member_prop;
use crate::lower_eval::{
    ast_expr_is_eval_ident, ast_string_literal_value, field_init_eval_arguments_error,
    source_contains_arguments_ident, try_lower_direct_eval_private,
};
use crate::lower_expr::{expr_ty, lower_arg, lower_expr, lower_expr_hint, lower_params};
use crate::lower_private::{
    optional_private_chain, private_access_checked, private_member_get, resolve_private_brand,
};

pub(crate) fn lower_expr_object(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    expr: &AstExpr,
    super_class: Option<&AstExpr>,
    name_hint: Option<&str>,
) -> Expr {
    match expr {
        AstExpr::Call {
            callee,
            args,
            optional,
            span,
        } => {
            // `super(args)` → derived ctor: Reflect.construct; object method: keep `super(...)`
            // (early SyntaxError for SuperCall in object methods is deferred to check/parser).
            if matches!(callee.as_ref(), AstExpr::Super { .. }) {
                // Derived constructor: bind this via Reflect.construct + field inits (E19.82.03).
                if ctx.derived_this.is_some() {
                    let call_args: Vec<Arg> = args
                        .iter()
                        .map(|a| lower_arg(checked, ctx, a, super_class))
                        .collect();
                    return derived_super_call_expr(ctx, call_args);
                }
                // Object methods and missing-extends: keep `super(...)` for JS emit (E19.34).
                if super_class.is_none() {
                    return Expr::Call {
                        callee: Box::new(Expr::Super {
                            ty: expr_ty(checked, *span),
                        }),
                        args: args
                            .iter()
                            .map(|a| lower_arg(checked, ctx, a, super_class))
                            .collect(),
                        optional: false,
                        ty: expr_ty(checked, *span),
                    };
                }
                let parent_ast = super_class.expect("super_class present");
                let parent = lower_expr(checked, ctx, parent_ast, None);
                let call_member = Expr::Member {
                    object: Box::new(parent),
                    property: Box::new(Expr::String {
                        value: "call".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Function,
                };
                let mut call_args = Vec::with_capacity(args.len() + 1);
                call_args.push(Arg::Expr(Expr::This { ty: Type::Any }));
                for a in args {
                    call_args.push(lower_arg(checked, ctx, a, super_class));
                }
                return Expr::Call {
                    callee: Box::new(call_member),
                    args: call_args,
                    optional: false,
                    ty: expr_ty(checked, *span),
                };
            }
            // Direct eval string literal handling (E19.82.06 / E19.82.08).
            // SuperProperty/new.target work via method HomeObject; SuperCall is SyntaxError
            // natively inside methods. Nested fn/arrow bodies clear `in_field_init`.
            if ast_expr_is_eval_ident(callee) {
                if let Some(first) = args.first() {
                    if let AstArg::Expr(arg_expr) = first {
                        if let Some(src) = ast_string_literal_value(arg_expr) {
                            // Field-init: ContainsArguments early error (E19.82.06).
                            if ctx.in_field_init && source_contains_arguments_ident(&src) {
                                return field_init_eval_arguments_error();
                            }
                            // Private names desugared to WeakMap/brand — rewrite eval
                            // source so `#m` resolves in the current private env (E19.82.08).
                            if let Some(lowered) =
                                try_lower_direct_eval_private(checked, ctx, &src, super_class)
                            {
                                return lowered;
                            }
                        }
                    }
                }
            }
            // `super.m(args)` → `Parent.prototype.m.call(this, ...args)`
            if let AstExpr::MemberExpression {
                object,
                property,
                computed,
                private,
                optional: member_optional,
                ..
            } = callee.as_ref()
            {
                if matches!(object.as_ref(), AstExpr::Super { .. }) {
                    // Object/class methods + field-init bodies: keep `super.m(...)` for
                    // JS home-object emit (E19.34 / E19.82). Base ctor: Object.prototype.
                    if super_class.is_none() && ctx.derived_super.is_none() {
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
                        if ctx.object_super || ctx.in_field_init {
                            let method = Expr::Member {
                                object: Box::new(Expr::Super { ty: Type::Any }),
                                property: Box::new(prop),
                                computed: *computed,
                                optional: false,
                                ty: Type::Function,
                            };
                            return Expr::Call {
                                callee: Box::new(method),
                                args: args
                                    .iter()
                                    .map(|a| lower_arg(checked, ctx, a, super_class))
                                    .collect(),
                                optional: false,
                                ty: expr_ty(checked, *span),
                            };
                        }
                        let method = Expr::Member {
                            object: Box::new(member_prop(
                                Expr::IdentName {
                                    name: "Object".into(),
                                    ty: Type::Function,
                                },
                                "prototype",
                                Type::Any,
                            )),
                            property: Box::new(prop),
                            computed: *computed,
                            optional: false,
                            ty: Type::Function,
                        };
                        let call_member = Expr::Member {
                            object: Box::new(method),
                            property: Box::new(Expr::String {
                                value: "call".into(),
                                ty: Type::String,
                            }),
                            computed: false,
                            optional: false,
                            ty: Type::Function,
                        };
                        let mut call_args = Vec::with_capacity(args.len() + 1);
                        call_args.push(Arg::Expr(Expr::This { ty: Type::Any }));
                        for a in args {
                            call_args.push(lower_arg(checked, ctx, a, super_class));
                        }
                        return Expr::Call {
                            callee: Box::new(call_member),
                            args: call_args,
                            optional: false,
                            ty: expr_ty(checked, *span),
                        };
                    }
                    let parent = if let Some(sid) = ctx.derived_super {
                        local_expr(sid)
                    } else {
                        let parent_ast = super_class.expect("super_class present");
                        lower_expr(checked, ctx, parent_ast, None)
                    };
                    let parent_proto = Expr::Member {
                        object: Box::new(parent),
                        property: Box::new(Expr::String {
                            value: "prototype".into(),
                            ty: Type::String,
                        }),
                        computed: false,
                        optional: false,
                        ty: Type::Any,
                    };
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
                    let method = Expr::Member {
                        object: Box::new(parent_proto),
                        property: Box::new(prop),
                        computed: *computed,
                        optional: false,
                        ty: Type::Function,
                    };
                    let call_member = Expr::Member {
                        object: Box::new(method),
                        property: Box::new(Expr::String {
                            value: "call".into(),
                            ty: Type::String,
                        }),
                        computed: false,
                        optional: false,
                        ty: Type::Function,
                    };
                    let this_arg = if let Some(tid) = ctx.derived_this {
                        assert_derived_this(tid)
                    } else {
                        Expr::This { ty: Type::Any }
                    };
                    let mut call_args = Vec::with_capacity(args.len() + 1);
                    // Placeholder this — real receiver bound via outer IIFE so GetThisBinding
                    // runs before Parent.prototype.m lookup (missing method → TypeError) (E19.82).
                    let this_param = ctx.alloc_synthetic_local("__drac_sthis".into(), Type::Any);
                    call_args.push(Arg::Expr(local_expr(this_param)));
                    for a in args {
                        call_args.push(lower_arg(checked, ctx, a, super_class));
                    }
                    let call = Expr::Call {
                        callee: Box::new(call_member),
                        args: call_args,
                        optional: false,
                        ty: expr_ty(checked, *span),
                    };
                    return Expr::Call {
                        callee: Box::new(Expr::Function {
                            name: None,
                            params: vec![Param {
                                pattern: Pattern::Local(this_param),
                                default: None,
                                rest: false,
                            }],
                            body: vec![Stmt::Return { value: Some(call) }],
                            is_async: false,
                            is_generator: false,
                            is_arrow: true,
                            is_method: false,
                            ty: Type::Function,
                        }),
                        args: vec![Arg::Expr(this_arg)],
                        optional: false,
                        ty: expr_ty(checked, *span),
                    };
                }
                // `obj.#m(args)` / `obj?.#m(args)` → brand-check then `__drac_pm_m.call(obj, …)` (E18.37 / E19.53)
                if *private {
                    let fname = match property.as_ref() {
                        AstExpr::Ident(id) => id.name.clone(),
                        _ => panic!("private member property must be ident"),
                    };
                    if let Some(fn_id) = ctx.private_methods.get(&fname).copied() {
                        let brand = resolve_private_brand(ctx, &fname);
                        let obj_expr = lower_expr(checked, ctx, object, super_class);
                        let mut lowered_args = Vec::with_capacity(args.len());
                        for a in args {
                            lowered_args.push(lower_arg(checked, ctx, a, super_class));
                        }
                        let err = format!("Cannot read private method #{fname}");
                        let result_ty = expr_ty(checked, *span);
                        let build = |ctx: &mut LowerCtx, base: Expr| {
                            private_access_checked(
                                ctx,
                                brand,
                                base,
                                |o| {
                                    let call_member = Expr::Member {
                                        object: Box::new(Expr::Local {
                                            id: fn_id,
                                            ty: Type::Function,
                                        }),
                                        property: Box::new(Expr::String {
                                            value: "call".into(),
                                            ty: Type::String,
                                        }),
                                        computed: false,
                                        optional: false,
                                        ty: Type::Function,
                                    };
                                    let mut call_args = Vec::with_capacity(lowered_args.len() + 1);
                                    call_args.push(Arg::Expr(o));
                                    call_args.extend(lowered_args.iter().cloned());
                                    Expr::Call {
                                        callee: Box::new(call_member),
                                        args: call_args,
                                        optional: false,
                                        ty: result_ty,
                                    }
                                },
                                &err,
                            )
                        };
                        if *member_optional || *optional {
                            return optional_private_chain(ctx, obj_expr, |ctx, o| build(ctx, o));
                        }
                        return build(ctx, obj_expr);
                    }
                }
            }
            Expr::Call {
                callee: Box::new(lower_expr(checked, ctx, callee, super_class)),
                args: args
                    .iter()
                    .map(|a| lower_arg(checked, ctx, a, super_class))
                    .collect(),
                optional: *optional,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::New { callee, args, span } => Expr::New {
            callee: Box::new(lower_expr(checked, ctx, callee, super_class)),
            args: args
                .iter()
                .map(|a| lower_arg(checked, ctx, a, super_class))
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::FunctionExpression {
            name,
            params,
            body,
            is_async,
            is_generator,
            is_method,
            span,
            ..
        } => {
            let name = name.as_ref().map(|n| {
                checked
                    .bound
                    .symbols()
                    .iter()
                    .find(|s| s.span == n.span)
                    .map(|s| s.id)
                    .expect("function expression name must be declared")
            });
            // Methods get object-home `super`; plain function expressions do not inherit `super`
            // or derived ctor this TDZ (E19.82.03).
            let prev_object_super = ctx.object_super;
            let prev_derived_this = ctx.derived_this.take();
            let prev_derived_super = ctx.derived_super.take();
            let prev_inits = std::mem::take(&mut ctx.derived_super_inits);
            let prev_ctor_body = ctx.derived_ctor_body;
            let prev_ctor_label = ctx.derived_ctor_label.take();
            let prev_ret_mode = ctx.derived_ret_mode.take();
            let prev_ret_val = ctx.derived_ret_val.take();
            let prev_field_init = ctx.in_field_init;
            ctx.object_super = *is_method;
            ctx.derived_ctor_body = false;
            // Nested functions are not field-init PerformEval sites (E19.82.06).
            ctx.in_field_init = false;
            let params = lower_params(checked, ctx, params, None);
            let body = lower_fn_body(checked, ctx, body, None);
            ctx.object_super = prev_object_super;
            ctx.derived_this = prev_derived_this;
            ctx.derived_super = prev_derived_super;
            ctx.derived_super_inits = prev_inits;
            ctx.derived_ctor_body = prev_ctor_body;
            ctx.derived_ctor_label = prev_ctor_label;
            ctx.derived_ret_mode = prev_ret_mode;
            ctx.derived_ret_val = prev_ret_val;
            ctx.in_field_init = prev_field_init;
            Expr::Function {
                name,
                params,
                body,
                is_async: *is_async,
                is_generator: *is_generator,
                is_arrow: false,
                is_method: *is_method,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::ClassExpression {
            name,
            super_class: sc,
            body,
            span,
        } => lower_class_expression(
            checked,
            ctx,
            name.as_ref(),
            sc.as_deref(),
            body,
            *span,
            name_hint,
        ),
        AstExpr::ArrowFunction {
            params,
            body,
            is_async,
            span,
            ..
        } => {
            // Arrows inherit lexical `super` / derived this; not construct-return wrapping.
            // Also inherit field-init PerformEval early errors (E19.82.06 nested arrows).
            let prev_ctor_body = ctx.derived_ctor_body;
            ctx.derived_ctor_body = false;
            let params = lower_params(checked, ctx, params, super_class);
            let body = match body {
                draconic_ast::ArrowBody::Block(stmt) => {
                    lower_fn_body(checked, ctx, stmt, super_class)
                }
                draconic_ast::ArrowBody::Expr(expr) => {
                    vec![Stmt::Return {
                        value: Some(lower_expr(checked, ctx, expr, super_class)),
                    }]
                }
            };
            ctx.derived_ctor_body = prev_ctor_body;
            Expr::Function {
                name: None,
                params,
                body,
                is_async: *is_async,
                is_generator: false,
                is_arrow: true,
                is_method: false,
                ty: expr_ty(checked, *span),
            }
        }
        AstExpr::ObjectExpression { properties, span } => Expr::Object {
            properties: properties
                .iter()
                .map(|p| match p {
                    AstObjectProp::Property { key, value, .. } => {
                        // NamedEvaluation: `{ id: class {} }` → constructor `.name === "id"` (E19.31).
                        let prop_name_hint: Option<String> = match key {
                            draconic_ast::ObjectKey::Ident(id) => Some(id.name.clone()),
                            draconic_ast::ObjectKey::String(s) => Some(s.value.to_string_lossy()),
                            draconic_ast::ObjectKey::Computed(_) => None,
                        };
                        ObjectProp::Property {
                            key: match key {
                                draconic_ast::ObjectKey::Ident(id) => {
                                    ObjectPropKey::Static(id.name.clone().into())
                                }
                                draconic_ast::ObjectKey::String(s) => {
                                    ObjectPropKey::Static(s.value.clone())
                                }
                                draconic_ast::ObjectKey::Computed(expr) => ObjectPropKey::Computed(
                                    lower_expr(checked, ctx, expr, super_class),
                                ),
                            },
                            value: lower_expr_hint(
                                checked,
                                ctx,
                                value,
                                super_class,
                                prop_name_hint.as_deref(),
                            ),
                        }
                    }
                    AstObjectProp::Accessor {
                        kind,
                        key,
                        params,
                        body,
                        ..
                    } => {
                        let key = match key {
                            draconic_ast::ObjectKey::Ident(id) => {
                                ObjectPropKey::Static(id.name.clone().into())
                            }
                            draconic_ast::ObjectKey::String(s) => {
                                ObjectPropKey::Static(s.value.clone())
                            }
                            draconic_ast::ObjectKey::Computed(expr) => {
                                ObjectPropKey::Computed(lower_expr(checked, ctx, expr, super_class))
                            }
                        };
                        ObjectProp::Accessor {
                            kind: *kind,
                            key,
                            value: {
                                let prev_object_super = ctx.object_super;
                                let prev_ctor_body = ctx.derived_ctor_body;
                                let prev_ctor_label = ctx.derived_ctor_label.take();
                                let prev_ret_mode = ctx.derived_ret_mode.take();
                                let prev_ret_val = ctx.derived_ret_val.take();
                                let prev_derived_this = ctx.derived_this.take();
                                let prev_derived_super = ctx.derived_super.take();
                                let prev_inits = std::mem::take(&mut ctx.derived_super_inits);
                                ctx.object_super = true;
                                ctx.derived_ctor_body = false;
                                let params = lower_params(checked, ctx, params, None);
                                let body = lower_fn_body(checked, ctx, body, None);
                                ctx.object_super = prev_object_super;
                                ctx.derived_ctor_body = prev_ctor_body;
                                ctx.derived_ctor_label = prev_ctor_label;
                                ctx.derived_ret_mode = prev_ret_mode;
                                ctx.derived_ret_val = prev_ret_val;
                                ctx.derived_this = prev_derived_this;
                                ctx.derived_super = prev_derived_super;
                                ctx.derived_super_inits = prev_inits;
                                Expr::Function {
                                    name: None,
                                    params,
                                    body,
                                    is_async: false,
                                    is_generator: false,
                                    is_arrow: false,
                                    is_method: true,
                                    ty: Type::Function,
                                }
                            },
                        }
                    }
                    AstObjectProp::Spread { expr, .. } => {
                        ObjectProp::Spread(lower_expr(checked, ctx, expr, super_class))
                    }
                })
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::ArrayExpression { elements, span, .. } => Expr::Array {
            elements: elements
                .iter()
                .map(|el| match el {
                    AstArrayElement::Expr(e) => {
                        ArrayElement::Expr(lower_expr(checked, ctx, e, super_class))
                    }
                    AstArrayElement::Spread(e) => {
                        ArrayElement::Spread(lower_expr(checked, ctx, e, super_class))
                    }
                    AstArrayElement::Elision => ArrayElement::Elision,
                })
                .collect(),
            ty: expr_ty(checked, *span),
        },
        AstExpr::MemberExpression {
            object,
            property,
            computed,
            optional,
            private,
            span,
        } => {
            if *private {
                let fname = match property.as_ref() {
                    AstExpr::Ident(id) => id.name.as_str(),
                    _ => panic!("private member property must be ident"),
                };
                // `obj?.#f` — optional on the private member itself.
                if *optional {
                    let obj = lower_expr(checked, ctx, object, super_class);
                    let fname = fname.to_string();
                    return optional_private_chain(ctx, obj, |ctx, o| {
                        private_member_get(ctx, &fname, o)
                    });
                }
                // `o?.c.#f` — private continues an optional chain; short-circuit on nullish
                // base before brand-check (not `(o?.c).#f` which would throw) (E19.53).
                if let AstExpr::MemberExpression {
                    object: inner,
                    property: mid_prop,
                    computed: mid_computed,
                    optional: true,
                    private: false,
                    ..
                } = object.as_ref()
                {
                    let base = lower_expr(checked, ctx, inner, super_class);
                    let mid_prop = if *mid_computed {
                        lower_expr(checked, ctx, mid_prop, super_class)
                    } else {
                        match mid_prop.as_ref() {
                            AstExpr::Ident(id) => Expr::String {
                                value: id.name.clone().into(),
                                ty: Type::String,
                            },
                            other => lower_expr(checked, ctx, other, super_class),
                        }
                    };
                    let fname = fname.to_string();
                    let mid_computed = *mid_computed;
                    return optional_private_chain(ctx, base, |ctx, o| {
                        let mid = Expr::Member {
                            object: Box::new(o),
                            property: Box::new(mid_prop.clone()),
                            computed: mid_computed,
                            optional: false,
                            ty: Type::Any,
                        };
                        private_member_get(ctx, &fname, mid)
                    });
                }
                let obj = lower_expr(checked, ctx, object, super_class);
                return private_member_get(ctx, fname, obj);
            }
            // `super.prop` → class with extends: `Parent.prototype.prop`;
            // object/class methods keep `super.prop`; base ctor uses Object.prototype (E19.34 / E19.82).
            if matches!(object.as_ref(), AstExpr::Super { .. }) {
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
                if super_class.is_none() && ctx.derived_super.is_none() {
                    // Keep Super for methods + field-init home-object emit (E19.82).
                    if ctx.object_super || ctx.in_field_init {
                        return Expr::Member {
                            object: Box::new(Expr::Super { ty: Type::Any }),
                            property: Box::new(property),
                            computed: *computed,
                            optional: false,
                            ty: expr_ty(checked, *span),
                        };
                    }
                    return Expr::Member {
                        object: Box::new(member_prop(
                            Expr::IdentName {
                                name: "Object".into(),
                                ty: Type::Function,
                            },
                            "prototype",
                            Type::Any,
                        )),
                        property: Box::new(property),
                        computed: *computed,
                        optional: false,
                        ty: expr_ty(checked, *span),
                    };
                }
                let parent = if let Some(sid) = ctx.derived_super {
                    local_expr(sid)
                } else {
                    let parent_ast = super_class.expect("super_class present");
                    lower_expr(checked, ctx, parent_ast, None)
                };
                let parent_proto = Expr::Member {
                    object: Box::new(parent),
                    property: Box::new(Expr::String {
                        value: "prototype".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Any,
                };
                let member = Expr::Member {
                    object: Box::new(parent_proto),
                    property: Box::new(property),
                    computed: *computed,
                    optional: false,
                    ty: expr_ty(checked, *span),
                };
                // SuperProperty uses GetThisBinding — TDZ before super() (E19.82.03).
                if let Some(tid) = ctx.derived_this {
                    return Expr::Binary {
                        left: Box::new(assert_derived_this(tid)),
                        op: BinaryOp::Comma,
                        right: Box::new(member),
                        ty: expr_ty(checked, *span),
                    };
                }
                return member;
            }
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
            Expr::Member {
                object: Box::new(lower_expr(checked, ctx, object, super_class)),
                property: Box::new(property),
                computed: *computed,
                optional: *optional,
                ty: expr_ty(checked, *span),
            }
        }

        _ => unreachable!("primary/ops exprs are lowered in lower_expr_hint"),
    }
}
