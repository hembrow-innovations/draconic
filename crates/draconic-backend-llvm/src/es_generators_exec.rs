use super::class::*;
use super::eval::*;
use super::*;

/// Resume generator until next `yield` / `yield*` step / `return` / end.
/// First `.next` ignores `resume`; later calls inject it as the yield value.
pub(super) fn gen_next(
    obj: &JsVal,
    resume: JsVal,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::GenInst(idx) = obj else {
        return Err(Ev::U);
    };
    if *idx >= gens.len() {
        return Err(Ev::U);
    }
    if gens[*idx].done {
        return Ok(iter_result(JsVal::Undef, true));
    }

    let mut inject: Option<JsVal> = if !gens[*idx].started {
        gens[*idx].started = true;
        None
    } else if gens[*idx].suspended || gens[*idx].yield_star.is_some() {
        gens[*idx].suspended = false;
        Some(resume)
    } else {
        Some(JsVal::Undef)
    };

    gen_continue(*idx, &mut inject, gen_fns, fn_bind, gens)
}

/// Generator.prototype.return(value)
pub(super) fn gen_return(
    obj: &JsVal,
    value: JsVal,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::GenInst(idx) = obj else {
        return Err(Ev::U);
    };
    if *idx >= gens.len() {
        return Err(Ev::U);
    }
    if gens[*idx].done {
        return Ok(iter_result(value, true));
    }
    if !gens[*idx].started {
        gens[*idx].started = true;
        gens[*idx].done = true;
        return Ok(iter_result(value, true));
    }
    gens[*idx].suspended = false;
    gens[*idx].yield_star = None;

    // Unwind through try: run finally if present, else close with value.
    if let Some(ctx) = gens[*idx].try_ctx.as_mut() {
        if ctx.has_finally && ctx.region != 2 {
            ctx.pending_return = Some(value);
            ctx.region = 2;
            ctx.pc = 0;
            let mut inject = None;
            return gen_continue(*idx, &mut inject, gen_fns, fn_bind, gens);
        }
    }
    gens[*idx].try_ctx = None;
    gens[*idx].done = true;
    Ok(iter_result(value, true))
}

/// Generator.prototype.throw(exception)
pub(super) fn gen_throw(
    obj: &JsVal,
    value: JsVal,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    let JsVal::GenInst(idx) = obj else {
        return Err(Ev::U);
    };
    if *idx >= gens.len() {
        return Err(Ev::U);
    }
    if gens[*idx].done {
        return Err(Ev::Throw(value));
    }
    if !gens[*idx].started {
        gens[*idx].started = true;
        gens[*idx].done = true;
        return Err(Ev::Throw(value));
    }
    gens[*idx].suspended = false;
    gens[*idx].yield_star = None;

    if let Some(ctx) = gens[*idx].try_ctx.clone() {
        if ctx.region == 0 && ctx.has_handler {
            if let Some(pid) = ctx.handler_param {
                gens[*idx].env.insert(pid, value);
            }
            if let Some(c) = gens[*idx].try_ctx.as_mut() {
                c.region = 1;
                c.pc = 0;
            }
            let mut inject = None;
            return gen_continue(*idx, &mut inject, gen_fns, fn_bind, gens);
        }
        if ctx.has_finally && ctx.region != 2 {
            if let Some(c) = gens[*idx].try_ctx.as_mut() {
                c.pending_throw = Some(value);
                c.region = 2;
                c.pc = 0;
            }
            let mut inject = None;
            return gen_continue(*idx, &mut inject, gen_fns, fn_bind, gens);
        }
    }
    gens[*idx].try_ctx = None;
    gens[*idx].done = true;
    Err(Ev::Throw(value))
}

