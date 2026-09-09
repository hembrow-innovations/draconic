use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_check::Type;

use crate::lower::LowerCtx;
use crate::{Arg, AssignTarget, Expr, LocalId, ObjectProp, ObjectPropKey, Param, Pattern, Stmt};

use crate::lower::lower_class_element::local_expr;

pub(crate) fn data_prop_desc(
    value: Expr,
    writable: bool,
    enumerable: bool,
    configurable: bool,
) -> Expr {
    Expr::Object {
        properties: vec![
            ObjectProp::Property {
                key: ObjectPropKey::Static("value".into()),
                value,
            },
            ObjectProp::Property {
                key: ObjectPropKey::Static("writable".into()),
                value: Expr::Boolean {
                    value: writable,
                    ty: Type::Boolean,
                },
            },
            ObjectProp::Property {
                key: ObjectPropKey::Static("enumerable".into()),
                value: Expr::Boolean {
                    value: enumerable,
                    ty: Type::Boolean,
                },
            },
            ObjectProp::Property {
                key: ObjectPropKey::Static("configurable".into()),
                value: Expr::Boolean {
                    value: configurable,
                    ty: Type::Boolean,
                },
            },
        ],
        ty: Type::Object,
    }
}

/// `Object.defineProperty(fn, "name", { value, writable: false, enumerable: false, configurable: true })`
/// — ECMA-262 SetFunctionName (used for class NamedEvaluation, E19.31).
pub(crate) fn set_function_name_stmt(local: LocalId, name: &str) -> Stmt {
    Stmt::Expr {
        expr: object_method_call(
            "defineProperty",
            vec![
                Arg::Expr(Expr::Local {
                    id: local,
                    ty: Type::Function,
                }),
                Arg::Expr(Expr::String {
                    value: "name".into(),
                    ty: Type::String,
                }),
                Arg::Expr(data_prop_desc(
                    Expr::String {
                        value: name.into(),
                        ty: Type::String,
                    },
                    false,
                    false,
                    true,
                )),
            ],
        ),
    }
}

/// NamedEvaluation: `((f) => (Object.defineProperty(f,"name",…), f))(fe)`.
/// Used for anonymous function/arrow field initializers (E19.82.04).
pub(crate) fn set_function_name_on_expr(ctx: &mut LowerCtx, fe: Expr, name: &str) -> Expr {
    let tmp = ctx.alloc_synthetic_local(
        format!("__drac_fnname_{}", ctx.next_synth_id),
        Type::Function,
    );
    let set_name = object_method_call(
        "defineProperty",
        vec![
            Arg::Expr(local_expr(tmp)),
            Arg::Expr(Expr::String {
                value: "name".into(),
                ty: Type::String,
            }),
            Arg::Expr(data_prop_desc(
                Expr::String {
                    value: name.into(),
                    ty: Type::String,
                },
                false,
                false,
                true,
            )),
        ],
    );
    Expr::Call {
        callee: Box::new(Expr::Function {
            name: None,
            params: vec![Param {
                pattern: Pattern::Local(tmp),
                default: None,
                rest: false,
            }],
            body: vec![Stmt::Return {
                value: Some(Expr::Binary {
                    left: Box::new(set_name),
                    op: BinaryOp::Comma,
                    right: Box::new(local_expr(tmp)),
                    ty: Type::Function,
                }),
            }],
            is_async: false,
            is_generator: false,
            is_arrow: true,
            is_method: false,
            ty: Type::Function,
        }),
        args: vec![Arg::Expr(fe)],
        optional: false,
        ty: Type::Function,
    }
}

/// CreateDataPropertyOrThrow via defineProperty (throws on non-writable `prototype`, E19.82.04).
pub(crate) fn create_data_property_or_throw(object: Expr, key: Expr, value: Expr) -> Expr {
    object_method_call(
        "defineProperty",
        vec![
            Arg::Expr(object),
            Arg::Expr(key),
            Arg::Expr(data_prop_desc(value, true, true, true)),
        ],
    )
}

pub(crate) fn object_key_private_name(key: &draconic_ast::ObjectKey) -> Option<&str> {
    match key {
        draconic_ast::ObjectKey::Ident(id) => Some(id.name.as_str()),
        _ => None,
    }
}

/// `obj.prop` member read helper.
pub(crate) fn member_prop(object: Expr, prop: &str, ty: Type) -> Expr {
    Expr::Member {
        object: Box::new(object),
        property: Box::new(Expr::String {
            value: prop.into(),
            ty: Type::String,
        }),
        computed: false,
        optional: false,
        ty,
    }
}

/// `Object.method(...)` call helper.
pub(crate) fn object_method_call(method: &str, args: Vec<Arg>) -> Expr {
    Expr::Call {
        callee: Box::new(member_prop(
            Expr::IdentName {
                name: "Object".into(),
                ty: Type::Object,
            },
            method,
            Type::Function,
        )),
        args,
        optional: false,
        ty: Type::Any,
    }
}

