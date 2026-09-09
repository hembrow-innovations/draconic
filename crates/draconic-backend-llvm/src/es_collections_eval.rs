use super::*;

pub(super) fn eval_body(body: &[Stmt], env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, env)? {
            Flow::Normal => {}
            other => return Ok(other),
        }
    }
    Ok(Flow::Normal)
}

fn eval_stmt(stmt: &Stmt, env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => match eval_expr(e, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                },
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Expr { expr } => match eval_expr(expr, env)? {
            Ok(_) => Ok(Flow::Normal),
            Err(flow) => Ok(flow),
        },
        Stmt::Throw { value } => match eval_expr(value, env)? {
            Ok(v) => Ok(Flow::Throw(v)),
            Err(flow) => Ok(flow),
        },
        Stmt::Return { value } => match value {
            Some(e) => match eval_expr(e, env)? {
                Ok(v) => Ok(Flow::Return(v)),
                Err(flow) => Ok(flow),
            },
            None => Ok(Flow::Return(JsVal::Undef)),
        },
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            let mut completion = match eval_body(block, env)? {
                Flow::Throw(exc) => {
                    if let Some(handler) = handler {
                        if let Some(Pattern::Local(pid)) = handler_param {
                            env.insert(*pid, exc);
                        }
                        eval_body(handler, env)?
                    } else {
                        Flow::Throw(exc)
                    }
                }
                other => other,
            };
            if let Some(fin) = finalizer {
                match eval_body(fin, env)? {
                    Flow::Normal => {}
                    abrupt => completion = abrupt,
                }
            }
            Ok(completion)
        }
        Stmt::Block { body } => eval_body(body, env),
        _ => Err(()),
    }
}

