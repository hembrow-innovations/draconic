use super::*;

pub(super) fn eval_body(body: &[Stmt], ctx: &mut EvalCtx<'_>) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, ctx)? {
            Flow::Normal => {}
            other => return Ok(other),
        }
    }
    Ok(Flow::Normal)
}

fn eval_stmt(stmt: &Stmt, ctx: &mut EvalCtx<'_>) -> Result<Flow, ()> {
    match stmt {
        Stmt::Function { .. } => Ok(Flow::Normal),
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => eval_expr(e, ctx)?,
                None => JsVal::Undef,
            };
            ctx.env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Return { value } => match value {
            None => Ok(Flow::Return(JsVal::Undef)),
            Some(e) => Ok(Flow::Return(eval_expr(e, ctx)?)),
        },
        Stmt::Block { body } => eval_body(body, ctx),
        Stmt::Expr { expr } => {
            eval_expr(expr, ctx)?;
            Ok(Flow::Normal)
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = to_boolean(&eval_expr(test, ctx)?);
            if t {
                eval_stmt(consequent, ctx)
            } else if let Some(a) = alternate {
                eval_stmt(a, ctx)
            } else {
                Ok(Flow::Normal)
            }
        }
        _ => Err(()),
    }
}

fn eval_expr(expr: &Expr, ctx: &mut EvalCtx<'_>) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
            let n: f64 = cleaned.parse().map_err(|_| ())?;
            Ok(JsVal::Num(n))
        }
        Expr::String { value, .. } => Ok(JsVal::Str(value.to_string_lossy())),
        Expr::Boolean { value, .. } => Ok(JsVal::Num(if *value { 1.0 } else { 0.0 })),
        Expr::Null { .. } => Ok(JsVal::Num(0.0)),
        Expr::Local { id, .. } => ctx.env.get(id).cloned().ok_or(()),
        Expr::This { .. } => {
            let id = ctx.this_obj.ok_or(())?;
            Ok(JsVal::Obj(id))
        }
        Expr::NewTarget { .. } | Expr::IdentName { .. } => Ok(JsVal::Undef),
        Expr::Unary { op, arg, .. } => {
            let v = eval_expr(arg, ctx)?;
            match op {
                UnaryOp::Plus => Ok(JsVal::Num(to_number(&v))),
                UnaryOp::Minus => Ok(JsVal::Num(-to_number(&v))),
                UnaryOp::Not => Ok(JsVal::Num(if to_boolean(&v) { 0.0 } else { 1.0 })),
                UnaryOp::Void => Ok(JsVal::Undef),
                UnaryOp::TypeOf => Ok(JsVal::Str(typeof_str(&v).into())),
                UnaryOp::Delete => Ok(JsVal::Num(1.0)),
                _ => Err(()),
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            if matches!(op, BinaryOp::And) {
                let l = eval_expr(left, ctx)?;
                if !to_boolean(&l) {
                    return Ok(l);
                }
                return eval_expr(right, ctx);
            }
            if matches!(op, BinaryOp::Or) {
                let l = eval_expr(left, ctx)?;
                if to_boolean(&l) {
                    return Ok(l);
                }
                return eval_expr(right, ctx);
            }
            if matches!(op, BinaryOp::Comma) {
                eval_expr(left, ctx)?;
                return eval_expr(right, ctx);
            }
            let l = eval_expr(left, ctx)?;
            let r = eval_expr(right, ctx)?;
            match op {
                BinaryOp::Add => {
                    if matches!((&l, &r), (JsVal::Str(_), _) | (_, JsVal::Str(_))) {
                        Ok(JsVal::Str(format!(
                            "{}{}",
                            to_string_val(&l),
                            to_string_val(&r)
                        )))
                    } else {
                        Ok(JsVal::Num(to_number(&l) + to_number(&r)))
                    }
                }
                BinaryOp::Sub => Ok(JsVal::Num(to_number(&l) - to_number(&r))),
                BinaryOp::Mul => Ok(JsVal::Num(to_number(&l) * to_number(&r))),
                BinaryOp::Div => Ok(JsVal::Num(to_number(&l) / to_number(&r))),
                BinaryOp::Rem => Ok(JsVal::Num(to_number(&l) % to_number(&r))),
                BinaryOp::EqEqEq => Ok(JsVal::Num(if strict_eq(&l, &r) { 1.0 } else { 0.0 })),
                BinaryOp::NotEqEq => Ok(JsVal::Num(if strict_eq(&l, &r) { 0.0 } else { 1.0 })),
                BinaryOp::Lt => Ok(JsVal::Num(if to_number(&l) < to_number(&r) {
                    1.0
                } else {
                    0.0
                })),
                BinaryOp::LtEq => Ok(JsVal::Num(if to_number(&l) <= to_number(&r) {
                    1.0
                } else {
                    0.0
                })),
                BinaryOp::Gt => Ok(JsVal::Num(if to_number(&l) > to_number(&r) {
                    1.0
                } else {
                    0.0
                })),
                BinaryOp::GtEq => Ok(JsVal::Num(if to_number(&l) >= to_number(&r) {
                    1.0
                } else {
                    0.0
                })),
                BinaryOp::In => Ok(JsVal::Num(0.0)),
                _ => Err(()),
            }
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = eval_expr(value, ctx)?;
            ctx.env.insert(*id, v.clone());
            Ok(v)
        }
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    computed: false,
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let obj = eval_expr(object, ctx)?;
            let key = prop_key(property, ctx)?;
            let v = eval_expr(value, ctx)?;
            let JsVal::Obj(oid) = obj else {
                return Err(());
            };
            let inst = ctx.heap.get_mut(&oid).ok_or(())?;
            inst.props.insert(key, v.clone());
            Ok(v)
        }
        Expr::Array { elements, .. } => {
            let mut items = Vec::with_capacity(elements.len());
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => items.push(eval_expr(e, ctx)?),
                    _ => return Err(()),
                }
            }
            Ok(JsVal::Arr(items))
        }
        Expr::Object { .. } => Ok(JsVal::Undef),
        Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => {
            let param_ids = simple_param_ids(params).ok_or(())?;
            Ok(JsVal::FnExpr(FnRec {
                params: param_ids,
                body: body.clone(),
            }))
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = eval_expr(object, ctx)?;
            let key = prop_key(property, ctx)?;
            match obj {
                JsVal::Ns(map) => match map.get(&key).cloned() {
                    Some(getter) => call_value(getter, &[], None, ctx),
                    None => Ok(JsVal::Undef),
                },
                JsVal::Obj(oid) => {
                    let inst = ctx.heap.get(&oid).ok_or(())?.clone();
                    if let Some(v) = inst.props.get(&key) {
                        return Ok(v.clone());
                    }
                    if let Some(m) = inst.methods.get(&key) {
                        return Ok(JsVal::FnExpr(m.clone()));
                    }
                    Err(())
                }
                JsVal::Class(c) => {
                    if let Some(m) = c.methods.get(&key) {
                        return Ok(JsVal::FnExpr(m.clone()));
                    }
                    Err(())
                }
                _ => Err(()),
            }
        }
        Expr::New { callee, args, .. } => {
            let c = eval_expr(callee, ctx)?;
            let mut arg_vals = Vec::with_capacity(args.len());
            for a in args {
                match a {
                    Arg::Expr(e) => arg_vals.push(eval_expr(e, ctx)?),
                    _ => return Err(()),
                }
            }
            let JsVal::Class(class) = c else {
                return Err(());
            };
            let oid = ctx.next_obj;
            ctx.next_obj += 1;
            ctx.heap.insert(
                oid,
                InstRec {
                    props: HashMap::new(),
                    methods: class.methods.clone(),
                },
            );
            let prev_this = ctx.this_obj.replace(oid);
            let _ = call_fnrec(&class.ctor, &arg_vals, Some(oid), ctx)?;
            ctx.this_obj = prev_this;
            Ok(JsVal::Obj(oid))
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            // Method call: recv.m(args) — bind `this` when callee is Member on instance.
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = callee.as_ref()
            {
                let recv = eval_expr(object, ctx)?;
                let key = prop_key(property, ctx)?;
                let mut arg_vals = Vec::with_capacity(args.len());
                for a in args {
                    match a {
                        Arg::Expr(e) => arg_vals.push(eval_expr(e, ctx)?),
                        _ => return Err(()),
                    }
                }
                if let JsVal::Obj(oid) = recv {
                    let inst = ctx.heap.get(&oid).ok_or(())?.clone();
                    let method = inst.methods.get(&key).cloned().ok_or(())?;
                    return call_fnrec(&method, &arg_vals, Some(oid), ctx);
                }
                if let JsVal::Ns(map) = recv {
                    let getter = map.get(&key).cloned().ok_or(())?;
                    // Namespace export accessed as call target is unusual; fall through.
                    let c = call_value(getter, &[], None, ctx)?;
                    return call_value(c, &arg_vals, None, ctx);
                }
                return Err(());
            }

            let c = eval_expr(callee, ctx)?;
            let mut arg_vals = Vec::with_capacity(args.len());
            for a in args {
                match a {
                    Arg::Expr(e) => arg_vals.push(eval_expr(e, ctx)?),
                    _ => return Err(()),
                }
            }
            // Class builder IIFE: ` (function(){ … return ctor })() `
            if let JsVal::FnExpr(f) = &c {
                if f.params.is_empty() && looks_like_class_iife(&f.body) {
                    return extract_class_iife(&f.body);
                }
            }
            if let JsVal::Fn(fid) = &c {
                if ctx.make_ns == Some(*fid) {
                    return eval_make_ns(&arg_vals);
                }
            }
            call_value(c, &arg_vals, None, ctx)
        }
        _ => Err(()),
    }
}