/// ClassDefinitionEvaluation heritage checks (E19.82.02):
/// - `null` is allowed (protoParent = null)
/// - else IsConstructor(superclass) must be true → TypeError
/// - else Get(superclass, "prototype") must be Object or Null → TypeError
///
/// When `proto_out` is set, stores the single Get(parent,"prototype") (or null for
/// `extends null`) into that local so ClassDefinitionEvaluation does not re-get (E19.82).
pub(crate) fn heritage_validation_stmts(parent: Expr, proto_out: Option<LocalId>) -> Vec<Stmt> {
    let is_null = Expr::Binary {
        left: Box::new(parent.clone()),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::Null { ty: Type::Any }),
        ty: Type::Boolean,
    };
    let not_null = Expr::Unary {
        op: UnaryOp::Not,
        arg: Box::new(is_null.clone()),
        ty: Type::Boolean,
    };

    // IsConstructor via Reflect.construct(empty, [], parent). Engines reject
    // arrows/async/generators here; a bare Proxy construct trap does not (E19.82).
    // Intercept Get(parent,"prototype") during GetPrototypeFromConstructor so the
    // single Get is cached for linking (prototype-getter calls === 1).
    let empty_ctor = Expr::Function {
        name: None,
        params: Vec::new(),
        body: Vec::new(),
        is_async: false,
        is_generator: false,
        is_arrow: false,
        is_method: false,
        ty: Type::Function,
    };

    let throw_not_ctor = Stmt::Throw {
        value: Expr::New {
            callee: Box::new(Expr::IdentName {
                name: "TypeError".into(),
                ty: Type::Function,
            }),
            args: vec![Arg::Expr(Expr::String {
                value: "Class extends value is not a constructor or null".into(),
                ty: Type::String,
            })],
            ty: Type::Any,
        },
    };

    // When proto_out is set, Reflect.construct uses newTarget = Proxy(parent) whose
    // get trap caches parent.prototype into proto_out (single Get for linking).
    let (try_is_ctor, proto_for_check, after_ctor_stmts): (Stmt, Expr, Vec<Stmt>) =
        if let Some(pid) = proto_out {
            let get_trap = Expr::Function {
                name: None,
                params: vec![
                    Param {
                        pattern: Pattern::Name("t".into()),
                        default: None,
                        rest: false,
                    },
                    Param {
                        pattern: Pattern::Name("p".into()),
                        default: None,
                        rest: false,
                    },
                    Param {
                        pattern: Pattern::Name("r".into()),
                        default: None,
                        rest: false,
                    },
                ],
                body: vec![Stmt::If {
                    test: Expr::Binary {
                        left: Box::new(Expr::IdentName {
                            name: "p".into(),
                            ty: Type::Any,
                        }),
                        op: BinaryOp::EqEqEq,
                        right: Box::new(Expr::String {
                            value: "prototype".into(),
                            ty: Type::String,
                        }),
                        ty: Type::Boolean,
                    },
                    consequent: Box::new(Stmt::Return {
                        value: Some(Expr::Assign {
                            target: AssignTarget::Local(pid),
                            op: AssignOp::Eq,
                            value: Box::new(Expr::Call {
                                callee: Box::new(member_prop(
                                    Expr::IdentName {
                                        name: "Reflect".into(),
                                        ty: Type::Object,
                                    },
                                    "get",
                                    Type::Function,
                                )),
                                args: vec![
                                    Arg::Expr(Expr::IdentName {
                                        name: "t".into(),
                                        ty: Type::Any,
                                    }),
                                    Arg::Expr(Expr::IdentName {
                                        name: "p".into(),
                                        ty: Type::Any,
                                    }),
                                    Arg::Expr(Expr::IdentName {
                                        name: "r".into(),
                                        ty: Type::Any,
                                    }),
                                ],
                                optional: false,
                                ty: Type::Any,
                            }),
                            ty: Type::Any,
                        }),
                    }),
                    alternate: Some(Box::new(Stmt::Return {
                        value: Some(Expr::Call {
                            callee: Box::new(member_prop(
                                Expr::IdentName {
                                    name: "Reflect".into(),
                                    ty: Type::Object,
                                },
                                "get",
                                Type::Function,
                            )),
                            args: vec![
                                Arg::Expr(Expr::IdentName {
                                    name: "t".into(),
                                    ty: Type::Any,
                                }),
                                Arg::Expr(Expr::IdentName {
                                    name: "p".into(),
                                    ty: Type::Any,
                                }),
                                Arg::Expr(Expr::IdentName {
                                    name: "r".into(),
                                    ty: Type::Any,
                                }),
                            ],
                            optional: false,
                            ty: Type::Any,
                        }),
                    })),
                }],
                is_async: false,
                is_generator: false,
                is_arrow: false,
                is_method: false,
                ty: Type::Function,
            };
            let proxy_new_target = Expr::New {
                callee: Box::new(Expr::IdentName {
                    name: "Proxy".into(),
                    ty: Type::Function,
                }),
                args: vec![
                    Arg::Expr(parent.clone()),
                    Arg::Expr(Expr::Object {
                        properties: vec![ObjectProp::Property {
                            key: ObjectPropKey::Static("get".into()),
                            value: get_trap,
                        }],
                        ty: Type::Object,
                    }),
                ],
                ty: Type::Any,
            };
            let is_ctor_probe = Expr::Call {
                callee: Box::new(member_prop(
                    Expr::IdentName {
                        name: "Reflect".into(),
                        ty: Type::Object,
                    },
                    "construct",
                    Type::Function,
                )),
                args: vec![
                    Arg::Expr(empty_ctor),
                    Arg::Expr(Expr::Array {
                        elements: Vec::new(),
                        ty: Type::Any,
                    }),
                    Arg::Expr(proxy_new_target),
                ],
                optional: false,
                ty: Type::Any,
            };
            let try_stmt = Stmt::Try {
                block: vec![Stmt::Expr {
                    expr: is_ctor_probe,
                }],
                handler_param: None,
                handler: Some(vec![throw_not_ctor]),
                finalizer: None,
            };
            (try_stmt, local_expr(pid), Vec::new())
        } else {
            let is_ctor_probe = Expr::Call {
                callee: Box::new(member_prop(
                    Expr::IdentName {
                        name: "Reflect".into(),
                        ty: Type::Object,
                    },
                    "construct",
                    Type::Function,
                )),
                args: vec![
                    Arg::Expr(empty_ctor),
                    Arg::Expr(Expr::Array {
                        elements: Vec::new(),
                        ty: Type::Any,
                    }),
                    Arg::Expr(parent.clone()),
                ],
                optional: false,
                ty: Type::Any,
            };
            let try_stmt = Stmt::Try {
                block: vec![Stmt::Expr {
                    expr: is_ctor_probe,
                }],
                handler_param: None,
                handler: Some(vec![throw_not_ctor]),
                finalizer: None,
            };
            let proto = member_prop(parent, "prototype", Type::Any);
            (try_stmt, proto, Vec::new())
        };

    let proto_is_null = Expr::Binary {
        left: Box::new(proto_for_check.clone()),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::Null { ty: Type::Any }),
        ty: Type::Boolean,
    };
    let typeof_proto = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(proto_for_check.clone()),
        ty: Type::String,
    };
    let proto_is_object = Expr::Binary {
        left: Box::new(typeof_proto.clone()),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::String {
            value: "object".into(),
            ty: Type::String,
        }),
        ty: Type::Boolean,
    };
    let typeof_proto_fn = Expr::Unary {
        op: UnaryOp::TypeOf,
        arg: Box::new(proto_for_check),
        ty: Type::String,
    };
    let proto_is_function = Expr::Binary {
        left: Box::new(typeof_proto_fn),
        op: BinaryOp::EqEqEq,
        right: Box::new(Expr::String {
            value: "function".into(),
            ty: Type::String,
        }),
        ty: Type::Boolean,
    };
    let proto_ok = Expr::Binary {
        left: Box::new(proto_is_null),
        op: BinaryOp::Or,
        right: Box::new(Expr::Binary {
            left: Box::new(proto_is_object),
            op: BinaryOp::Or,
            right: Box::new(proto_is_function),
            ty: Type::Boolean,
        }),
        ty: Type::Boolean,
    };
    let proto_bad = Expr::Unary {
        op: UnaryOp::Not,
        arg: Box::new(proto_ok),
        ty: Type::Boolean,
    };
    let throw_bad_proto = Stmt::Throw {
        value: Expr::New {
            callee: Box::new(Expr::IdentName {
                name: "TypeError".into(),
                ty: Type::Function,
            }),
            args: vec![Arg::Expr(Expr::String {
                value: "Class extends value does not have valid prototype property".into(),
                ty: Type::String,
            })],
            ty: Type::Any,
        },
    };
    let check_proto = Stmt::If {
        test: proto_bad,
        consequent: Box::new(throw_bad_proto),
        alternate: None,
    };

    let mut not_null_body = vec![try_is_ctor];
    not_null_body.extend(after_ctor_stmts);
    not_null_body.push(check_proto);
    // extends null → cached proto is null
    vec![Stmt::If {
        test: not_null,
        consequent: Box::new(Stmt::Block {
            body: not_null_body,
        }),
        alternate: proto_out.map(|pid| {
            Box::new(Stmt::Expr {
                expr: Expr::Assign {
                    target: AssignTarget::Local(pid),
                    op: AssignOp::Eq,
                    value: Box::new(Expr::Null { ty: Type::Any }),
                    ty: Type::Any,
                },
            })
        }),
    }]
}

/// `(parent === null) ? null : parent.prototype` — `extends null` super base (E19.72).
pub(crate) fn parent_instance_super_base(parent: Expr) -> Expr {
    Expr::Conditional {
        test: Box::new(Expr::Binary {
            left: Box::new(parent.clone()),
            op: BinaryOp::EqEqEq,
            right: Box::new(Expr::Null { ty: Type::Any }),
            ty: Type::Boolean,
        }),
        consequent: Box::new(Expr::Null { ty: Type::Any }),
        alternate: Box::new(member_prop(parent, "prototype", Type::Any)),
        ty: Type::Any,
    }
}
