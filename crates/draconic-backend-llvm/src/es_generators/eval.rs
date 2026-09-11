use super::class::*;
use super::exec::*;
use super::*;

pub(super) fn eval_body(
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<(), ()> {
    for stmt in body.iter() {
        match eval_stmt(stmt, env, gen_fns, fn_bind, gens) {
            Ok(Flow::Next) => {}
            Ok(Flow::Break | Flow::Continue | Flow::Return(_) | Flow::Throw(_)) => {
                return Err(());
            }
            Err(()) => {
                return Err(());
            }
        }
    }
    Ok(())
}

pub(super) fn eval_stmt(
    stmt: &Stmt,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<Flow, ()> {
    match stmt {
        Stmt::Function {
            local,
            params,
            body,
            is_async: true,
            is_generator: false,
            ..
        } => {
            let param_ids = simple_param_locals(params).ok_or(())?;
            env.insert(
                *local,
                JsVal::AsyncFn {
                    params: param_ids,
                    body: filter_gen_body(body),
                },
            );
            Ok(Flow::Next)
        }
        Stmt::Function { .. } => Ok(Flow::Next),
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => match eval_expr(e, env, gen_fns, fn_bind, gens) {
                    Ok(v) => v,
                    Err(Ev::Throw(exc)) => return Ok(Flow::Throw(exc)),
                    Err(Ev::U) => return Err(()),
                },
                None => JsVal::Undef,
            };
            if let JsVal::GenFn(idx) = &v {
                fn_bind.insert(*local, *idx);
            }
            env.insert(*local, v);
            Ok(Flow::Next)
        }
        Stmt::Expr { expr } => match eval_expr(expr, env, gen_fns, fn_bind, gens) {
            Ok(_) => Ok(Flow::Next),
            Err(Ev::Throw(exc)) => Ok(Flow::Throw(exc)),
            Err(Ev::U) => Err(()),
        },
        Stmt::Block { body } => eval_block(body, env, gen_fns, fn_bind, gens),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = match eval_expr(test, env, gen_fns, fn_bind, gens) {
                Ok(v) => v,
                Err(Ev::Throw(exc)) => return Ok(Flow::Throw(exc)),
                Err(Ev::U) => return Err(()),
            };
            if is_truthy(&t) {
                eval_stmt(consequent, env, gen_fns, fn_bind, gens)
            } else if let Some(alt) = alternate {
                eval_stmt(alt, env, gen_fns, fn_bind, gens)
            } else {
                Ok(Flow::Next)
            }
        }
        Stmt::Break { label: None } => Ok(Flow::Break),
        Stmt::Continue { label: None } => Ok(Flow::Continue),
        Stmt::Return { value: Some(e) } => {
            let v = match eval_expr(e, env, gen_fns, fn_bind, gens) {
                Ok(v) => v,
                Err(Ev::Throw(exc)) => return Ok(Flow::Throw(exc)),
                Err(Ev::U) => return Err(()),
            };
            Ok(Flow::Return(v))
        }
        Stmt::Return { value: None } => Ok(Flow::Return(JsVal::Undef)),
        Stmt::ForOf {
            left,
            right,
            body,
            is_await: _,
        } => {
            eval_for_of(left, right, body, env, gen_fns, fn_bind, gens)?;
            Ok(Flow::Next)
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            let mut completion = match eval_block(block, env, gen_fns, fn_bind, gens)? {
                Flow::Throw(exc) => {
                    if let Some(handler) = handler {
                        if let Some(Pattern::Local(pid)) = handler_param {
                            env.insert(*pid, exc);
                        } else if handler_param.is_some() {
                            return Err(());
                        }
                        eval_block(handler, env, gen_fns, fn_bind, gens)?
                    } else {
                        Flow::Throw(exc)
                    }
                }
                other => other,
            };
            if let Some(fin) = finalizer {
                match eval_block(fin, env, gen_fns, fn_bind, gens)? {
                    Flow::Next => {}
                    abrupt => completion = abrupt,
                }
            }
            Ok(completion)
        }
        _ => Err(()),
    }
}