fn prop_key(property: &Expr, ctx: &mut EvalCtx<'_>) -> Result<String, ()> {
    match property {
        Expr::String { value, .. } => Ok(value.to_string_lossy()),
        other => match eval_expr(other, ctx)? {
            JsVal::Str(s) => Ok(s),
            JsVal::Num(n) if n.is_finite() && n.fract() == 0.0 => Ok(format!("{}", n as i64)),
            _ => Err(()),
        },
    }
}

fn extract_class_iife(body: &[Stmt]) -> Result<JsVal, ()> {
    let mut ctor: Option<FnRec> = None;
    let mut methods: HashMap<String, FnRec> = HashMap::new();
    let mut pending_key: Option<String> = None;
    let mut ctor_local: Option<LocalId> = None;

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
            } if ctor.is_none() => {
                let param_ids = simple_param_ids(params).ok_or(())?;
                ctor = Some(FnRec {
                    params: param_ids,
                    body: filter_ctor_body(cbody),
                });
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
                let Some(cl) = ctor_local else {
                    continue;
                };
                // Skip defineProperty on the constructor itself (name/prototype).
                if matches!(&args[0], Arg::Expr(Expr::Local { id, .. }) if *id == cl) {
                    pending_key = None;
                    continue;
                }
                if !is_define_on_proto(args, cl) {
                    return Err(());
                }
                let key = pending_key
                    .take()
                    .or_else(|| string_arg(&args[1]))
                    .ok_or(())?;
                let Arg::Expr(desc) = &args[2] else {
                    return Err(());
                };
                let method_fn = find_method_function(desc).ok_or(())?;
                let Expr::Function {
                    params,
                    body: mbody,
                    is_async: false,
                    is_generator: false,
                    ..
                } = method_fn
                else {
                    return Err(());
                };
                let param_ids = simple_param_ids(params).ok_or(())?;
                methods.insert(
                    key,
                    FnRec {
                        params: param_ids,
                        body: filter_method_body(mbody),
                    },
                );
            }
            Stmt::Return {
                value: Some(Expr::Local { id, .. }),
            } if Some(*id) == ctor_local => {}
            _ => return Err(()),
        }
    }

    Ok(JsVal::Class(ClassRec {
        ctor: ctor.ok_or(())?,
        methods,
    }))
}

