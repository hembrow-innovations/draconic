use draconic_ast::{AssignOp, BinaryOp, Expr as AstExpr, UnaryOp};
use draconic_check::{CheckedProgram, Type};

use crate::lower::LowerCtx;
use crate::{
    Arg, AssignTarget, BindingKind, Expr, LocalId, ObjectProp, ObjectPropKey, Param, Pattern, Stmt,
};

use crate::lower::lower_class_prop::{member_prop, object_method_call, parent_instance_super_base};
use crate::lower::lower_expr::lower_expr;
use crate::lower::lower_private::throw_type_error_expr;

pub(crate) fn define_class_element_with_home(
    ctx: &mut LowerCtx,
    target: Expr,
    key_expr: Expr,
    home_prop: ObjectProp,
    home_proto: Option<Expr>,
) -> Vec<Stmt> {
    // Evaluate key once (yield/await/ToPropertyKey side effects) (E19.78).
    // Unique name: JS emit uses local names, not ids.
    let key_id = ctx.alloc_synthetic_local(format!("__drac_ck_{}", ctx.next_synth_id), Type::Any);
    let key_local = Expr::Local {
        id: key_id,
        ty: Type::Any,
    };
    let mut out = vec![Stmt::Declare {
        local: key_id,
        init: Some(key_expr),
        kind: BindingKind::Let,
    }];
    // Rewrite home prop key to the temp so the object literal does not re-eval.
    let home_prop = match home_prop {
        ObjectProp::Property { value, .. } => ObjectProp::Property {
            key: ObjectPropKey::Computed(key_local.clone()),
            value,
        },
        ObjectProp::Accessor { kind, value, .. } => ObjectProp::Accessor {
            kind,
            key: ObjectPropKey::Computed(key_local.clone()),
            value,
        },
        other => other,
    };
    let mut home_props = Vec::new();
    if let Some(proto) = home_proto {
        home_props.push(ObjectProp::Property {
            key: ObjectPropKey::Static("__proto__".into()),
            value: proto,
        });
    }
    home_props.push(home_prop);
    let home = Expr::Object {
        properties: home_props,
        ty: Type::Object,
    };
    let gopd = object_method_call(
        "getOwnPropertyDescriptor",
        vec![Arg::Expr(home), Arg::Expr(key_local.clone())],
    );
    // ((d) => (d.enumerable = false, d.get === void 0 && delete d.get, d.set === void 0 && delete d.set, d))(gopd)
    let d_id = ctx.alloc_synthetic_local("__drac_desc".into(), Type::Any);
    let d_local = Expr::Local {
        id: d_id,
        ty: Type::Any,
    };
    let set_enumerable = Expr::Assign {
        target: AssignTarget::Member {
            object: Box::new(d_local.clone()),
            property: Box::new(Expr::String {
                value: "enumerable".into(),
                ty: Type::String,
            }),
            computed: false,
        },
        op: AssignOp::Eq,
        value: Box::new(Expr::Boolean {
            value: false,
            ty: Type::Boolean,
        }),
        ty: Type::Any,
    };
    let undef = Expr::Unary {
        op: UnaryOp::Void,
        arg: Box::new(Expr::Number {
            raw: "0".into(),
            ty: Type::Number,
        }),
        ty: Type::Any,
    };
    let delete_if_undef = |prop: &str| {
        let get_prop = member_prop(d_local.clone(), prop, Type::Any);
        let is_undef = Expr::Binary {
            left: Box::new(get_prop),
            op: BinaryOp::EqEqEq,
            right: Box::new(undef.clone()),
            ty: Type::Boolean,
        };
        let del = Expr::Unary {
            op: UnaryOp::Delete,
            arg: Box::new(member_prop(d_local.clone(), prop, Type::Any)),
            ty: Type::Boolean,
        };
        Expr::Binary {
            left: Box::new(is_undef),
            op: BinaryOp::And,
            right: Box::new(del),
            ty: Type::Any,
        }
    };
    let clean = Expr::Binary {
        left: Box::new(set_enumerable),
        op: BinaryOp::Comma,
        right: Box::new(Expr::Binary {
            left: Box::new(delete_if_undef("get")),
            op: BinaryOp::Comma,
            right: Box::new(Expr::Binary {
                left: Box::new(delete_if_undef("set")),
                op: BinaryOp::Comma,
                right: Box::new(d_local.clone()),
                ty: Type::Any,
            }),
            ty: Type::Any,
        }),
        ty: Type::Any,
    };
    let desc = Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(d_id),
                default: None,
                rest: false,
            }],
            body: vec![Stmt::Return { value: Some(clean) }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(gopd)],
        optional: false,
        ty: Type::Any,
    };
    out.push(Stmt::Expr {
        expr: object_method_call(
            "defineProperty",
            vec![Arg::Expr(target), Arg::Expr(key_local), Arg::Expr(desc)],
        ),
    });
    out
}

