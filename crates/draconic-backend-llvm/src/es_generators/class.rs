use super::eval::*;
use super::exec::*;
use super::*;

pub(super) fn eval_object_lit(
    properties: &[ObjectProp],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let mut props = HashMap::new();
    let methods = HashMap::new();
    for p in properties {
        let ObjectProp::Property { key, value } = p else {
            return Err(Ev::U);
        };
        let name = match key {
            ObjectPropKey::Static(k) => k.to_string_lossy(),
            ObjectPropKey::Computed(kexpr) => {
                let kv = eval_expr(kexpr, env, gen_fns, fn_bind, gens)?;
                key_string(&kv)?
            }
        };
        let v = eval_expr(value, env, gen_fns, fn_bind, gens)?;
        props.insert(name, v);
    }
    Ok(JsVal::Object { props, methods })
}

pub(super) fn lookup_prop(obj: &JsVal, prop: &str) -> Result<JsVal, Ev> {
    match obj {
        JsVal::Result { value, done } => match prop {
            "value" => Ok(*value.clone()),
            "done" => Ok(JsVal::Bool(*done)),
            _ => Err(Ev::U),
        },
        JsVal::Array(elems) => match prop {
            "length" => Ok(JsVal::Num(elems.len() as f64)),
            _ => Err(Ev::U),
        },
        JsVal::Object { props, methods } => {
            if let Some(v) = props.get(prop) {
                return Ok(v.clone());
            }
            if let Some(&fid) = methods.get(prop) {
                return Ok(JsVal::GenFn(fid));
            }
            Err(Ev::U)
        }
        JsVal::Class { statics, .. } => {
            if let Some(&fid) = statics.get(prop) {
                return Ok(JsVal::GenFn(fid));
            }
            Err(Ev::U)
        }
        _ => Err(Ev::U),
    }
}

pub(super) fn eval_new(
    callee: JsVal,
    args: &[Arg],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::Class {
        methods,
        ctor_params,
        ctor_assigns,
        ..
    } = callee
    else {
        return Err(Ev::U);
    };
    if args.len() > ctor_params.len() {
        return Err(Ev::U);
    }
    let mut arg_vals = Vec::new();
    for a in args {
        match a {
            Arg::Expr(e) => arg_vals.push(eval_expr(e, env, gen_fns, fn_bind, gens)?),
            _ => return Err(Ev::U),
        }
    }
    let mut param_env = HashMap::new();
    for (i, pid) in ctor_params.iter().enumerate() {
        let v = if i < arg_vals.len() {
            arg_vals[i].clone()
        } else {
            JsVal::Undef
        };
        param_env.insert(*pid, v);
    }
    let mut props = HashMap::new();
    for (prop, pid) in &ctor_assigns {
        let v = param_env.get(pid).cloned().unwrap_or(JsVal::Undef);
        props.insert(prop.clone(), v);
    }
    Ok(JsVal::Object { props, methods })
}

/// Extract class from builder IIFE body (ctor + proto/static generator methods).
pub(super) fn try_eval_class_iife(
    body: &[Stmt],
    gen_fns: &mut Vec<GenFnRec>,
    _fn_bind: &mut HashMap<LocalId, usize>,
) -> Result<JsVal, ()> {
    let mut ctor_local: Option<LocalId> = None;
    let mut ctor_params: Vec<LocalId> = Vec::new();
    let mut ctor_assigns: Vec<(String, LocalId)> = Vec::new();
    let mut pending_methods: Vec<(bool, String, Vec<LocalId>, Vec<Stmt>)> = Vec::new();
    let mut pending_key: Option<String> = None;
    let mut saw_return_ctor = false;
    let mut saw_any_gen_method = false;

    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            Stmt::Declare {
                local,
                init:
                    Some(Expr::Function {
                        params,
                        body: cbody,
                        is_async: false,
                        is_generator: false,
                        is_arrow: false,
                        ..
                    }),
                ..
            } if ctor_local.is_none() => {
                ctor_params = simple_param_locals(params).ok_or(())?;
                ctor_assigns = extract_ctor_assigns(cbody)?;
                ctor_local = Some(*local);
            }
            Stmt::Declare {
                init: Some(Expr::String { value, .. }),
                ..
            } => {
                pending_key = Some(value.to_string_lossy());
            }
            Stmt::Expr {
                expr: Expr::Call { callee, args, .. },
            } if is_object_define_property(callee) && args.len() == 3 => {
                let ctor = ctor_local.ok_or(())?;
                let key = pending_key
                    .take()
                    .or_else(|| string_arg_key(&args[1]))
                    .ok_or(())?;
                // Skip non-method defines (name, prototype descriptor without method).
                let Some(method_fn) = find_method_function(&args[2]) else {
                    continue;
                };
                let Expr::Function {
                    params,
                    body: mbody,
                    is_generator: true,
                    ..
                } = method_fn
                else {
                    // Non-generator method — not in this adapter's scope.
                    return Err(());
                };
                let param_ids = simple_param_locals(params).ok_or(())?;
                let is_static = if is_define_on_ctor(args, ctor) {
                    true
                } else if is_define_on_proto(args, ctor) {
                    false
                } else {
                    return Err(());
                };
                saw_any_gen_method = true;
                pending_methods.push((is_static, key, param_ids, filter_gen_body(mbody)));
            }
            Stmt::Return {
                value: Some(Expr::Local { id, .. }),
            } if Some(*id) == ctor_local => {
                saw_return_ctor = true;
            }
            // Ignore other class-builder scaffolding (defineProperty name/proto, etc.).
            Stmt::Declare { .. } | Stmt::Expr { .. } | Stmt::If { .. } => {}
            _ => return Err(()),
        }
    }

    if !saw_return_ctor || ctor_local.is_none() || !saw_any_gen_method {
        return Err(());
    }

    let mut methods: HashMap<String, usize> = HashMap::new();
    let mut statics: HashMap<String, usize> = HashMap::new();
    for (is_static, key, params, mbody) in pending_methods {
        let idx = gen_fns.len();
        gen_fns.push(GenFnRec {
            params,
            body: mbody,
            name: None,
        });
        if is_static {
            statics.insert(key, idx);
        } else {
            methods.insert(key, idx);
        }
    }

    Ok(JsVal::Class {
        methods,
        statics,
        ctor_params,
        ctor_assigns,
    })
}