pub(super) fn eval_block(
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, env, gen_fns, fn_bind, gens)? {
            Flow::Next => {}
            flow => return Ok(flow),
        }
    }
    Ok(Flow::Next)
}

/// Run statements until completion or nested `return` / throw / unsupported.
pub(super) fn eval_fn_body_stmts(
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    for stmt in body {
        match stmt {
            Stmt::Function { .. } => {}
            other => match eval_stmt(other, env, gen_fns, fn_bind, gens) {
                Ok(Flow::Next) => {}
                Ok(Flow::Return(v)) => return Ok(v),
                Ok(Flow::Throw(exc)) => return Err(Ev::Throw(exc)),
                Ok(Flow::Break | Flow::Continue) | Err(()) => return Err(Ev::U),
            },
        }
    }
    Ok(JsVal::Undef)
}

pub(super) fn is_truthy(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef => false,
        _ => true,
    }
}

pub(super) fn bind_for_of_left(
    left: &Stmt,
    value: JsVal,
    env: &mut HashMap<LocalId, JsVal>,
) -> Result<(), ()> {
    match left {
        Stmt::Declare {
            local, init: None, ..
        } => {
            env.insert(*local, value);
            Ok(())
        }
        Stmt::Expr {
            expr: Expr::Local { id, .. },
        } => {
            env.insert(*id, value);
            Ok(())
        }
        _ => Err(()),
    }
}

