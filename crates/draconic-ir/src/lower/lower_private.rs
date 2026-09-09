use std::collections::HashMap;

use draconic_ast::{AssignOp, BinaryOp, Expr as AstExpr, UnaryOp, UpdateOp};
use draconic_check::{CheckedProgram, Type};

use crate::lower::LowerCtx;
use crate::{
    Arg, AssignTarget, BindingKind, Expr, LocalId, ObjectProp, ObjectPropKey, Param, Pattern, Stmt,
};

use crate::lower::lower_class_element::{local_expr, with_use_strict};
use crate::lower::lower_class_prop::member_prop;
use crate::lower::lower_expr::lower_expr;

pub(crate) fn call_method_with_home(home_proto: Expr, body: Vec<Stmt>, receiver: Expr) -> Expr {
    let method_fn = Expr::Function {
        name: None,
        params: Vec::new(),
        body: with_use_strict(&[], body),
        is_async: false,
        is_generator: false,
        is_arrow: false,
        is_method: true,
        ty: Type::Function,
    };
    let home = Expr::Object {
        properties: vec![
            ObjectProp::Property {
                key: ObjectPropKey::Static("__proto__".into()),
                value: home_proto,
            },
            ObjectProp::Property {
                key: ObjectPropKey::Static("__fi".into()),
                value: method_fn,
            },
        ],
        ty: Type::Object,
    };
    Expr::Call {
        callee: Box::new(member_prop(
            member_prop(home, "__fi", Type::Function),
            "call",
            Type::Function,
        )),
        args: vec![Arg::Expr(receiver)],
        optional: false,
        ty: Type::Any,
    }
}

/// `fn.call(object, ...args)` for private method/accessor invocation.
pub(crate) fn private_fn_call(fn_id: LocalId, object: Expr, args: Vec<Arg>) -> Expr {
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
    let mut call_args = Vec::with_capacity(args.len() + 1);
    call_args.push(Arg::Expr(object));
    call_args.extend(args);
    Expr::Call {
        callee: Box::new(call_member),
        args: call_args,
        optional: false,
        ty: Type::Any,
    }
}

/// `Object.isExtensible(object)`.
pub(crate) fn object_is_extensible(object: Expr) -> Expr {
    Expr::Call {
        callee: Box::new(member_prop(
            Expr::IdentName {
                name: "Object".into(),
                ty: Type::Function,
            },
            "isExtensible",
            Type::Function,
        )),
        args: vec![Arg::Expr(object)],
        optional: false,
        ty: Type::Boolean,
    }
}

/// PrivateMethodOrAccessorAdd: non-extensible or already branded → TypeError (E18.40 / E19.82.09).
pub(crate) fn private_brand_add(ctx: &mut LowerCtx, brand: LocalId, object: Expr) -> Expr {
    let oid = ctx.alloc_synthetic_local("__drac_o".into(), Type::Any);
    let o = local_expr(oid);
    let not_ext = Expr::Unary {
        op: UnaryOp::Not,
        arg: Box::new(object_is_extensible(o.clone())),
        ty: Type::Boolean,
    };
    let already = private_brand_has(brand, o.clone());
    let add_call = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(local_expr(brand)),
            property: Box::new(Expr::String {
                value: "add".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(o)],
        optional: false,
        ty: Type::Any,
    };
    let body = Expr::Conditional {
        test: Box::new(not_ext),
        consequent: Box::new(throw_type_error_expr(
            "Cannot define private method on non-extensible object",
        )),
        alternate: Box::new(Expr::Conditional {
            test: Box::new(already),
            consequent: Box::new(throw_type_error_expr(
                "Cannot add private method that already exists",
            )),
            alternate: Box::new(add_call),
            ty: Type::Any,
        }),
        ty: Type::Any,
    };
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(oid),
                default: None,
                rest: false,
            }],
            body: vec![Stmt::Return { value: Some(body) }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(object)],
        optional: false,
        ty: Type::Any,
    }
}