pub(super) fn extract_ctor_assigns(body: &[Stmt]) -> Result<Vec<(String, LocalId)>, ()> {
    let mut out = Vec::new();
    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            Stmt::If { .. } => {} // new.target check
            Stmt::Expr {
                expr:
                    Expr::Assign {
                        target:
                            AssignTarget::Member {
                                object, property, ..
                            },
                        op: AssignOp::Eq,
                        value,
                        ..
                    },
            } => {
                if !matches!(object.as_ref(), Expr::This { .. }) {
                    return Err(());
                }
                let prop = prop_name(property).map_err(|_| ())?;
                let Expr::Local { id, .. } = value.as_ref() else {
                    return Err(());
                };
                out.push((prop, *id));
            }
            _ => {}
        }
    }
    Ok(out)
}

pub(super) fn is_object_define_property(callee: &Expr) -> bool {
    let Expr::Member {
        object, property, ..
    } = callee
    else {
        return false;
    };
    matches!(
        (object.as_ref(), property.as_ref()),
        (
            Expr::IdentName { name, .. },
            Expr::String { value, .. }
        ) if name == "Object" && value.to_string_lossy() == "defineProperty"
    )
}

pub(super) fn is_define_on_ctor(args: &[Arg], ctor: LocalId) -> bool {
    matches!(
        &args[0],
        Arg::Expr(Expr::Local { id, .. }) if *id == ctor
    )
}

pub(super) fn is_define_on_proto(args: &[Arg], ctor: LocalId) -> bool {
    let Arg::Expr(Expr::Member {
        object, property, ..
    }) = &args[0]
    else {
        return false;
    };
    matches!(
        (object.as_ref(), property.as_ref()),
        (
            Expr::Local { id, .. },
            Expr::String { value, .. }
        ) if *id == ctor && value.to_string_lossy() == "prototype"
    )
}

pub(super) fn string_arg_key(arg: &Arg) -> Option<String> {
    match arg {
        Arg::Expr(Expr::String { value, .. }) => Some(value.to_string_lossy()),
        _ => None,
    }
}

pub(super) fn find_method_function(arg: &Arg) -> Option<&Expr> {
    let Arg::Expr(expr) = arg else {
        return None;
    };
    find_method_function_expr(expr)
}

pub(super) fn find_method_function_expr(expr: &Expr) -> Option<&Expr> {
    match expr {
        Expr::Function {
            is_method: true, ..
        } => Some(expr),
        Expr::Function { body, .. } => {
            for s in body {
                if let Some(f) = find_method_function_in_stmt(s) {
                    return Some(f);
                }
            }
            None
        }
        Expr::Call { callee, args, .. } => {
            if let Some(f) = find_method_function_expr(callee) {
                return Some(f);
            }
            for a in args {
                if let Some(f) = find_method_function(a) {
                    return Some(f);
                }
            }
            None
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                if let ObjectProp::Property { value, .. } = p {
                    if let Some(f) = find_method_function_expr(value) {
                        return Some(f);
                    }
                }
            }
            None
        }
        Expr::Member {
            object, property, ..
        } => find_method_function_expr(object).or_else(|| find_method_function_expr(property)),
        Expr::Binary { left, right, .. } => {
            find_method_function_expr(left).or_else(|| find_method_function_expr(right))
        }
        Expr::Assign { value, .. } => find_method_function_expr(value),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => find_method_function_expr(test)
            .or_else(|| find_method_function_expr(consequent))
            .or_else(|| find_method_function_expr(alternate)),
        Expr::Unary { arg, .. } => find_method_function_expr(arg),
        _ => None,
    }
}

