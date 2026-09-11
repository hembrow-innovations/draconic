use super::*;

fn body_returns_fn(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::Return {
            value: Some(Expr::Function { .. }),
        } => true,
        Stmt::Block { body } => body_returns_fn(body),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            body_returns_fn(std::slice::from_ref(consequent))
                || alternate
                    .as_ref()
                    .is_some_and(|a| body_returns_fn(std::slice::from_ref(a)))
        }
        _ => false,
    })
}

fn nested_rest_locals(
    params: &[Param],
    by_id: &HashMap<LocalId, &Local>,
) -> Option<HashSet<LocalId>> {
    let (_, _, rest) = simple_params(params, by_id)?;
    let mut s = HashSet::new();
    if let Some(r) = rest {
        s.insert(r);
    }
    Some(s)
}

pub(super) fn fn_body_ok(
    body: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    obj_methods: &HashMap<LocalId, HashMap<String, usize>>,
    rest_locals: &HashSet<LocalId>,
) -> bool {
    body.iter().all(|s| match s {
        Stmt::Return { value: Some(v) } => match v {
            Expr::Function {
                is_async,
                is_generator,
                params,
                body,
                ..
            } => {
                !*is_async
                    && !*is_generator
                    && simple_params(params, by_id).is_some()
                    && nested_rest_locals(params, by_id).is_some_and(|rl| {
                        fn_body_ok(
                            body,
                            by_id,
                            fn_arities,
                            functions,
                            fn_binding,
                            obj_methods,
                            &rl,
                        )
                    })
            }
            _ => number_expr_ok(v, by_id, fn_arities, functions, fn_binding, obj_methods),
        },
        Stmt::Return { value: None } => false,
        Stmt::Block { body } => fn_body_ok(
            body,
            by_id,
            fn_arities,
            functions,
            fn_binding,
            obj_methods,
            rest_locals,
        ),
        Stmt::Declare { local, init, .. } => {
            let Some(loc) = by_id.get(local) else {
                return false;
            };
            if !matches!(loc.ty, Type::Number | Type::Any | Type::Function) {
                return false;
            }
            match init {
                Some(Expr::Function {
                    is_async,
                    is_generator,
                    params,
                    body,
                    ..
                }) => {
                    !*is_async
                        && !*is_generator
                        && simple_params(params, by_id).is_some()
                        && nested_rest_locals(params, by_id).is_some_and(|rl| {
                            fn_body_ok(
                                body,
                                by_id,
                                fn_arities,
                                functions,
                                fn_binding,
                                obj_methods,
                                &rl,
                            )
                        })
                }
                Some(e) => number_expr_ok(e, by_id, fn_arities, functions, fn_binding, obj_methods),
                None => true,
            }
        }
        Stmt::Function {
            params,
            body,
            is_async,
            is_generator,
            ..
        } => {
            if *is_async || *is_generator {
                return false;
            }
            simple_params(params, by_id).is_some()
                && nested_rest_locals(params, by_id).is_some_and(|rl| {
                    fn_body_ok(
                        body,
                        by_id,
                        fn_arities,
                        functions,
                        fn_binding,
                        obj_methods,
                        &rl,
                    )
                })
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            bool_expr_ok(test, by_id, fn_arities, functions, fn_binding, obj_methods)
                && fn_body_ok(
                    std::slice::from_ref(consequent),
                    by_id,
                    fn_arities,
                    functions,
                    fn_binding,
                    obj_methods,
                    rest_locals,
                )
                && alternate.as_ref().is_none_or(|a| {
                    fn_body_ok(
                        std::slice::from_ref(a),
                        by_id,
                        fn_arities,
                        functions,
                        fn_binding,
                        obj_methods,
                        rest_locals,
                    )
                })
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
        } => {
            if *is_await {
                return false;
            }
            let Expr::Local { id, .. } = right else {
                return false;
            };
            if !rest_locals.contains(id) {
                return false;
            }
            matches!(left.as_ref(), Stmt::Declare { init: None, .. })
                && fn_body_ok(
                    std::slice::from_ref(body),
                    by_id,
                    fn_arities,
                    functions,
                    fn_binding,
                    obj_methods,
                    rest_locals,
                )
        }
        Stmt::Expr { expr } => {
            if crate::es_console::console_log_string_arg(expr, by_id).is_some() {
                true
            } else {
                match expr {
                    Expr::Assign {
                        target: AssignTarget::Local(_),
                        op: AssignOp::Eq,
                        value,
                        ..
                    } => {
                        number_expr_ok(value, by_id, fn_arities, functions, fn_binding, obj_methods)
                    }
                    _ => false,
                }
            }
        }
        Stmt::Labeled { body, .. } => fn_body_ok(
            std::slice::from_ref(body),
            by_id,
            fn_arities,
            functions,
            fn_binding,
            obj_methods,
            rest_locals,
        ),
        _ => false,
    })
}