/// PrivateFieldAdd: non-extensible or already present → TypeError (E18.35 / E19.82.09).
pub(crate) fn private_field_add(
    ctx: &mut LowerCtx,
    wm: LocalId,
    object: Expr,
    value: Expr,
) -> Expr {
    let oid = ctx.alloc_synthetic_local("__drac_o".into(), Type::Any);
    let vid = ctx.alloc_synthetic_local("__drac_v".into(), Type::Any);
    let o = local_expr(oid);
    let v = local_expr(vid);
    let not_ext = Expr::Unary {
        op: UnaryOp::Not,
        arg: Box::new(object_is_extensible(o.clone())),
        ty: Type::Boolean,
    };
    let already = private_brand_has(wm, o.clone());
    let set_call = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(local_expr(wm)),
            property: Box::new(Expr::String {
                value: "set".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(o), Arg::Expr(v.clone())],
        optional: false,
        ty: Type::Any,
    };
    let set_and_yield = Expr::Binary {
        left: Box::new(set_call),
        op: BinaryOp::Comma,
        right: Box::new(v),
        ty: Type::Any,
    };
    let body = Expr::Conditional {
        test: Box::new(not_ext),
        consequent: Box::new(throw_type_error_expr(
            "Cannot define private field on non-extensible object",
        )),
        alternate: Box::new(Expr::Conditional {
            test: Box::new(already),
            consequent: Box::new(throw_type_error_expr(
                "Cannot add private field that already exists",
            )),
            alternate: Box::new(set_and_yield),
            ty: Type::Any,
        }),
        ty: Type::Any,
    };
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![
                Param {
                    pattern: Pattern::Local(oid),
                    default: None,
                    rest: false,
                },
                Param {
                    pattern: Pattern::Local(vid),
                    default: None,
                    rest: false,
                },
            ],
            body: vec![Stmt::Return { value: Some(body) }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(object), Arg::Expr(value)],
        optional: false,
        ty: Type::Any,
    }
}

/// `#name in object` → object is object-like and brand/WeakMap has it (E18.40).
pub(crate) fn private_in_check(brand: LocalId, object: Expr) -> Expr {
    // `obj != null && (typeof obj === "object" || typeof obj === "function") && brand.has(obj)`
    let not_nullish = Expr::Binary {
        left: Box::new(object.clone()),
        op: BinaryOp::NotEq,
        right: Box::new(Expr::Null { ty: Type::Null }),
        ty: Type::Boolean,
    };
    let typeof_obj = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(object.clone()),
        ty: Type::String,
    };
    let is_object = Expr::Binary {
        left: Box::new(typeof_obj),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::String {
            value: "object".into(),
            ty: Type::String,
        }),
        ty: Type::Boolean,
    };
    let typeof_fn = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(object.clone()),
        ty: Type::String,
    };
    let is_function = Expr::Binary {
        left: Box::new(typeof_fn),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::String {
            value: "function".into(),
            ty: Type::String,
        }),
        ty: Type::Boolean,
    };
    let is_obj_like = Expr::Binary {
        left: Box::new(is_object),
        op: BinaryOp::Or,
        right: Box::new(is_function),
        ty: Type::Boolean,
    };
    let guard = Expr::Binary {
        left: Box::new(not_nullish),
        op: BinaryOp::And,
        right: Box::new(is_obj_like),
        ty: Type::Boolean,
    };
    let has_call = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(Expr::Local {
                id: brand,
                ty: Type::Any,
            }),
            property: Box::new(Expr::String {
                value: "has".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(object)],
        optional: false,
        ty: Type::Boolean,
    };
    Expr::Binary {
        left: Box::new(guard),
        op: BinaryOp::And,
        right: Box::new(has_call),
        ty: Type::Boolean,
    }
}

pub(crate) fn ensure_private_brand(
    ctx: &mut LowerCtx,
    class_local: LocalId,
    private_brand_map: &mut HashMap<String, LocalId>,
    private_brand_decls: &mut Vec<Stmt>,
    instance_brands: &mut Vec<LocalId>,
    static_brands: &mut Vec<LocalId>,
    name: &str,
    is_static: bool,
) {
    if let Some(existing) = private_brand_map.get(name) {
        if is_static {
            if !static_brands.contains(existing) {
                static_brands.push(*existing);
            }
        } else if !instance_brands.contains(existing) {
            instance_brands.push(*existing);
        }
        return;
    }
    let brand_name = format!("__drac_pb_{}_{}", class_local.0, name);
    let brand_id = ctx.alloc_synthetic_local(brand_name, Type::Any);
    private_brand_map.insert(name.to_string(), brand_id);
    private_brand_decls.push(Stmt::Declare {
        local: brand_id,
        init: Some(Expr::New {
            callee: Box::new(Expr::IdentName {
                name: "WeakSet".into(),
                ty: Type::Function,
            }),
            args: Vec::new(),
            ty: Type::Any,
        }),
        kind: BindingKind::Let,
    });
    if is_static {
        static_brands.push(brand_id);
    } else {
        instance_brands.push(brand_id);
    }
}