pub(super) fn eval_for_of(
    left: &Stmt,
    right: &Expr,
    body: &Stmt,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<(), ()> {
    let iterable = match eval_expr(right, env, gen_fns, fn_bind, gens) {
        Ok(v) => v,
        Err(_) => return Err(()),
    };
    match iterable {
        JsVal::GenInst(idx) => loop {
            let r = match gen_next(&JsVal::GenInst(idx), JsVal::Undef, gen_fns, fn_bind, gens) {
                Ok(v) => v,
                Err(_) => return Err(()),
            };
            let (value, done) = iter_result_parts(r).map_err(|_| ())?;
            if done {
                break;
            }
            bind_for_of_left(left, value, env)?;
            match eval_stmt(body, env, gen_fns, fn_bind, gens)? {
                Flow::Next | Flow::Continue => {}
                Flow::Break => break,
                Flow::Throw(_) | Flow::Return(_) => return Err(()),
            }
        },
        JsVal::Array(elems) => {
            for el in elems {
                bind_for_of_left(left, el, env)?;
                match eval_stmt(body, env, gen_fns, fn_bind, gens)? {
                    Flow::Next | Flow::Continue => {}
                    Flow::Break => break,
                    Flow::Throw(_) | Flow::Return(_) => return Err(()),
                }
            }
        }
        JsVal::Object { .. } => {
            // Custom async iterable: obj[Symbol.asyncIterator]().next() loop.
            let get_iter = match lookup_prop(&iterable, "@@asyncIterator") {
                Ok(v) => v,
                Err(_) => return Err(()),
            };
            let iterator = match call_value(get_iter, &[], env, gen_fns, fn_bind, gens) {
                Ok(v) => v,
                Err(_) => return Err(()),
            };
            loop {
                let next_fn = match lookup_prop(&iterator, "next") {
                    Ok(v) => v,
                    Err(_) => return Err(()),
                };
                let r = match call_value(next_fn, &[], env, gen_fns, fn_bind, gens) {
                    Ok(v) => v,
                    Err(_) => return Err(()),
                };
                let (value, done) = iter_result_parts(r).map_err(|_| ())?;
                if done {
                    break;
                }
                bind_for_of_left(left, value, env)?;
                match eval_stmt(body, env, gen_fns, fn_bind, gens)? {
                    Flow::Next | Flow::Continue => {}
                    Flow::Break => break,
                    Flow::Throw(_) | Flow::Return(_) => return Err(()),
                }
            }
        }
        _ => return Err(()),
    }
    Ok(())
}

/// Unpack `{ value, done }` from `JsVal::Result` or plain object props.
pub(super) fn iter_result_parts(r: JsVal) -> Result<(JsVal, bool), Ev> {
    match r {
        JsVal::Result { value, done } => Ok((*value, done)),
        JsVal::Object { props, .. } => {
            let done = match props.get("done") {
                Some(JsVal::Bool(b)) => *b,
                _ => return Err(Ev::U),
            };
            let value = props.get("value").cloned().unwrap_or(JsVal::Undef);
            Ok((value, done))
        }
        _ => Err(Ev::U),
    }
}

pub(super) fn call_value(
    callee: JsVal,
    args: &[Arg],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    match callee {
        JsVal::AsyncFn { params, body } => {
            call_async_fn(&params, &body, args, env, gen_fns, fn_bind, gens)
        }
        other => spawn_gen_call(other, args, None, env, gen_fns, fn_bind, gens),
    }
}

pub(super) fn key_string(v: &JsVal) -> Result<String, Ev> {
    match v {
        JsVal::Str(s) => Ok(s.clone()),
        JsVal::SymKey(s) => Ok(format!("@@{s}")),
        JsVal::Num(n) if n.is_finite() && *n >= 0.0 && n.fract() == 0.0 => {
            Ok(format!("{}", *n as u64))
        }
        _ => Err(Ev::U),
    }
}

pub(super) fn register_gen_fn_expr(
    name: Option<LocalId>,
    params: &[Param],
    body: &[Stmt],
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
) -> Result<JsVal, Ev> {
    let param_ids = simple_param_locals(params).ok_or(Ev::U)?;
    let idx = gen_fns.len();
    gen_fns.push(GenFnRec {
        params: param_ids,
        body: filter_gen_body(body),
        name,
    });
    if let Some(n) = name {
        fn_bind.insert(n, idx);
    }
    Ok(JsVal::GenFn(idx))
}

pub(super) fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| Ev::U)?;
            Ok(JsVal::Num(n))
        }
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::String { value, .. } => Ok(JsVal::Str(value.to_string_lossy())),
        Expr::Local { id, .. } => env.get(id).cloned().ok_or(Ev::U),
        Expr::This { .. } => Err(Ev::U),
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => Ok(JsVal::Undef),
        Expr::Array { elements, .. } => {
            let mut vals = Vec::new();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => vals.push(eval_expr(e, env, gen_fns, fn_bind, gens)?),
                    _ => return Err(Ev::U),
                }
            }
            Ok(JsVal::Array(vals))
        }
        Expr::Object { properties, .. } => eval_object_lit(properties, env, gen_fns, fn_bind, gens),
        Expr::Binary {
            left, op, right, ..
        } => {
            let l = eval_expr(left, env, gen_fns, fn_bind, gens)?;
            let r = eval_expr(right, env, gen_fns, fn_bind, gens)?;
            bin_val(op, &l, &r)
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = eval_expr(value, env, gen_fns, fn_bind, gens)?;
            env.insert(*id, v.clone());
            Ok(v)
        }
        Expr::Function {
            name,
            params,
            body,
            is_generator: true,
            is_arrow: false,
            ..
        } => register_gen_fn_expr(*name, params, body, gen_fns, fn_bind),
        Expr::Function {
            params,
            body,
            is_async: true,
            is_generator: false,
            is_arrow: false,
            ..
        } => {
            let param_ids = simple_param_locals(params).ok_or(Ev::U)?;
            Ok(JsVal::AsyncFn {
                params: param_ids,
                body: filter_gen_body(body),
            })
        }
        // Plain function expression (object methods, async-iterator factories).
        Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            is_arrow: false,
            ..
        } => {
            let param_ids = simple_param_locals(params).ok_or(Ev::U)?;
            Ok(JsVal::AsyncFn {
                params: param_ids,
                body: filter_gen_body(body),
            })
        }
        Expr::Unary {
            op: UnaryOp::Await,
            arg,
            ..
        } => eval_expr(arg, env, gen_fns, fn_bind, gens),
        // Class builder IIFE: `(function(){ … return ctor })()`
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } if args.is_empty() => {
            if let Expr::Function {
                params,
                body,
                is_async: false,
                is_generator: false,
                is_arrow: false,
                ..
            } = callee.as_ref()
            {
                if params.is_empty() {
                    if let Ok(cls) = try_eval_class_iife(body, gen_fns, fn_bind) {
                        return Ok(cls);
                    }
                }
            }
            eval_call(callee, args, env, gen_fns, fn_bind, gens)
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => eval_call(callee, args, env, gen_fns, fn_bind, gens),
        Expr::New { callee, args, .. } => {
            let c = eval_expr(callee, env, gen_fns, fn_bind, gens)?;
            eval_new(c, args, env, gen_fns, fn_bind, gens)
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = eval_expr(object, env, gen_fns, fn_bind, gens)?;
            if let Ok(prop) = prop_name(property) {
                return lookup_prop(&obj, &prop);
            }
            let key = eval_expr(property, env, gen_fns, fn_bind, gens)?;
            match (&obj, &key) {
                (JsVal::Array(elems), JsVal::Num(i))
                    if i.is_finite() && *i >= 0.0 && i.fract() == 0.0 =>
                {
                    let idx = *i as usize;
                    Ok(elems.get(idx).cloned().unwrap_or(JsVal::Undef))
                }
                _ => {
                    let prop = key_string(&key)?;
                    lookup_prop(&obj, &prop)
                }
            }
        }
        _ => Err(Ev::U),
    }
}