fn is_object_define_property(callee: &Expr) -> bool {
    matches!(
        callee,
        Expr::Member {
            object,
            property,
            ..
        } if matches!(
            (object.as_ref(), property.as_ref()),
            (
                Expr::IdentName { name, .. },
                Expr::String { value, .. }
            ) if name == "Object" && value.to_string_lossy() == "defineProperty"
        )
    )
}

fn is_define_on_proto(args: &[Arg], ctor: LocalId) -> bool {
    matches!(
        &args[0],
        Arg::Expr(Expr::Member {
            object,
            property,
            ..
        }) if matches!(
            (object.as_ref(), property.as_ref()),
            (
                Expr::Local { id, .. },
                Expr::String { value, .. }
            ) if *id == ctor && value.to_string_lossy() == "prototype"
        )
    )
}

fn string_arg(arg: &Arg) -> Option<String> {
    match arg {
        Arg::Expr(Expr::String { value, .. }) => Some(value.to_string_lossy()),
        _ => None,
    }
}

fn find_method_function(expr: &Expr) -> Option<&Expr> {
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
            if let Some(f) = find_method_function(callee) {
                return Some(f);
            }
            for a in args {
                if let Arg::Expr(e) = a {
                    if let Some(f) = find_method_function(e) {
                        return Some(f);
                    }
                }
            }
            None
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                if let ObjectProp::Property { value, .. } = p {
                    if let Some(f) = find_method_function(value) {
                        return Some(f);
                    }
                }
            }
            None
        }
        Expr::Member {
            object, property, ..
        } => find_method_function(object).or_else(|| find_method_function(property)),
        Expr::Binary { left, right, .. } => {
            find_method_function(left).or_else(|| find_method_function(right))
        }
        Expr::Assign { value, .. } => find_method_function(value),
        Expr::Unary { arg, .. } => find_method_function(arg),
        _ => None,
    }
}