pub(super) fn bool_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    obj_methods: &HashMap<LocalId, HashMap<String, usize>>,
) -> bool {
    match expr {
        Expr::Boolean { .. } => true,
        Expr::Binary {
            left, op, right, ..
        } => {
            use draconic_ast::BinaryOp::*;
            matches!(op, Lt | LtEq | Gt | GtEq | EqEq | NotEq | EqEqEq | NotEqEq)
                && number_expr_ok(left, by_id, fn_arities, functions, fn_binding, obj_methods)
                && number_expr_ok(right, by_id, fn_arities, functions, fn_binding, obj_methods)
        }
        _ => number_expr_ok(expr, by_id, fn_arities, functions, fn_binding, obj_methods),
    }
}

pub(super) fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    fn_arities: &HashMap<LocalId, usize>,
    functions: &[FnInfo],
    fn_binding: &HashMap<LocalId, usize>,
    obj_methods: &HashMap<LocalId, HashMap<String, usize>>,
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::IdentName { name, .. } => number_id_named(name, by_id).is_some_and(|id| {
            !fn_arities.contains_key(&id) && !functions.iter().any(|f| f.rest == Some(id))
        }),
        Expr::Local { id, ty } => {
            if fn_arities.contains_key(id) {
                return false;
            }
            if functions.iter().any(|f| f.rest == Some(*id)) {
                return false;
            }
            matches!(ty, Type::Number | Type::Any)
                && by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::Unary {
            op: draconic_ast::UnaryOp::Plus | draconic_ast::UnaryOp::Minus,
            arg,
            ..
        } => number_expr_ok(arg, by_id, fn_arities, functions, fn_binding, obj_methods),
        Expr::Unary {
            op: draconic_ast::UnaryOp::Void,
            arg,
            ..
        } => {
            number_expr_ok(arg, by_id, fn_arities, functions, fn_binding, obj_methods)
                || matches!(arg.as_ref(), Expr::Number { .. } | Expr::Local { .. })
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            use draconic_ast::BinaryOp::*;
            matches!(op, Add | Sub | Mul | Div | Rem)
                && number_expr_ok(left, by_id, fn_arities, functions, fn_binding, obj_methods)
                && number_expr_ok(right, by_id, fn_arities, functions, fn_binding, obj_methods)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return false;
            }
            if !args.iter().all(|a| match a {
                Arg::Expr(e) => {
                    number_expr_ok(e, by_id, fn_arities, functions, fn_binding, obj_methods)
                }
                Arg::Spread(_) => false,
            }) {
                return false;
            }
            match callee.as_ref() {
                Expr::Local { .. } | Expr::IdentName { .. } => {
                    let Some(id) = callee_fn_id(callee, by_id, fn_binding) else {
                        return false;
                    };
                    let Some(&idx) = fn_binding.get(&id) else {
                        return fn_arities.get(&id).is_some_and(|n| args.len() <= *n);
                    };
                    call_arity_ok(&functions[idx], args.len())
                }
                Expr::Member {
                    object,
                    property,
                    computed,
                    optional: opt_m,
                    ..
                } => {
                    if *opt_m || *computed {
                        return false;
                    }
                    let Expr::Local { id: oid, .. } = object.as_ref() else {
                        return false;
                    };
                    let Some(name) = static_prop_name(property) else {
                        return false;
                    };
                    let Some(&idx) = obj_methods.get(oid).and_then(|m| m.get(&name)) else {
                        return false;
                    };
                    call_arity_ok(&functions[idx], args.len())
                }
                Expr::Function {
                    params,
                    is_async,
                    is_generator,
                    body,
                    ..
                } => {
                    !*is_async
                        && !*is_generator
                        && simple_params(params, by_id).is_some_and(|(_, defaults, rest)| {
                            call_arity_ok_params(
                                &defaults,
                                rest.is_some(),
                                find_arguments_local(body, by_id).is_some(),
                                args.len(),
                            ) && nested_rest_locals(params, by_id).is_some_and(|rl| {
                                fn_body_ok(
                                    body,
                                    by_id,
                                    fn_arities,
                                    functions,
                                    fn_binding,
                                    obj_methods,
                                    &rl,
                                )
                            })
                        })
                }
                Expr::Call {
                    callee: inner,
                    args: inner_args,
                    optional: opt2,
                    ..
                } => {
                    if *opt2 {
                        return false;
                    }
                    if !inner_args.iter().all(|a| match a {
                        Arg::Expr(e) => {
                            number_expr_ok(e, by_id, fn_arities, functions, fn_binding, obj_methods)
                        }
                        Arg::Spread(_) => false,
                    }) {
                        return false;
                    }
                    let Some(id) = callee_fn_id(inner, by_id, fn_binding) else {
                        return false;
                    };
                    let Some(&caller_idx) = fn_binding.get(&id) else {
                        return false;
                    };
                    let f = &functions[caller_idx];
                    if !body_returns_fn(&f.body) {
                        return false;
                    }
                    let Some(ret_idx) = returned_fn_idx_in_body(&f.body, functions) else {
                        return false;
                    };
                    call_arity_ok(&functions[ret_idx], args.len())
                }
                _ => false,
            }
        }
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } => {
            if *optional {
                return false;
            }
            let Expr::Local { id, .. } = object.as_ref() else {
                return false;
            };
            if by_id.get(id).map(|l| l.name.as_str()) != Some("arguments") {
                return false;
            }
            if !*computed {
                // arguments.length
                matches!(
                    property.as_ref(),
                    Expr::String { value, .. } if value.to_string_lossy() == "length"
                )
            } else {
                // arguments[N] with constant non-neg index
                match property.as_ref() {
                    Expr::Number { raw, .. } => {
                        parse_nonneg_index(raw).is_some_and(|i| i < MAX_ARGS)
                    }
                    _ => false,
                }
            }
        }
        _ => false,
    }
}

pub(super) fn returned_fn_idx_in_body(body: &[Stmt], functions: &[FnInfo]) -> Option<usize> {
    for s in body {
        match s {
            Stmt::Return {
                value: Some(Expr::Function { params, .. }),
            } => {
                return find_fn_idx_by_param_patterns(params, functions);
            }
            Stmt::Block { body } => {
                if let Some(i) = returned_fn_idx_in_body(body, functions) {
                    return Some(i);
                }
            }
            Stmt::If {
                consequent,
                alternate,
                ..
            } => {
                if let Some(i) =
                    returned_fn_idx_in_body(std::slice::from_ref(consequent), functions)
                {
                    return Some(i);
                }
                if let Some(a) = alternate {
                    if let Some(i) = returned_fn_idx_in_body(std::slice::from_ref(a), functions) {
                        return Some(i);
                    }
                }
            }
            _ => {}
        }
    }
    None
}