pub(crate) fn resolve_private_brand(ctx: &LowerCtx, name: &str) -> LocalId {
    if let Some(wm) = ctx.private_fields.get(name).copied() {
        return wm;
    }
    if let Some(brand) = ctx.private_brands.get(name).copied() {
        return brand;
    }
    panic!("unknown private brand #{name}");
}

/// `brand.has(object)` (WeakMap/WeakSet).
pub(crate) fn private_brand_has(brand: LocalId, object: Expr) -> Expr {
    Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(Expr::Local {
                id: brand,
                ty: Type::Any,
            }),
            property: Box::new(Expr::String {
                value: "has".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(object)],
        optional: false,
        ty: Type::Boolean,
    }
}

/// Object-like check: `o != null && (typeof o === "object" || typeof o === "function")`.
pub(crate) fn is_object_like_expr(object: Expr) -> Expr {
    let not_nullish = Expr::Binary {
        left: Box::new(object.clone()),
        op: BinaryOp::NotEq,
        right: Box::new(Expr::Null { ty: Type::Null }),
        ty: Type::Boolean,
    };
    let typeof_obj = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(object.clone()),
        ty: Type::String,
    };
    let is_object = Expr::Binary {
        left: Box::new(typeof_obj),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::String {
            value: "object".into(),
            ty: Type::String,
        }),
        ty: Type::Boolean,
    };
    let typeof_fn = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(object),
        ty: Type::String,
    };
    let is_function = Expr::Binary {
        left: Box::new(typeof_fn),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::String {
            value: "function".into(),
            ty: Type::String,
        }),
        ty: Type::Boolean,
    };
    let is_obj_like = Expr::Binary {
        left: Box::new(is_object),
        op: BinaryOp::Or,
        right: Box::new(is_function),
        ty: Type::Boolean,
    };
    Expr::Binary {
        left: Box::new(not_nullish),
        op: BinaryOp::And,
        right: Box::new(is_obj_like),
        ty: Type::Boolean,
    }
}

/// `((o) => body)(arg)` with `o` bound once (E19.53 brand / optional private).
pub(crate) fn iife_bind_arg(
    ctx: &mut LowerCtx,
    arg: Expr,
    body: impl FnOnce(Expr) -> Expr,
) -> Expr {
    let pid = ctx.alloc_synthetic_local("__drac_o".into(), Type::Any);
    let body_expr = body(Expr::Local {
        id: pid,
        ty: Type::Any,
    });
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(pid),
                default: None,
                rest: false,
            }],
            body: vec![Stmt::Return {
                value: Some(body_expr),
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(arg)],
        optional: false,
        ty: Type::Any,
    }
}

/// `base?.#priv…` → `((o) => o == null ? undefined : then(o))(base)`.
pub(crate) fn optional_private_chain(
    ctx: &mut LowerCtx,
    base: Expr,
    then: impl FnOnce(&mut LowerCtx, Expr) -> Expr,
) -> Expr {
    let pid = ctx.alloc_synthetic_local("__drac_o".into(), Type::Any);
    let o = Expr::Local {
        id: pid,
        ty: Type::Any,
    };
    let nullish = Expr::Binary {
        left: Box::new(o.clone()),
        op: BinaryOp::EqEq,
        right: Box::new(Expr::Null { ty: Type::Null }),
        ty: Type::Boolean,
    };
    let then_expr = then(ctx, o);
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(pid),
                default: None,
                rest: false,
            }],
            body: vec![Stmt::Return {
                value: Some(Expr::Conditional {
                    test: Box::new(nullish),
                    consequent: Box::new(Expr::IdentName {
                        name: "undefined".into(),
                        ty: Type::Any,
                    }),
                    alternate: Box::new(then_expr),
                    ty: Type::Any,
                }),
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(base)],
        optional: false,
        ty: Type::Any,
    }
}