fn find_method_function_in_stmt(stmt: &Stmt) -> Option<&Expr> {
    match stmt {
        Stmt::Expr { expr } | Stmt::Return { value: Some(expr) } => find_method_function(expr),
        Stmt::Declare {
            init: Some(expr), ..
        } => find_method_function(expr),
        Stmt::Block { body } => body.iter().find_map(find_method_function_in_stmt),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => find_method_function(test)
            .or_else(|| find_method_function_in_stmt(consequent))
            .or_else(|| {
                alternate
                    .as_ref()
                    .and_then(|a| find_method_function_in_stmt(a))
            }),
        _ => None,
    }
}

fn filter_ctor_body(body: &[Stmt]) -> Vec<Stmt> {
    body.iter()
        .filter(|s| {
            matches!(
                s,
                Stmt::Expr {
                    expr: Expr::Assign {
                        target: AssignTarget::Member { .. },
                        op: AssignOp::Eq,
                        ..
                    },
                } | Stmt::Return { .. }
                    | Stmt::Block { .. }
            )
        })
        .cloned()
        .collect()
}

fn filter_method_body(body: &[Stmt]) -> Vec<Stmt> {
    body.iter()
        .filter(|s| {
            !matches!(
                s,
                Stmt::Expr {
                    expr: Expr::String { value, .. },
                } if value.to_string_lossy() == "use strict"
            )
        })
        .cloned()
        .collect()
}