pub(crate) fn lower_object_prop_key(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    key: &draconic_ast::ObjectKey,
    super_class: Option<&AstExpr>,
) -> ObjectPropKey {
    match key {
        draconic_ast::ObjectKey::Ident(id) => ObjectPropKey::Static(id.name.clone().into()),
        draconic_ast::ObjectKey::String(s) => ObjectPropKey::Static(s.value.clone()),
        draconic_ast::ObjectKey::Computed(expr) => {
            ObjectPropKey::Computed(lower_expr(checked, ctx, expr, super_class))
        }
    }
}

/// A parameter list is "simple" when every param is a plain identifier (no
/// default, no rest, no destructuring). ECMA-262 §14.1.3: a function whose
/// body contains a `"use strict"` directive must have a simple parameter list —
/// injecting the directive into a non-simple-param function is a SyntaxError.
pub(crate) fn params_are_simple(params: &[Param]) -> bool {
    params
        .iter()
        .all(|p| !p.rest && p.default.is_none() && matches!(p.pattern, Pattern::Local(_)))
}

/// Class bodies are strict; method-form install is sloppy unless we inject a directive (E19.72).
///
/// The directive is only valid for simple parameter lists. For non-simple ones
/// (E19.87) strictness comes from the enclosing strict class-builder IIFE
/// (`wrap_class_builder_iife`) — function code contained in strict code is strict.
pub(crate) fn with_use_strict(params: &[Param], mut body: Vec<Stmt>) -> Vec<Stmt> {
    if params_are_simple(params) {
        body.insert(
            0,
            Stmt::Expr {
                expr: Expr::String {
                    value: "use strict".into(),
                    ty: Type::String,
                },
            },
        );
    }
    body
}

pub(crate) fn undef_expr() -> Expr {
    Expr::IdentName {
        name: "undefined".into(),
        ty: Type::Any,
    }
}

pub(crate) fn local_expr(id: LocalId) -> Expr {
    Expr::Local { id, ty: Type::Any }
}