/// Brand-check `object` then yield `then` (PrivateBrandCheck / PrivateFieldFind).
pub(crate) fn private_access_checked(
    ctx: &mut LowerCtx,
    brand: LocalId,
    object: Expr,
    then: impl FnOnce(Expr) -> Expr,
    err_msg: &str,
) -> Expr {
    iife_bind_arg(ctx, object, |o| {
        let ok = Expr::Binary {
            left: Box::new(is_object_like_expr(o.clone())),
            op: BinaryOp::And,
            right: Box::new(private_brand_has(brand, o.clone())),
            ty: Type::Boolean,
        };
        Expr::Conditional {
            test: Box::new(ok),
            consequent: Box::new(then(o)),
            alternate: Box::new(throw_type_error_expr(err_msg)),
            ty: Type::Any,
        }
    })
}

/// `wm.get(object)` with brand check (missing → TypeError).
pub(crate) fn private_field_get(ctx: &mut LowerCtx, wm: LocalId, object: Expr) -> Expr {
    private_access_checked(
        ctx,
        wm,
        object,
        |o| Expr::Call {
            callee: Box::new(Expr::Member {
                object: Box::new(Expr::Local {
                    id: wm,
                    ty: Type::Any,
                }),
                property: Box::new(Expr::String {
                    value: "get".into(),
                    ty: Type::String,
                }),
                computed: false,
                optional: false,
                ty: Type::Function,
            }),
            args: vec![Arg::Expr(o)],
            optional: false,
            ty: Type::Any,
        },
        "Cannot read private member from an object whose class did not declare it",
    )
}

/// `(wm.set(object, value), value)` with brand check so assignment yields the RHS.
pub(crate) fn private_field_set(
    ctx: &mut LowerCtx,
    wm: LocalId,
    object: Expr,
    value: Expr,
) -> Expr {
    private_access_checked(
        ctx,
        wm,
        object,
        |o| {
            let set_call = Expr::Call {
                callee: Box::new(Expr::Member {
                    object: Box::new(Expr::Local {
                        id: wm,
                        ty: Type::Any,
                    }),
                    property: Box::new(Expr::String {
                        value: "set".into(),
                        ty: Type::String,
                    }),
                    computed: false,
                    optional: false,
                    ty: Type::Function,
                }),
                args: vec![Arg::Expr(o), Arg::Expr(value.clone())],
                optional: false,
                ty: Type::Any,
            };
            Expr::Binary {
                left: Box::new(set_call),
                op: BinaryOp::Comma,
                right: Box::new(value),
                ty: Type::Any,
            }
        },
        "Cannot write private member to an object whose class did not declare it",
    )
}

/// Read private field / accessor / method value for `object.#name` (with brand check).
pub(crate) fn private_member_get(ctx: &mut LowerCtx, fname: &str, object: Expr) -> Expr {
    if let Some(fn_id) = ctx.private_methods.get(fname).copied() {
        let brand = resolve_private_brand(ctx, fname);
        return private_access_checked(
            ctx,
            brand,
            object,
            |_| Expr::Local {
                id: fn_id,
                ty: Type::Function,
            },
            &format!("Cannot read private method #{fname}"),
        );
    }
    if let Some((get, set)) = ctx.private_accessors.get(fname).copied() {
        let _ = set;
        let brand = resolve_private_brand(ctx, fname);
        if let Some(get_id) = get {
            return private_access_checked(
                ctx,
                brand,
                object,
                |o| private_fn_call(get_id, o, Vec::new()),
                &format!("Cannot read private accessor #{fname}"),
            );
        }
        return throw_type_error_expr(&format!("Private accessor #{fname} has no getter"));
    }
    if let Some(wm) = ctx.private_fields.get(fname).copied() {
        return private_field_get(ctx, wm, object);
    }
    throw_type_error_expr(&format!("unknown private field #{fname}"))
}