pub(super) fn gen_continue(
    idx: usize,
    inject: &mut Option<JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, Ev> {
    // Active yield* first.
    if gens[idx].yield_star.is_some() {
        match map_in_gen(step_yield_star(
            idx,
            inject.take().unwrap_or(JsVal::Undef),
            gen_fns,
            fn_bind,
            gens,
        ))? {
            Step::Yield(v) => return Ok(v),
            Step::Continue => {}
        }
    }

    let fn_id = gens[idx].fn_id;
    if fn_id >= gen_fns.len() {
        return Err(Ev::U);
    }
    let body = gen_fns[fn_id].body.clone();

    loop {
        // Nested try/catch/finally execution.
        if gens[idx].try_ctx.is_some() {
            match step_try_region(idx, inject, &body, gen_fns, fn_bind, gens)? {
                Some(v) => return Ok(v),
                None => continue,
            }
        }

        let pc = gens[idx].pc;
        if pc >= body.len() {
            gens[idx].done = true;
            return Ok(iter_result(JsVal::Undef, true));
        }

        match &body[pc] {
            Stmt::Try {
                block: _,
                handler_param,
                handler,
                finalizer,
            } => {
                let hp = match handler_param {
                    None => None,
                    Some(Pattern::Local(id)) => Some(*id),
                    Some(_) => return Err(Ev::U),
                };
                gens[idx].try_ctx = Some(TryCtx {
                    try_pc: pc,
                    region: 0,
                    pc: 0,
                    handler_param: hp,
                    has_handler: handler.is_some(),
                    has_finally: finalizer.is_some(),
                    pending_return: None,
                    pending_throw: None,
                });
                // do not advance outer pc yet
            }
            other => match exec_gen_stmt(idx, other, inject, true, gen_fns, fn_bind, gens)? {
                ExecOut::Yield(v) => return Ok(v),
                ExecOut::Done(v) => return Ok(v),
                ExecOut::Advanced => {
                    gens[idx].pc = pc + 1;
                }
                ExecOut::Stay => {}
            },
        }
    }
}

pub(super) enum ExecOut {
    Yield(JsVal),
    Done(JsVal),
    Advanced,
    Stay,
}

/// Execute one generator statement. `advance_outer` unused for nest (caller advances nest pc).
pub(super) fn exec_gen_stmt(
    idx: usize,
    stmt: &Stmt,
    inject: &mut Option<JsVal>,
    _advance_outer: bool,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<ExecOut, Ev> {
    match stmt {
        Stmt::Expr {
            expr:
                Expr::Unary {
                    op: UnaryOp::Yield,
                    arg,
                    ..
                },
        } => {
            if let Some(_v) = inject.take() {
                return Ok(ExecOut::Advanced);
            }
            let yv = map_in_gen(eval_in_gen(arg, idx, gen_fns, fn_bind, gens))?;
            gens[idx].suspended = true;
            Ok(ExecOut::Yield(iter_result(yv, false)))
        }
        Stmt::Expr {
            expr:
                Expr::Unary {
                    op: UnaryOp::YieldStar,
                    arg,
                    ..
                },
        } => {
            if inject.is_some() {
                inject.take();
            }
            map_in_gen(start_yield_star(idx, arg, None, gen_fns, fn_bind, gens))?;
            match map_in_gen(step_yield_star(idx, JsVal::Undef, gen_fns, fn_bind, gens))? {
                Step::Yield(v) => Ok(ExecOut::Yield(v)),
                Step::Continue => Ok(ExecOut::Stay),
            }
        }
        Stmt::Declare {
            local,
            init:
                Some(Expr::Unary {
                    op: UnaryOp::Yield,
                    arg,
                    ..
                }),
            ..
        } => {
            if let Some(v) = inject.take() {
                gens[idx].env.insert(*local, v);
                return Ok(ExecOut::Advanced);
            }
            let yv = map_in_gen(eval_in_gen(arg, idx, gen_fns, fn_bind, gens))?;
            gens[idx].suspended = true;
            Ok(ExecOut::Yield(iter_result(yv, false)))
        }
        Stmt::Declare {
            local,
            init:
                Some(Expr::Unary {
                    op: UnaryOp::YieldStar,
                    arg,
                    ..
                }),
            ..
        } => {
            if inject.is_some() {
                inject.take();
            }
            let bind = Some(*local);
            map_in_gen(start_yield_star(idx, arg, bind, gen_fns, fn_bind, gens))?;
            match map_in_gen(step_yield_star(idx, JsVal::Undef, gen_fns, fn_bind, gens))? {
                Step::Yield(v) => Ok(ExecOut::Yield(v)),
                Step::Continue => Ok(ExecOut::Stay),
            }
        }
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                None => JsVal::Undef,
                Some(e) => map_in_gen(eval_in_gen(e, idx, gen_fns, fn_bind, gens))?,
            };
            gens[idx].env.insert(*local, v);
            Ok(ExecOut::Advanced)
        }
        Stmt::Return { value } => {
            let v = match value {
                None => JsVal::Undef,
                Some(e) => map_in_gen(eval_in_gen(e, idx, gen_fns, fn_bind, gens))?,
            };
            // If inside try with finally, run finally before completing.
            if let Some(ctx) = gens[idx].try_ctx.as_mut() {
                if ctx.has_finally && ctx.region != 2 {
                    ctx.pending_return = Some(v);
                    ctx.region = 2;
                    ctx.pc = 0;
                    return Ok(ExecOut::Stay);
                }
            }
            gens[idx].try_ctx = None;
            gens[idx].done = true;
            Ok(ExecOut::Done(iter_result(v, true)))
        }
        Stmt::Expr { expr } => {
            map_in_gen(eval_in_gen(expr, idx, gen_fns, fn_bind, gens))?;
            Ok(ExecOut::Advanced)
        }
        Stmt::Block { body } => {
            // Flatten one level: not expected as nested block PC; unsupported multi-stmt block
            // without its own PC — only empty or single-pass via sequential not supported.
            let _ = body;
            Err(Ev::U)
        }
        _ => Err(Ev::U),
    }
}