pub(super) fn find_method_function_in_stmt(stmt: &Stmt) -> Option<&Expr> {
    match stmt {
        Stmt::Expr { expr } | Stmt::Return { value: Some(expr) } => find_method_function_expr(expr),
        Stmt::Declare {
            init: Some(expr), ..
        } => find_method_function_expr(expr),
        Stmt::Block { body } => {
            for s in body {
                if let Some(f) = find_method_function_in_stmt(s) {
                    return Some(f);
                }
            }
            None
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => find_method_function_expr(test)
            .or_else(|| find_method_function_in_stmt(consequent))
            .or_else(|| {
                alternate
                    .as_ref()
                    .and_then(|a| find_method_function_in_stmt(a))
            }),
        _ => None,
    }
}

pub(super) fn spawn_gen_call(
    callee: JsVal,
    args: &[Arg],
    this_val: Option<JsVal>,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::GenFn(fid) = callee else {
        return Err(Ev::U);
    };
    if fid >= gen_fns.len() {
        return Err(Ev::U);
    }
    let n_params = gen_fns[fid].params.len();
    if args.len() > n_params {
        return Err(Ev::U);
    }
    let mut arg_vals = Vec::new();
    for a in args {
        match a {
            Arg::Expr(e) => arg_vals.push(eval_expr(e, env, gen_fns, fn_bind, gens)?),
            _ => return Err(Ev::U),
        }
    }
    spawn_gen_vals(
        JsVal::GenFn(fid),
        &arg_vals,
        this_val,
        gen_fns,
        fn_bind,
        gens,
    )
}

pub(super) fn bin_val(op: &BinaryOp, left: &JsVal, right: &JsVal) -> Result<JsVal, Ev> {
    match op {
        BinaryOp::Add => match (left, right) {
            (JsVal::Num(a), JsVal::Num(b)) => Ok(JsVal::Num(a + b)),
            (JsVal::Str(a), JsVal::Str(b)) => Ok(JsVal::Str(format!("{a}{b}"))),
            (JsVal::Str(a), JsVal::Num(b)) => Ok(JsVal::Str(format!("{a}{b}"))),
            (JsVal::Num(a), JsVal::Str(b)) => Ok(JsVal::Str(format!("{a}{b}"))),
            _ => Err(Ev::U),
        },
        BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
            let JsVal::Num(a) = left else {
                return Err(Ev::U);
            };
            let JsVal::Num(b) = right else {
                return Err(Ev::U);
            };
            let n = match op {
                BinaryOp::Sub => a - b,
                BinaryOp::Mul => a * b,
                BinaryOp::Div => a / b,
                BinaryOp::Rem => a % b,
                _ => unreachable!(),
            };
            Ok(JsVal::Num(n))
        }
        BinaryOp::EqEqEq => Ok(JsVal::Bool(strict_eq(left, right))),
        BinaryOp::NotEqEq => Ok(JsVal::Bool(!strict_eq(left, right))),
        BinaryOp::Lt => {
            let JsVal::Num(a) = left else {
                return Err(Ev::U);
            };
            let JsVal::Num(b) = right else {
                return Err(Ev::U);
            };
            Ok(JsVal::Bool(a < b))
        }
        _ => Err(Ev::U),
    }
}

pub(super) fn strict_eq(left: &JsVal, right: &JsVal) -> bool {
    match (left, right) {
        (JsVal::Num(a), JsVal::Num(b)) => a == b,
        (JsVal::Bool(a), JsVal::Bool(b)) => a == b,
        (JsVal::Str(a), JsVal::Str(b)) => a == b,
        (JsVal::Undef, JsVal::Undef) => true,
        _ => false,
    }
}

pub(super) fn prop_name(expr: &Expr) -> Result<String, Ev> {
    match expr {
        Expr::String { value, .. } => Ok(value.to_string_lossy()),
        _ => Err(Ev::U),
    }
}

pub(super) fn map_in_gen<T>(r: Result<T, ()>) -> Result<T, Ev> {
    r.map_err(|_| Ev::U)
}

pub(super) fn iter_result(value: JsVal, done: bool) -> JsVal {
    JsVal::Result {
        value: Box::new(value),
        done,
    }
}