fn eval_expr(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<JsVal, Flow>, ()> {
    match expr {
        Expr::Number { raw, .. } => Ok(Ok(JsVal::Num(raw.parse().map_err(|_| ())?))),
        Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
        Expr::String { value, .. } => Ok(Ok(JsVal::Str(js_string_to_utf8(value)))),
        Expr::Null { .. } => Ok(Ok(JsVal::Undef)),
        Expr::Local { id, .. } => Ok(Ok(env.get(id).cloned().ok_or(())?)),
        Expr::IdentName { name, .. } => {
            if name == "undefined" {
                return Ok(Ok(JsVal::Undef));
            }
            Ok(Ok(JsVal::Builtin(builtin_for_name(name).ok_or(())?)))
        }
        Expr::Function { .. } => Ok(Ok(user_fn_from_expr(expr).ok_or(())?)),
        Expr::Array { elements, .. } => {
            let mut items = Vec::new();
            for el in elements {
                let ArrayElement::Expr(e) = el else {
                    return Err(());
                };
                match eval_expr(e, env)? {
                    Ok(v) => items.push(v),
                    Err(flow) => return Ok(Err(flow)),
                }
            }
            Ok(Ok(JsVal::Array(items)))
        }
        Expr::Object { properties, .. } => {
            let mut props = Vec::new();
            for p in properties {
                let ObjectProp::Property {
                    key: ObjectPropKey::Static(k),
                    value,
                } = p
                else {
                    return Err(());
                };
                let v = match eval_expr(value, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                props.push((js_string_to_utf8(k), v));
            }
            Ok(Ok(JsVal::Object(props)))
        }
        Expr::Unary { op, arg, .. } => {
            let v = match eval_expr(arg, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match op {
                UnaryOp::TypeOf => Ok(Ok(JsVal::Str(typeof_str(&v)))),
                UnaryOp::Minus => match v {
                    JsVal::Num(n) => Ok(Ok(JsVal::Num(-n))),
                    _ => Err(()),
                },
                UnaryOp::Plus => match v {
                    JsVal::Num(n) => Ok(Ok(JsVal::Num(n))),
                    _ => Err(()),
                },
                _ => Err(()),
            }
        }
        Expr::Binary {
            op, left, right, ..
        } => {
            let l = match eval_expr(left, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            if *op == BinaryOp::And {
                if !to_boolean(&l) {
                    return Ok(Ok(l));
                }
                return eval_expr(right, env);
            }
            if *op == BinaryOp::Or {
                if to_boolean(&l) {
                    return Ok(Ok(l));
                }
                return eval_expr(right, env);
            }
            let r = match eval_expr(right, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match op {
                BinaryOp::EqEqEq | BinaryOp::EqEq => Ok(Ok(JsVal::Bool(strict_eq(&l, &r)))),
                BinaryOp::NotEqEq | BinaryOp::NotEq => Ok(Ok(JsVal::Bool(!strict_eq(&l, &r)))),
                BinaryOp::Add => match (&l, &r) {
                    (JsVal::Num(a), JsVal::Num(b)) => Ok(Ok(JsVal::Num(a + b))),
                    (JsVal::Str(a), JsVal::Str(b)) => Ok(Ok(JsVal::Str(format!("{a}{b}")))),
                    _ => Err(()),
                },
                BinaryOp::Sub => match (&l, &r) {
                    (JsVal::Num(a), JsVal::Num(b)) => Ok(Ok(JsVal::Num(a - b))),
                    _ => Err(()),
                },
                BinaryOp::Mul => match (&l, &r) {
                    (JsVal::Num(a), JsVal::Num(b)) => Ok(Ok(JsVal::Num(a * b))),
                    _ => Err(()),
                },
                BinaryOp::Div => match (&l, &r) {
                    (JsVal::Num(a), JsVal::Num(b)) => Ok(Ok(JsVal::Num(a / b))),
                    _ => Err(()),
                },
                _ => Err(()),
            }
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            let t = match eval_expr(test, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            if to_boolean(&t) {
                eval_expr(consequent, env)
            } else {
                eval_expr(alternate, env)
            }
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = match eval_expr(object, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let key = match eval_key(property, env)? {
                Ok(k) => k,
                Err(flow) => return Ok(Err(flow)),
            };
            Ok(Ok(member_get(&obj, &key)))
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => arg_vals.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    _ => return Err(()),
                }
            }
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match eval_call_fn(&c, &arg_vals, env) {
                Ok(v) => Ok(Ok(v)),
                Err(Some(flow)) => Ok(Err(flow)),
                Err(None) => Err(()),
            }
        }
        Expr::New { callee, args, .. } => {
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => arg_vals.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    _ => return Err(()),
                }
            }
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match eval_new(&c, &arg_vals) {
                Ok(v) => Ok(Ok(v)),
                Err(Some(flow)) => Ok(Err(flow)),
                Err(None) => Err(()),
            }
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = match eval_expr(value, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            env.insert(*id, v.clone());
            Ok(Ok(v))
        }
        _ => Err(()),
    }
}

fn eval_key(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<String, Flow>, ()> {
    match expr {
        Expr::String { value, .. } => Ok(Ok(js_string_to_utf8(value))),
        e => match eval_expr(e, env)? {
            Ok(v) => Ok(Ok(to_key_string(&v))),
            Err(flow) => Ok(Err(flow)),
        },
    }
}

fn user_fn_from_expr(expr: &Expr) -> Option<JsVal> {
    match expr {
        Expr::Function {
            name: None,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } if simple_fn_params_ok(params) && body_ok(body) => {
            let ids = params
                .iter()
                .map(|p| match &p.pattern {
                    Pattern::Local(id) => *id,
                    _ => unreachable!(),
                })
                .collect();
            Some(JsVal::UserFn {
                params: ids,
                body: body.clone(),
            })
        }
        _ => None,
    }
}

fn type_error(msg: &str) -> Flow {
    Flow::Throw(JsVal::ErrorInst {
        name: "TypeError".into(),
        message: msg.into(),
    })
}

fn range_error(msg: &str) -> Flow {
    Flow::Throw(JsVal::ErrorInst {
        name: "RangeError".into(),
        message: msg.into(),
    })
}

fn eval_call_fn(
    callee: &JsVal,
    args: &[JsVal],
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<JsVal, Option<Flow>> {
    match callee {
        JsVal::Builtin(BuiltinId::GroupBy) => group_by(args, env),
        JsVal::Builtin(BuiltinId::Chunk) => chunk(args),
        JsVal::Builtin(BuiltinId::Deque) => Ok(make_deque()),
        JsVal::DequeMethod { kind, items } => call_deque_method(*kind, items, args),
        JsVal::UserFn { params, body } => call_user_fn(params, body, args, env),
        _ => Err(None),
    }
}

fn eval_new(callee: &JsVal, _args: &[JsVal]) -> Result<JsVal, Option<Flow>> {
    match callee {
        JsVal::Builtin(BuiltinId::Deque) => Ok(make_deque()),
        _ => Err(None),
    }
}

fn make_deque() -> JsVal {
    JsVal::Deque(Rc::new(RefCell::new(VecDeque::new())))
}

fn call_deque_method(
    kind: DequeOp,
    items: &Rc<RefCell<VecDeque<JsVal>>>,
    args: &[JsVal],
) -> Result<JsVal, Option<Flow>> {
    match kind {
        DequeOp::PushBack => {
            items
                .borrow_mut()
                .push_back(args.first().cloned().unwrap_or(JsVal::Undef));
            Ok(JsVal::Undef)
        }
        DequeOp::PushFront => {
            items
                .borrow_mut()
                .push_front(args.first().cloned().unwrap_or(JsVal::Undef));
            Ok(JsVal::Undef)
        }
        DequeOp::PopBack => Ok(items.borrow_mut().pop_back().unwrap_or(JsVal::Undef)),
        DequeOp::PopFront => Ok(items.borrow_mut().pop_front().unwrap_or(JsVal::Undef)),
    }
}

fn call_user_fn(
    params: &[LocalId],
    body: &[Stmt],
    args: &[JsVal],
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<JsVal, Option<Flow>> {
    for (i, pid) in params.iter().enumerate() {
        env.insert(*pid, args.get(i).cloned().unwrap_or(JsVal::Undef));
    }
    match eval_body(body, env) {
        Ok(Flow::Return(v)) => Ok(v),
        Ok(Flow::Normal) => Ok(JsVal::Undef),
        Ok(Flow::Throw(e)) => Err(Some(Flow::Throw(e))),
        Err(()) => Err(None),
    }
}

fn group_by(args: &[JsVal], env: &mut HashMap<LocalId, JsVal>) -> Result<JsVal, Option<Flow>> {
    let items = match args.first() {
        Some(JsVal::Array(a)) => a,
        _ => return Err(Some(type_error("groupBy expects an array"))),
    };
    enum Mode<'a> {
        Id,
        Fn {
            params: &'a [LocalId],
            body: &'a [Stmt],
        },
        Prop(&'a str),
    }
    let mode = match args.get(1) {
        None | Some(JsVal::Undef) => Mode::Id,
        Some(JsVal::UserFn { params, body }) => Mode::Fn { params, body },
        Some(JsVal::Str(s)) => Mode::Prop(s),
        _ => return Err(Some(type_error("groupBy key must be a function or string"))),
    };
    let mut out: Vec<(String, Vec<JsVal>)> = Vec::new();
    for (i, item) in items.iter().enumerate() {
        let kraw = match &mode {
            Mode::Id => item.clone(),
            Mode::Fn { params, body } => {
                call_user_fn(params, body, &[item.clone(), JsVal::Num(i as f64)], env)?
            }
            Mode::Prop(p) => member_get(item, p),
        };
        let ks = to_key_string(&kraw);
        if let Some((_, bucket)) = out.iter_mut().find(|(k, _)| k == &ks) {
            bucket.push(item.clone());
        } else {
            out.push((ks, vec![item.clone()]));
        }
    }
    Ok(JsVal::Object(
        out.into_iter().map(|(k, v)| (k, JsVal::Array(v))).collect(),
    ))
}

fn chunk(args: &[JsVal]) -> Result<JsVal, Option<Flow>> {
    let items = match args.first() {
        Some(JsVal::Array(a)) => a,
        _ => return Err(Some(type_error("chunk expects an array"))),
    };
    let size = match args.get(1) {
        Some(JsVal::Num(n)) if n.is_finite() && *n > 0.0 && n.fract() == 0.0 => *n as usize,
        _ => return Err(Some(range_error("chunk size must be a positive integer"))),
    };
    let mut out = Vec::new();
    let mut i = 0;
    while i < items.len() {
        let end = (i + size).min(items.len());
        out.push(JsVal::Array(items[i..end].to_vec()));
        i = end;
    }
    Ok(JsVal::Array(out))
}

fn member_get(obj: &JsVal, key: &str) -> JsVal {
    match obj {
        JsVal::Builtin(BuiltinId::GlobalThis) => match key {
            "groupBy" => JsVal::Builtin(BuiltinId::GroupBy),
            "chunk" => JsVal::Builtin(BuiltinId::Chunk),
            "Deque" => JsVal::Builtin(BuiltinId::Deque),
            "Array" => JsVal::Builtin(BuiltinId::Array),
            "globalThis" => JsVal::Builtin(BuiltinId::GlobalThis),
            _ => JsVal::Undef,
        },
        JsVal::Deque(items) => match key {
            "length" => JsVal::Num(items.borrow().len() as f64),
            "pushBack" => JsVal::DequeMethod {
                kind: DequeOp::PushBack,
                items: items.clone(),
            },
            "pushFront" => JsVal::DequeMethod {
                kind: DequeOp::PushFront,
                items: items.clone(),
            },
            "popBack" => JsVal::DequeMethod {
                kind: DequeOp::PopBack,
                items: items.clone(),
            },
            "popFront" => JsVal::DequeMethod {
                kind: DequeOp::PopFront,
                items: items.clone(),
            },
            _ => JsVal::Undef,
        },
        JsVal::ErrorInst { name, message } => match key {
            "name" => JsVal::Str(name.clone()),
            "message" => JsVal::Str(message.clone()),
            _ => JsVal::Undef,
        },
        JsVal::Array(items) if key == "length" => JsVal::Num(items.len() as f64),
        JsVal::Array(items) => key
            .parse::<usize>()
            .ok()
            .and_then(|i| items.get(i).cloned())
            .unwrap_or(JsVal::Undef),
        JsVal::Object(props) => props
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .unwrap_or(JsVal::Undef),
        _ => JsVal::Undef,
    }
}

fn to_key_string(v: &JsVal) -> String {
    match v {
        JsVal::Str(s) => s.clone(),
        JsVal::Num(n) => num_to_string(*n),
        JsVal::Bool(true) => "true".into(),
        JsVal::Bool(false) => "false".into(),
        JsVal::Undef => "undefined".into(),
        _ => "[object Object]".into(),
    }
}

fn num_to_string(n: f64) -> String {
    if n.is_nan() {
        "NaN".into()
    } else if n.is_infinite() {
        if n.is_sign_positive() {
            "Infinity".into()
        } else {
            "-Infinity".into()
        }
    } else if n == 0.0 {
        "0".into()
    } else if n.fract() == 0.0 && n.abs() < 1e15 {
        format!("{}", n as i64)
    } else {
        format!("{n}")
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::ErrorInst { .. }
        | JsVal::Array(_)
        | JsVal::Object(_)
        | JsVal::Deque(_)
        | JsVal::Builtin(BuiltinId::GlobalThis) => "object".into(),
        JsVal::Builtin(
            BuiltinId::GroupBy | BuiltinId::Chunk | BuiltinId::Deque | BuiltinId::Array,
        )
        | JsVal::UserFn { .. }
        | JsVal::DequeMethod { .. } => "function".into(),
    }
}

fn strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => x == y,
        (JsVal::Bool(x), JsVal::Bool(y)) => x == y,
        (JsVal::Str(x), JsVal::Str(y)) => x == y,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Builtin(x), JsVal::Builtin(y)) => x == y,
        _ => false,
    }
}

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef => false,
        _ => true,
    }
}

fn js_string_to_utf8(s: &JsString) -> String {
    s.to_string_lossy()
}