/// Build a namespace object from `__draconic_make_ns(pairs, names, tag)` args.
/// Each pair is `[exportName, getterFn]`; getters run on property access (live bindings).
fn eval_make_ns(args: &[JsVal]) -> Result<JsVal, ()> {
    if args.is_empty() {
        return Err(());
    }
    let JsVal::Arr(pairs) = &args[0] else {
        return Err(());
    };
    let mut map = HashMap::new();
    for p in pairs {
        let JsVal::Arr(kv) = p else {
            return Err(());
        };
        if kv.len() != 2 {
            return Err(());
        }
        let name = match &kv[0] {
            JsVal::Str(s) => s.clone(),
            _ => return Err(()),
        };
        match &kv[1] {
            JsVal::Fn(_) | JsVal::FnExpr(_) => {}
            _ => return Err(()),
        }
        map.insert(name, kv[1].clone());
    }
    Ok(JsVal::Ns(map))
}

fn call_value(
    callee: JsVal,
    arg_vals: &[JsVal],
    this_obj: Option<u32>,
    ctx: &mut EvalCtx<'_>,
) -> Result<JsVal, ()> {
    match callee {
        JsVal::Fn(fid) => {
            let frec = ctx.functions.get(&fid).ok_or(())?.clone();
            call_fnrec(&frec, arg_vals, this_obj, ctx)
        }
        JsVal::FnExpr(f) => call_fnrec(&f, arg_vals, this_obj, ctx),
        JsVal::Class(_) => Err(()),
        _ => Err(()),
    }
}

fn call_fnrec(
    frec: &FnRec,
    arg_vals: &[JsVal],
    this_obj: Option<u32>,
    ctx: &mut EvalCtx<'_>,
) -> Result<JsVal, ()> {
    if arg_vals.len() > frec.params.len() {
        return Err(());
    }
    for (i, pid) in frec.params.iter().enumerate() {
        let v = arg_vals.get(i).cloned().unwrap_or(JsVal::Undef);
        ctx.env.insert(*pid, v);
    }
    let prev_this = ctx.this_obj;
    if this_obj.is_some() {
        ctx.this_obj = this_obj;
    }
    let flow = eval_body(&frec.body, ctx);
    ctx.this_obj = prev_this;
    match flow? {
        Flow::Normal => Ok(JsVal::Undef),
        Flow::Return(v) => Ok(v),
    }
}

fn to_number(v: &JsVal) -> f64 {
    match v {
        JsVal::Num(n) => *n,
        JsVal::Str(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse().unwrap_or(f64::NAN)
            }
        }
        JsVal::Undef => f64::NAN,
        JsVal::Fn(_)
        | JsVal::FnExpr(_)
        | JsVal::Class(_)
        | JsVal::Obj(_)
        | JsVal::Ns(_)
        | JsVal::Arr(_) => f64::NAN,
    }
}

fn to_string_val(v: &JsVal) -> String {
    match v {
        JsVal::Num(n) => {
            if n.is_nan() {
                "NaN".into()
            } else if n.is_infinite() {
                if n.is_sign_negative() {
                    "-Infinity".into()
                } else {
                    "Infinity".into()
                }
            } else if *n == 0.0 {
                "0".into()
            } else {
                format!("{n}")
            }
        }
        JsVal::Str(s) => s.clone(),
        JsVal::Undef => "undefined".into(),
        _ => "[object Object]".into(),
    }
}

fn typeof_str(v: &JsVal) -> &'static str {
    match v {
        JsVal::Num(_) => "number",
        JsVal::Str(_) => "string",
        JsVal::Undef => "undefined",
        JsVal::Fn(_) | JsVal::FnExpr(_) | JsVal::Class(_) => "function",
        JsVal::Obj(_) | JsVal::Ns(_) | JsVal::Arr(_) => "object",
    }
}

fn strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => x == y,
        (JsVal::Str(x), JsVal::Str(y)) => x == y,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Obj(x), JsVal::Obj(y)) => x == y,
        _ => false,
    }
}

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef => false,
        JsVal::Fn(_) | JsVal::FnExpr(_) | JsVal::Class(_) | JsVal::Obj(_) | JsVal::Ns(_) => true,
        JsVal::Arr(a) => !a.is_empty(),
    }
}