/// `(() => { throw new ReferenceError(msg); })()`
pub(crate) fn throw_reference_error_expr(msg: &str) -> Expr {
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: Vec::new(),
            body: vec![Stmt::Throw {
                value: Expr::New {
                    callee: Box::new(Expr::IdentName {
                        name: "ReferenceError".into(),
                        ty: Type::Function,
                    }),
                    args: vec![Arg::Expr(Expr::String {
                        value: msg.into(),
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

/// Class constructors have [[FunctionKind]] "classConstructor": call without `new` → TypeError (E19.82).
pub(crate) fn class_ctor_new_target_check() -> Stmt {
    Stmt::If {
        test: Expr::Binary {
            left: Box::new(Expr::NewTarget { ty: Type::Any }),
            op: BinaryOp::EqEqEq,
            right: Box::new(undef_expr()),
            ty: Type::Boolean,
        },
        consequent: Box::new(Stmt::Throw {
            value: Expr::New {
                callee: Box::new(Expr::IdentName {
                    name: "TypeError".into(),
                    ty: Type::Function,
                }),
                args: vec![Arg::Expr(Expr::String {
                    value: "Class constructor cannot be invoked without 'new'".into(),
                    ty: Type::String,
                })],
                ty: Type::Any,
            },
        }),
        alternate: None,
    }
}

/// `Object.setPrototypeOf(obj, proto)` — avoids poisoned `__proto__` setters (E19.82).
pub(crate) fn object_set_prototype_of(obj: Expr, proto: Expr) -> Expr {
    object_method_call("setPrototypeOf", vec![Arg::Expr(obj), Arg::Expr(proto)])
}

/// ES GetThisBinding for derived ctor: uninitialized → ReferenceError.
pub(crate) fn assert_derived_this(this_id: LocalId) -> Expr {
    Expr::Conditional {
        test: Box::new(Expr::Binary {
            left: Box::new(local_expr(this_id)),
            op: BinaryOp::EqEqEq,
            right: Box::new(undef_expr()),
            ty: Type::Boolean,
        }),
        consequent: Box::new(throw_reference_error_expr(
            "Must call super constructor in derived class before accessing 'this' or returning from it",
        )),
        alternate: Box::new(local_expr(this_id)),
        ty: Type::Any,
    }
}

/// `AssignOp` (compound) → matching `BinaryOp` for the read-modify-write (`Eq` is a no-op).
pub(crate) fn assign_op_as_binary_op(op: AssignOp) -> BinaryOp {
    match op {
        AssignOp::Eq => BinaryOp::Comma,
        AssignOp::AddEq => BinaryOp::Add,
        AssignOp::SubEq => BinaryOp::Sub,
        AssignOp::MulEq => BinaryOp::Mul,
        AssignOp::DivEq => BinaryOp::Div,
        AssignOp::RemEq => BinaryOp::Rem,
        AssignOp::PowEq => BinaryOp::Pow,
        AssignOp::ShlEq => BinaryOp::Shl,
        AssignOp::ShrEq => BinaryOp::Shr,
        AssignOp::UShrEq => BinaryOp::UShr,
        AssignOp::BitAndEq => BinaryOp::BitAnd,
        AssignOp::BitOrEq => BinaryOp::BitOr,
        AssignOp::BitXorEq => BinaryOp::BitXor,
        AssignOp::AndAndEq => BinaryOp::And,
        AssignOp::OrOrEq => BinaryOp::Or,
        AssignOp::NullishEq => BinaryOp::Nullish,
    }
}

/// `super.x = v` / `super.x op= v` in a derived constructor (E19.86).
///
/// SuperProperty Set targets the superclass prototype with the derived `this` as
/// receiver — setters and receiver fall-through (e.g. a module namespace from a
/// returning super constructor) depend on the receiver, so the assignment desugars
/// to `Reflect.set(base, key, value, this)`. Compound ops read first through
/// `Reflect.get(base, key, this)`.
pub(crate) fn lower_super_prop_assign(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    property: Expr,
    op: AssignOp,
    value: &AstExpr,
    super_class: Option<&AstExpr>,
) -> Expr {
    let super_id = ctx.derived_super.expect("derived_super for super assign");
    let this_id = ctx.derived_this.expect("derived_this for super assign");
    let base = parent_instance_super_base(local_expr(super_id));
    let receiver = assert_derived_this(this_id);
    let reflect_member = |method: &str| Expr::Member {
        object: Box::new(Expr::IdentName {
            name: "Reflect".into(),
            ty: Type::Object,
        }),
        property: Box::new(Expr::String {
            value: method.into(),
            ty: Type::String,
        }),
        computed: false,
        optional: false,
        ty: Type::Function,
    };
    if matches!(op, AssignOp::Eq) {
        let rhs = lower_expr(checked, ctx, value, super_class);
        let set = Expr::Call {
            callee: Box::new(reflect_member("set")),
            args: vec![
                Arg::Expr(base),
                Arg::Expr(property),
                Arg::Expr(rhs.clone()),
                Arg::Expr(receiver),
            ],
            optional: false,
            ty: Type::Boolean,
        };
        return Expr::Binary {
            left: Box::new(set),
            op: BinaryOp::Comma,
            right: Box::new(rhs),
            ty: Type::Any,
        };
    }
    // Compound: (_sv = (Reflect.get(base, key, this) op v), Reflect.set(base, key, _sv, this), _sv)
    let cur = Expr::Call {
        callee: Box::new(reflect_member("get")),
        args: vec![
            Arg::Expr(base.clone()),
            Arg::Expr(property.clone()),
            Arg::Expr(receiver.clone()),
        ],
        optional: false,
        ty: Type::Any,
    };
    let rhs = lower_expr(checked, ctx, value, super_class);
    let combined = Expr::Binary {
        left: Box::new(cur),
        op: assign_op_as_binary_op(op),
        right: Box::new(rhs),
        ty: Type::Any,
    };
    let tmp_id = ctx.alloc_synthetic_local(format!("__drac_sv_{}", ctx.next_synth_id), Type::Any);
    let tmp = local_expr(tmp_id);
    let assign_tmp = Expr::Assign {
        target: AssignTarget::Local(tmp_id),
        op: AssignOp::Eq,
        value: Box::new(combined),
        ty: Type::Any,
    };
    let set = Expr::Call {
        callee: Box::new(reflect_member("set")),
        args: vec![
            Arg::Expr(base),
            Arg::Expr(property),
            Arg::Expr(tmp.clone()),
            Arg::Expr(receiver),
        ],
        optional: false,
        ty: Type::Boolean,
    };
    Expr::Binary {
        left: Box::new(Expr::Binary {
            left: Box::new(assign_tmp),
            op: BinaryOp::Comma,
            right: Box::new(set),
            ty: Type::Any,
        }),
        op: BinaryOp::Comma,
        right: Box::new(tmp),
        ty: Type::Any,
    }
}

/// [[Construct]] completion for derived constructors (E19.82.03):
/// object return → that object; undefined → assert this; else TypeError.
pub(crate) fn possible_constructor_return(this_id: LocalId, value: Option<Expr>) -> Expr {
    let v = value.unwrap_or_else(undef_expr);
    // (v === undefined) ? assertThis(_this)
    //   : (v !== null && (typeof v === "object" || typeof v === "function")) ? v
    //   : throw TypeError
    let is_undef = Expr::Binary {
        left: Box::new(v.clone()),
        op: BinaryOp::EqEqEq,
        right: Box::new(undef_expr()),
        ty: Type::Boolean,
    };
    let is_null = Expr::Binary {
        left: Box::new(v.clone()),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::Null { ty: Type::Any }),
        ty: Type::Boolean,
    };
    let typeof_v = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(v.clone()),
        ty: Type::String,
    };
    let is_object_type = Expr::Binary {
        left: Box::new(Expr::Binary {
            left: Box::new(typeof_v.clone()),
            op: BinaryOp::EqEqEq,
            right: Box::new(Expr::String {
                value: "object".into(),
                ty: Type::String,
            }),
            ty: Type::Boolean,
        }),
        op: BinaryOp::Or,
        right: Box::new(Expr::Binary {
            left: Box::new(typeof_v),
            op: BinaryOp::EqEqEq,
            right: Box::new(Expr::String {
                value: "function".into(),
                ty: Type::String,
            }),
            ty: Type::Boolean,
        }),
        ty: Type::Boolean,
    };
    let is_object = Expr::Binary {
        left: Box::new(Expr::Unary {
            op: UnaryOp::Not,
            arg: Box::new(is_null),
            ty: Type::Boolean,
        }),
        op: BinaryOp::And,
        right: Box::new(is_object_type),
        ty: Type::Boolean,
    };
    Expr::Conditional {
        test: Box::new(is_undef),
        consequent: Box::new(assert_derived_this(this_id)),
        alternate: Box::new(Expr::Conditional {
            test: Box::new(is_object),
            consequent: Box::new(v),
            alternate: Box::new(throw_type_error_expr(
                "Derived constructors may only return object or undefined",
            )),
            ty: Type::Any,
        }),
        ty: Type::Any,
    }
}

/// `super(...args)` in derived ctor → Reflect.construct + field inits + bind this.
/// Spec order: Construct first, then BindThisValue (double-super throws after parent runs).
pub(crate) fn derived_super_call_expr(ctx: &mut LowerCtx, args: Vec<Arg>) -> Expr {
    let this_id = ctx.derived_this.expect("derived_this");
    let super_id = ctx.derived_super.expect("derived_super");
    let inits = ctx.derived_super_inits.clone();
    let args_id = ctx.alloc_synthetic_local("__drac_sargs".into(), Type::Any);
    let result_id = ctx.alloc_synthetic_local("__drac_sres".into(), Type::Any);
    let mut body = Vec::new();
    // let result = Reflect.construct(Super, args, new.target) — always (E19.82.05 double-super).
    let reflect = Expr::Call {
        callee: Box::new(Expr::Member {
            object: Box::new(Expr::IdentName {
                name: "Reflect".into(),
                ty: Type::Object,
            }),
            property: Box::new(Expr::String {
                value: "construct".into(),
                ty: Type::String,
            }),
            computed: false,
            optional: false,
            ty: Type::Function,
        }),
        args: vec![
            Arg::Expr(local_expr(super_id)),
            Arg::Expr(local_expr(args_id)),
            Arg::Expr(Expr::NewTarget { ty: Type::Any }),
        ],
        optional: false,
        ty: Type::Any,
    };
    body.push(Stmt::Declare {
        local: result_id,
        init: Some(reflect),
        kind: BindingKind::Let,
    });
    // if (_this !== undefined) throw ReferenceError (already initialized)
    body.push(Stmt::If {
        test: Expr::Binary {
            left: Box::new(local_expr(this_id)),
            op: BinaryOp::NotEqEq,
            right: Box::new(undef_expr()),
            ty: Type::Boolean,
        },
        consequent: Box::new(Stmt::Throw {
            value: Expr::New {
                callee: Box::new(Expr::IdentName {
                    name: "ReferenceError".into(),
                    ty: Type::Function,
                }),
                args: vec![Arg::Expr(Expr::String {
                    value: "Super constructor may only be called once".into(),
                    ty: Type::String,
                })],
                ty: Type::Any,
            },
        }),
        alternate: None,
    });
    // _this = result; field inits once
    body.push(Stmt::Expr {
        expr: Expr::Assign {
            target: AssignTarget::Local(this_id),
            op: AssignOp::Eq,
            value: Box::new(local_expr(result_id)),
            ty: Type::Any,
        },
    });
    for init in inits {
        body.push(Stmt::Expr { expr: init });
    }
    body.push(Stmt::Return {
        value: Some(local_expr(this_id)),
    });
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(args_id),
                default: None,
                rest: true,
            }],
            body,
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args,
        optional: false,
        ty: Type::Any,
    }
}

/// Property key expression for `Object.defineProperty` / member name (always a value expr).
pub(crate) fn lower_object_key_name_expr(
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

/// Member property + computed flag for assignment targets.
pub(crate) fn lower_object_key_prop(
    checked: &CheckedProgram,
    ctx: &mut LowerCtx,
    key: &draconic_ast::ObjectKey,
    super_class: Option<&AstExpr>,
) -> (Expr, bool) {
    match key {
        draconic_ast::ObjectKey::Ident(id) => (
            Expr::String {
                value: id.name.clone().into(),
                ty: Type::String,
            },
            false,
        ),
        draconic_ast::ObjectKey::String(s) => (
            Expr::String {
                value: s.value.clone(),
                ty: Type::String,
            },
            false,
        ),
        draconic_ast::ObjectKey::Computed(expr) => {
            (lower_expr(checked, ctx, expr, super_class), true)
        }
    }
}