/// `(() => { throw new TypeError(msg); })()` — expression-position TypeError (E19.36).
pub(crate) fn throw_type_error_expr(message: &str) -> Expr {
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: Vec::new(),
            body: vec![Stmt::Throw {
                value: Expr::New {
                    callee: Box::new(Expr::IdentName {
                        name: "TypeError".into(),
                        ty: Type::Function,
                    }),
                    args: vec![Arg::Expr(Expr::String {
                        value: message.into(),
                        ty: Type::String,
                    })],
                    ty: Type::Any,
                },
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
    }
}

/// Write private field / accessor: yields `value` (with brand check).
pub(crate) fn private_member_set(
    ctx: &mut LowerCtx,
    fname: &str,
    object: Expr,
    value: Expr,
) -> Expr {
    // Private methods are not writable (TypeError, not IR panic).
    if ctx.private_methods.contains_key(fname) {
        return throw_type_error_expr(&format!("Private method #{fname} is not writable"));
    }
    if let Some((get, set)) = ctx.private_accessors.get(fname).copied() {
        let _ = get;
        let brand = resolve_private_brand(ctx, fname);
        if let Some(set_id) = set {
            return private_access_checked(
                ctx,
                brand,
                object,
                |o| {
                    let set_call = private_fn_call(set_id, o, vec![Arg::Expr(value.clone())]);
                    Expr::Binary {
                        left: Box::new(set_call),
                        op: BinaryOp::Comma,
                        right: Box::new(value),
                        ty: Type::Any,
                    }
                },
                &format!("Cannot write private accessor #{fname}"),
            );
        }
        return throw_type_error_expr(&format!("Private accessor #{fname} has no setter"));
    }
    if let Some(wm) = ctx.private_fields.get(fname).copied() {
        return private_field_set(ctx, wm, object, value);
    }
    throw_type_error_expr(&format!("unknown private field #{fname}"))
}

/// `obj.#f = v` / compound / logical assign; object evaluated once (E19.36).
pub(crate) fn lower_private_assign(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    fname: &str,
    object: &AstExpr,
    op: AssignOp,
    value: &AstExpr,
    super_class: Option<&AstExpr>,
) -> Expr {
    let obj_expr = lower_expr(checked, ctx, object, super_class);
    let rhs = lower_expr(checked, ctx, value, super_class);
    let tmp = ctx.alloc_synthetic_local(format!("__drac_pobj_{fname}"), Type::Any);
    let bind_obj = Expr::Assign {
        target: AssignTarget::Local(tmp),
        op: AssignOp::Eq,
        value: Box::new(obj_expr),
        ty: Type::Any,
    };
    let obj_local = || Expr::Local {
        id: tmp,
        ty: Type::Any,
    };
    // Bind computed RHS to a temp so `private_field_set`'s `(set, value)` does not
    // re-evaluate get+binop (would double-increment).
    let val_id = ctx.alloc_synthetic_local(format!("__drac_pval_{fname}"), Type::Any);
    let bind_val = |v: Expr| Expr::Assign {
        target: AssignTarget::Local(val_id),
        op: AssignOp::Eq,
        value: Box::new(v),
        ty: Type::Any,
    };
    let val_local = || Expr::Local {
        id: val_id,
        ty: Type::Any,
    };
    let assigned = match op {
        AssignOp::Eq => {
            let set = private_member_set(ctx, fname, obj_local(), val_local());
            Expr::Binary {
                left: Box::new(bind_val(rhs)),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            }
        }
        AssignOp::AndAndEq => {
            let cur = private_member_get(ctx, fname, obj_local());
            let set = private_member_set(ctx, fname, obj_local(), val_local());
            let then_set = Expr::Binary {
                left: Box::new(bind_val(rhs)),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            };
            Expr::Binary {
                left: Box::new(cur),
                op: BinaryOp::And,
                right: Box::new(then_set),
                ty: Type::Any,
            }
        }
        AssignOp::OrOrEq => {
            let cur = private_member_get(ctx, fname, obj_local());
            let set = private_member_set(ctx, fname, obj_local(), val_local());
            let then_set = Expr::Binary {
                left: Box::new(bind_val(rhs)),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            };
            Expr::Binary {
                left: Box::new(cur),
                op: BinaryOp::Or,
                right: Box::new(then_set),
                ty: Type::Any,
            }
        }
        AssignOp::NullishEq => {
            let cur = private_member_get(ctx, fname, obj_local());
            let set = private_member_set(ctx, fname, obj_local(), val_local());
            let then_set = Expr::Binary {
                left: Box::new(bind_val(rhs)),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            };
            Expr::Binary {
                left: Box::new(cur),
                op: BinaryOp::Nullish,
                right: Box::new(then_set),
                ty: Type::Any,
            }
        }
        other => {
            let binop = other.binary_op().expect("compound assign op has binary_op");
            let cur = private_member_get(ctx, fname, obj_local());
            let combined = Expr::Binary {
                left: Box::new(cur),
                op: binop,
                right: Box::new(rhs),
                ty: Type::Any,
            };
            let set = private_member_set(ctx, fname, obj_local(), val_local());
            Expr::Binary {
                left: Box::new(bind_val(combined)),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            }
        }
    };
    Expr::Binary {
        left: Box::new(bind_obj),
        op: BinaryOp::Comma,
        right: Box::new(assigned),
        ty: Type::Any,
    }
}

pub(crate) fn lower_private_update(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    fname: &str,
    object: &AstExpr,
    op: UpdateOp,
    prefix: bool,
    super_class: Option<&AstExpr>,
) -> Expr {
    let obj_expr = lower_expr(checked, ctx, object, super_class);
    let tmp = ctx.alloc_synthetic_local(format!("__drac_pobj_{fname}"), Type::Any);
    let bind_obj = Expr::Assign {
        target: AssignTarget::Local(tmp),
        op: AssignOp::Eq,
        value: Box::new(obj_expr),
        ty: Type::Any,
    };
    let obj_local = || Expr::Local {
        id: tmp,
        ty: Type::Any,
    };
    let one = Expr::Number {
        raw: "1".into(),
        ty: Type::Number,
    };
    let binop = match op {
        UpdateOp::Inc => BinaryOp::Add,
        UpdateOp::Dec => BinaryOp::Sub,
    };
    let next_id = ctx.alloc_synthetic_local(format!("__drac_pnext_{fname}"), Type::Any);
    let next_local = || Expr::Local {
        id: next_id,
        ty: Type::Any,
    };
    if prefix {
        let cur = private_member_get(ctx, fname, obj_local());
        let bind_next = Expr::Assign {
            target: AssignTarget::Local(next_id),
            op: AssignOp::Eq,
            value: Box::new(Expr::Binary {
                left: Box::new(cur),
                op: binop,
                right: Box::new(one),
                ty: Type::Any,
            }),
            ty: Type::Any,
        };
        let set = private_member_set(ctx, fname, obj_local(), next_local());
        return Expr::Binary {
            left: Box::new(bind_obj),
            op: BinaryOp::Comma,
            right: Box::new(Expr::Binary {
                left: Box::new(bind_next),
                op: BinaryOp::Comma,
                right: Box::new(set),
                ty: Type::Any,
            }),
            ty: Type::Any,
        };
    }
    let cur_id = ctx.alloc_synthetic_local(format!("__drac_pcur_{fname}"), Type::Any);
    let bind_cur = Expr::Assign {
        target: AssignTarget::Local(cur_id),
        op: AssignOp::Eq,
        value: Box::new(private_member_get(ctx, fname, obj_local())),
        ty: Type::Any,
    };
    let bind_next = Expr::Assign {
        target: AssignTarget::Local(next_id),
        op: AssignOp::Eq,
        value: Box::new(Expr::Binary {
            left: Box::new(Expr::Local {
                id: cur_id,
                ty: Type::Any,
            }),
            op: binop,
            right: Box::new(one),
            ty: Type::Any,
        }),
        ty: Type::Any,
    };
    let set = private_member_set(ctx, fname, obj_local(), next_local());
    let set_then_old = Expr::Binary {
        left: Box::new(set),
        op: BinaryOp::Comma,
        right: Box::new(Expr::Local {
            id: cur_id,
            ty: Type::Any,
        }),
        ty: Type::Any,
    };
    Expr::Binary {
        left: Box::new(bind_obj),
        op: BinaryOp::Comma,
        right: Box::new(Expr::Binary {
            left: Box::new(bind_cur),
            op: BinaryOp::Comma,
            right: Box::new(Expr::Binary {
                left: Box::new(bind_next),
                op: BinaryOp::Comma,
                right: Box::new(set_then_old),
                ty: Type::Any,
            }),
            ty: Type::Any,
        }),
        ty: Type::Any,
    }
}