/// Step inside active try_ctx. `Some(v)` = yield/done result; `None` = keep looping.
pub(super) fn step_try_region(
    idx: usize,
    inject: &mut Option<JsVal>,
    body: &[Stmt],
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<Option<JsVal>, Ev> {
    let ctx = gens[idx].try_ctx.clone().ok_or(Ev::U)?;
    let try_pc = ctx.try_pc;
    if try_pc >= body.len() {
        return Err(Ev::U);
    }
    let Stmt::Try {
        block,
        handler,
        finalizer,
        ..
    } = &body[try_pc]
    else {
        return Err(Ev::U);
    };

    let region_body: &[Stmt] = match ctx.region {
        0 => block.as_slice(),
        1 => handler.as_ref().map(|h| h.as_slice()).unwrap_or(&[]),
        2 => finalizer.as_ref().map(|f| f.as_slice()).unwrap_or(&[]),
        _ => return Err(Ev::U),
    };

    if ctx.pc >= region_body.len() {
        // Region finished.
        match ctx.region {
            0 => {
                if ctx.has_finally {
                    if let Some(c) = gens[idx].try_ctx.as_mut() {
                        c.region = 2;
                        c.pc = 0;
                    }
                } else {
                    gens[idx].try_ctx = None;
                    gens[idx].pc = try_pc + 1;
                }
                return Ok(None);
            }
            1 => {
                if ctx.has_finally {
                    if let Some(c) = gens[idx].try_ctx.as_mut() {
                        c.region = 2;
                        c.pc = 0;
                    }
                } else {
                    gens[idx].try_ctx = None;
                    gens[idx].pc = try_pc + 1;
                }
                return Ok(None);
            }
            2 => {
                let pending_return = ctx.pending_return.clone();
                let pending_throw = ctx.pending_throw.clone();
                gens[idx].try_ctx = None;
                if let Some(v) = pending_return {
                    gens[idx].done = true;
                    return Ok(Some(iter_result(v, true)));
                }
                if let Some(t) = pending_throw {
                    gens[idx].done = true;
                    return Err(Ev::Throw(t));
                }
                gens[idx].pc = try_pc + 1;
                return Ok(None);
            }
            _ => return Err(Ev::U),
        }
    }

    let stmt = &region_body[ctx.pc];
    match exec_gen_stmt(idx, stmt, inject, false, gen_fns, fn_bind, gens)? {
        ExecOut::Yield(v) => Ok(Some(v)),
        ExecOut::Done(v) => Ok(Some(v)),
        ExecOut::Advanced => {
            if let Some(c) = gens[idx].try_ctx.as_mut() {
                c.pc += 1;
            }
            Ok(None)
        }
        ExecOut::Stay => Ok(None),
    }
}

pub(super) enum Step {
    Yield(JsVal),
    Continue,
}

pub(super) fn start_yield_star(
    outer_idx: usize,
    arg: &Expr,
    bind: Option<LocalId>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<(), ()> {
    let iterable = eval_in_gen(arg, outer_idx, gen_fns, fn_bind, gens)?;
    let state = match iterable {
        JsVal::GenInst(i) => YieldStarState::Gen { idx: i, bind },
        JsVal::Array(elems) => YieldStarState::Array {
            elems,
            next_i: 0,
            bind,
        },
        JsVal::GenFn(fid) => {
            let inst = match spawn_gen_vals(JsVal::GenFn(fid), &[], None, gen_fns, fn_bind, gens) {
                Ok(v) => v,
                Err(_) => return Err(()),
            };
            let JsVal::GenInst(i) = inst else {
                return Err(());
            };
            YieldStarState::Gen { idx: i, bind }
        }
        _ => return Err(()),
    };
    gens[outer_idx].yield_star = Some(state);
    Ok(())
}

pub(super) fn step_yield_star(
    outer_idx: usize,
    resume: JsVal,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<Step, ()> {
    let state = gens[outer_idx].yield_star.take().ok_or(())?;
    match state {
        YieldStarState::Gen { idx: inner, bind } => {
            let r = match gen_next(&JsVal::GenInst(inner), resume, gen_fns, fn_bind, gens) {
                Ok(v) => v,
                Err(_) => return Err(()),
            };
            match r {
                JsVal::Result { value, done: false } => {
                    gens[outer_idx].yield_star = Some(YieldStarState::Gen { idx: inner, bind });
                    gens[outer_idx].suspended = true;
                    Ok(Step::Yield(JsVal::Result { value, done: false }))
                }
                JsVal::Result { value, done: true } => {
                    if let Some(local) = bind {
                        gens[outer_idx].env.insert(local, *value);
                    }
                    gens[outer_idx].pc += 1;
                    gens[outer_idx].suspended = false;
                    Ok(Step::Continue)
                }
                _ => Err(()),
            }
        }
        YieldStarState::Array {
            elems,
            next_i,
            bind,
        } => {
            let _ = resume; // array iterators ignore resume for this subset
            if next_i < elems.len() {
                let v = elems[next_i].clone();
                gens[outer_idx].yield_star = Some(YieldStarState::Array {
                    elems,
                    next_i: next_i + 1,
                    bind,
                });
                gens[outer_idx].suspended = true;
                Ok(Step::Yield(JsVal::Result {
                    value: Box::new(v),
                    done: false,
                }))
            } else {
                // Array iterator completion value is undefined.
                if let Some(local) = bind {
                    gens[outer_idx].env.insert(local, JsVal::Undef);
                }
                gens[outer_idx].pc += 1;
                gens[outer_idx].suspended = false;
                Ok(Step::Continue)
            }
        }
    }
}

/// Evaluate expression in generator body (params, arithmetic, arrays, gen calls, this).
pub(super) fn eval_in_gen(
    expr: &Expr,
    gen_idx: usize,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &mut HashMap<LocalId, usize>,
    gens: &mut Vec<GenInst>,
) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| ())?;
            Ok(JsVal::Num(n))
        }
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::String { value, .. } => Ok(JsVal::Str(value.to_string_lossy())),
        Expr::Local { id, .. } => gens[gen_idx].env.get(id).cloned().ok_or(()),
        Expr::This { .. } => gens[gen_idx].this_val.clone().ok_or(()),
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => Ok(JsVal::Undef),
        Expr::Unary {
            op: UnaryOp::Await,
            arg,
            ..
        } => eval_in_gen(arg, gen_idx, gen_fns, fn_bind, gens),
        Expr::Array { elements, .. } => {
            let mut vals = Vec::new();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => {
                        vals.push(eval_in_gen(e, gen_idx, gen_fns, fn_bind, gens)?)
                    }
                    _ => return Err(()),
                }
            }
            Ok(JsVal::Array(vals))
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let l = eval_in_gen(left, gen_idx, gen_fns, fn_bind, gens)?;
            let r = eval_in_gen(right, gen_idx, gen_fns, fn_bind, gens)?;
            bin_val(op, &l, &r).map_err(|_| ())
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = eval_in_gen(object, gen_idx, gen_fns, fn_bind, gens)?;
            let prop = prop_name(property).map_err(|_| ())?;
            lookup_prop(&obj, &prop).map_err(|_| ())
        }
        Expr::Function {
            name,
            params,
            body,
            is_generator: true,
            is_arrow: false,
            ..
        } => register_gen_fn_expr(*name, params, body, gen_fns, fn_bind).map_err(|_| ()),
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            // `Promise.resolve(x)` inside async gen (Promise may be missing from gen env).
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = callee.as_ref()
            {
                let prop = prop_name(property).map_err(|_| ())?;
                if prop == "resolve" {
                    let obj = match object.as_ref() {
                        Expr::Local { id, .. } => gens[gen_idx]
                            .env
                            .get(id)
                            .cloned()
                            .unwrap_or(JsVal::BuiltinPromise),
                        Expr::IdentName { name, .. } if name == "Promise" => JsVal::BuiltinPromise,
                        _ => eval_in_gen(object, gen_idx, gen_fns, fn_bind, gens)?,
                    };
                    if matches!(obj, JsVal::BuiltinPromise) && args.len() == 1 {
                        let Arg::Expr(e) = &args[0] else {
                            return Err(());
                        };
                        return eval_in_gen(e, gen_idx, gen_fns, fn_bind, gens);
                    }
                }
            }
            // Resolve callee in gen env; spawn if GenFn.
            let c = match callee.as_ref() {
                Expr::Local { id, .. } => gens[gen_idx].env.get(id).cloned().ok_or(())?,
                Expr::Function {
                    name,
                    params,
                    body,
                    is_generator: true,
                    is_arrow: false,
                    ..
                } => register_gen_fn_expr(*name, params, body, gen_fns, fn_bind).map_err(|_| ())?,
                _ => return Err(()),
            };
            // Build args using gen env.
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => arg_vals.push(eval_in_gen(e, gen_idx, gen_fns, fn_bind, gens)?),
                    _ => return Err(()),
                }
            }
            spawn_gen_vals(c, &arg_vals, None, gen_fns, fn_bind, gens).map_err(|_| ())
        }
        _ => Err(()),
    }
}

pub(super) fn spawn_gen_vals(
    callee: JsVal,
    args: &[JsVal],
    this_val: Option<JsVal>,
    gen_fns: &mut Vec<GenFnRec>,
    fn_bind: &HashMap<LocalId, usize>,
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
    let params = gen_fns[fid].params.clone();
    let name = gen_fns[fid].name;
    let mut gen_env = HashMap::new();
    for (i, pid) in params.iter().enumerate() {
        let v = if i < args.len() {
            args[i].clone()
        } else {
            JsVal::Undef
        };
        gen_env.insert(*pid, v);
    }
    // Free GenFn bindings: inject all known gen fn bindings so nested yield* can call peers.
    for (lid, idx) in fn_bind {
        gen_env.entry(*lid).or_insert(JsVal::GenFn(*idx));
    }
    // Named function expression self-binding.
    if let Some(n) = name {
        gen_env.insert(n, JsVal::GenFn(fid));
    }
    let idx = gens.len();
    gens.push(GenInst {
        fn_id: fid,
        pc: 0,
        started: false,
        suspended: false,
        done: false,
        env: gen_env,
        this_val,
        yield_star: None,
        try_ctx: None,
    });
    Ok(JsVal::GenInst(idx))
}