pub(super) fn eval_call(
    callee: &Expr,
    args: &[Arg],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    // Method call: `it.next()` / `obj.m(args)` / `C.sgen(args)` / `.then` / `Promise.resolve`.
    if let Expr::Member {
        object,
        property,
        optional: false,
        ..
    } = callee
    {
        let obj = eval_expr(object, env, gen_fns, fn_bind, gens)?;
        let prop = prop_name(property)?;
        if prop == "next" && matches!(&obj, JsVal::GenInst(_)) {
            let resume = if args.is_empty() {
                JsVal::Undef
            } else if args.len() == 1 {
                match &args[0] {
                    Arg::Expr(e) => eval_expr(e, env, gen_fns, fn_bind, gens)?,
                    _ => return Err(Ev::U),
                }
            } else {
                return Err(Ev::U);
            };
            return gen_next(&obj, resume, gen_fns, fn_bind, gens);
        }
        if prop == "return" {
            let val = if args.is_empty() {
                JsVal::Undef
            } else if args.len() == 1 {
                match &args[0] {
                    Arg::Expr(e) => eval_expr(e, env, gen_fns, fn_bind, gens)?,
                    _ => return Err(Ev::U),
                }
            } else {
                return Err(Ev::U);
            };
            return gen_return(&obj, val, gen_fns, fn_bind, gens);
        }
        if prop == "throw" {
            let val = if args.is_empty() {
                JsVal::Undef
            } else if args.len() == 1 {
                match &args[0] {
                    Arg::Expr(e) => eval_expr(e, env, gen_fns, fn_bind, gens)?,
                    _ => return Err(Ev::U),
                }
            } else {
                return Err(Ev::U);
            };
            return gen_throw(&obj, val, gen_fns, fn_bind, gens);
        }
        if prop == "then" {
            // Immediately-settled thenable: invoke onfulfill with `obj` as value.
            if args.is_empty() || args.len() > 2 {
                return Err(Ev::U);
            }
            let Arg::Expr(cb) = &args[0] else {
                return Err(Ev::U);
            };
            return call_thenable_cb(cb, obj, env, gen_fns, fn_bind, gens);
        }
        if prop == "resolve" {
            if !matches!(obj, JsVal::BuiltinPromise) || args.len() != 1 {
                return Err(Ev::U);
            }
            let Arg::Expr(e) = &args[0] else {
                return Err(Ev::U);
            };
            return eval_expr(e, env, gen_fns, fn_bind, gens);
        }
        let method = lookup_prop(&obj, &prop)?;
        let this_val = match &obj {
            JsVal::Object { .. } | JsVal::Class { .. } => Some(obj),
            _ => None,
        };
        return match method {
            JsVal::AsyncFn { params, body } => {
                call_async_fn(&params, &body, args, env, gen_fns, fn_bind, gens)
            }
            other => spawn_gen_call(other, args, this_val, env, gen_fns, fn_bind, gens),
        };
    }

    // Call generator / async function: `g(args)` / IIFE → iterator or promise value.
    let c = eval_expr(callee, env, gen_fns, fn_bind, gens)?;
    call_value(c, args, env, gen_fns, fn_bind, gens)
}

pub(super) fn call_thenable_cb(
    cb: &Expr,
    value: JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let Expr::Function {
        params,
        body,
        is_async: false,
        is_generator: false,
        ..
    } = cb
    else {
        return Err(Ev::U);
    };
    let param_ids = simple_param_locals(params).ok_or(Ev::U)?;
    let mut saved: HashMap<LocalId, Option<JsVal>> = HashMap::new();
    for (i, pid) in param_ids.iter().enumerate() {
        saved.insert(*pid, env.get(pid).cloned());
        let v = if i == 0 { value.clone() } else { JsVal::Undef };
        env.insert(*pid, v);
    }
    let ret = match eval_fn_body_stmts(body, env, gen_fns, fn_bind, gens) {
        Ok(v) => v,
        Err(e) => {
            restore_params(env, &saved);
            return Err(e);
        }
    };
    restore_params(env, &saved);
    Ok(ret)
}

pub(super) fn restore_params(
    env: &mut HashMap<LocalId, JsVal>,
    saved: &HashMap<LocalId, Option<JsVal>>,
) {
    for (pid, prev) in saved {
        match prev {
            Some(v) => {
                env.insert(*pid, v.clone());
            }
            None => {
                env.remove(pid);
            }
        }
    }
}

pub(super) fn call_async_fn(
    params: &[LocalId],
    body: &[Stmt],
    args: &[Arg],
    env: &mut HashMap<LocalId, JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    if args.len() > params.len() {
        return Err(Ev::U);
    }
    let mut arg_vals = Vec::new();
    for a in args {
        match a {
            Arg::Expr(e) => arg_vals.push(eval_expr(e, env, gen_fns, fn_bind, gens)?),
            _ => return Err(Ev::U),
        }
    }
    // Nested bindings shadow outer; restore after.
    let mut saved: HashMap<LocalId, Option<JsVal>> = HashMap::new();
    for (i, pid) in params.iter().enumerate() {
        saved.entry(*pid).or_insert_with(|| env.get(pid).cloned());
        let v = if i < arg_vals.len() {
            arg_vals[i].clone()
        } else {
            JsVal::Undef
        };
        env.insert(*pid, v);
    }
    // Hoist nested generator / async-gen decls inside async function body.
    for stmt in body {
        if let Stmt::Function {
            local,
            params: gp,
            body: gbody,
            is_generator: true,
            ..
        } = stmt
        {
            let param_ids = simple_param_locals(gp).ok_or(Ev::U)?;
            let idx = gen_fns.len();
            gen_fns.push(GenFnRec {
                params: param_ids,
                body: filter_gen_body(gbody),
                name: None,
            });
            fn_bind.insert(*local, idx);
            saved
                .entry(*local)
                .or_insert_with(|| env.get(local).cloned());
            env.insert(*local, JsVal::GenFn(idx));
        }
    }
    let ret = match eval_fn_body_stmts(body, env, gen_fns, fn_bind, gens) {
        Ok(v) => v,
        Err(e) => {
            restore_params(env, &saved);
            return Err(e);
        }
    };
    restore_params(env, &saved);
    Ok(ret)
}
