use super::*;

pub(super) fn eval_body(body: &[Stmt], ctx: &mut EvalCtx) -> Result<Flow, ()> {
    for stmt in body {
        match eval_stmt(stmt, ctx)? {
            Flow::Normal => {}
            other => return Ok(other),
        }
    }
    Ok(Flow::Normal)
}

fn eval_stmt(stmt: &Stmt, ctx: &mut EvalCtx) -> Result<Flow, ()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => match eval_expr(e, ctx)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                },
                None => JsVal::Undef,
            };
            ctx.env.insert(*local, v);
            Ok(Flow::Normal)
        }
        Stmt::Expr { expr } => match eval_expr(expr, ctx)? {
            Ok(_) => Ok(Flow::Normal),
            Err(flow) => Ok(flow),
        },
        Stmt::Throw { value } => match eval_expr(value, ctx)? {
            Ok(v) => Ok(Flow::Throw(v)),
            Err(flow) => Ok(flow),
        },
        Stmt::Return { value } => {
            let v = match value {
                Some(e) => match eval_expr(e, ctx)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                },
                None => JsVal::Undef,
            };
            Ok(Flow::Return(v))
        }
        Stmt::Block { body } => eval_body(body, ctx),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = match eval_expr(test, ctx)? {
                Ok(v) => v,
                Err(flow) => return Ok(flow),
            };
            if to_boolean(&t) {
                eval_stmt(consequent, ctx)
            } else if let Some(a) = alternate {
                eval_stmt(a, ctx)
            } else {
                Ok(Flow::Normal)
            }
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            let after_try = match eval_body(block, ctx)? {
                Flow::Throw(exc) => {
                    if let Some(h) = handler {
                        if let Some(param) = handler_param {
                            match param {
                                Pattern::Local(id) => {
                                    ctx.env.insert(*id, exc);
                                }
                                _ => return Err(()),
                            }
                        }
                        eval_body(h, ctx)?
                    } else {
                        Flow::Throw(exc)
                    }
                }
                other => other,
            };
            if let Some(f) = finalizer {
                match eval_body(f, ctx)? {
                    Flow::Normal => {}
                    other => return Ok(other),
                }
            }
            Ok(after_try)
        }
        _ => Err(()),
    }
}

fn eval_expr(expr: &Expr, ctx: &mut EvalCtx) -> Result<Result<JsVal, Flow>, ()> {
    match expr {
        Expr::Number { raw, .. } => Ok(Ok(JsVal::Num(raw.parse().map_err(|_| ())?))),
        Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
        Expr::String { value, .. } => Ok(Ok(JsVal::Str(js_string_to_utf8(value)))),
        Expr::Null { .. } => Ok(Ok(JsVal::Undef)),
        Expr::Local { id, .. } => Ok(Ok(ctx.env.get(id).cloned().ok_or(())?)),
        Expr::IdentName { name, .. } => Ok(Ok(JsVal::Builtin(ident_builtin(name).ok_or(())?))),
        Expr::Unary { op, arg, .. } => {
            let v = match eval_expr(arg, ctx)? {
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
            let l = match eval_expr(left, ctx)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let r = match eval_expr(right, ctx)? {
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
            let t = match eval_expr(test, ctx)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            if to_boolean(&t) {
                eval_expr(consequent, ctx)
            } else {
                eval_expr(alternate, ctx)
            }
        }
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => {
            let obj = match eval_expr(object, ctx)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            let key = match eval_key(property, ctx)? {
                Ok(k) => k,
                Err(flow) => return Ok(Err(flow)),
            };
            Ok(Ok(member_get(&obj, &key)?))
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
                    Arg::Expr(e) => match eval_expr(e, ctx)? {
                        Ok(v) => arg_vals.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    _ => return Err(()),
                }
            }
            let c = match eval_expr(callee, ctx)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match eval_call(&c, &arg_vals, ctx) {
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
            let v = match eval_expr(value, ctx)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            ctx.env.insert(*id, v.clone());
            Ok(Ok(v))
        }
        Expr::Function { body, .. } => Ok(Ok(JsVal::Closure { body: body.clone() })),
        _ => Err(()),
    }
}

fn eval_key(property: &Expr, ctx: &mut EvalCtx) -> Result<Result<String, Flow>, ()> {
    match property {
        Expr::String { value, .. } => Ok(Ok(js_string_to_utf8(value))),
        _ => match eval_expr(property, ctx)? {
            Ok(JsVal::Str(s)) => Ok(Ok(s)),
            Ok(_) => Err(()),
            Err(flow) => Ok(Err(flow)),
        },
    }
}

fn eval_call(callee: &JsVal, args: &[JsVal], ctx: &mut EvalCtx) -> Result<JsVal, Option<Flow>> {
    match callee {
        JsVal::Builtin(BuiltinId::Describe) => {
            let fn_val = args.get(1).ok_or(None)?;
            ctx.suites.push(SuiteHooks::default());
            let body_res = call_closure(fn_val, ctx);
            let afters = ctx
                .suites
                .last()
                .map(|s| s.after.clone())
                .unwrap_or_default();
            let after_res = run_hook_list(&afters, ctx);
            ctx.suites.pop();
            match after_res {
                Err(e) => Err(e),
                Ok(()) => match body_res {
                    Ok(_) => Ok(JsVal::Undef),
                    Err(e) => Err(e),
                },
            }
        }
        JsVal::Builtin(BuiltinId::It) => {
            let fn_val = args.get(1).ok_or(None)?;
            let mut ok = true;
            match run_all_before_each(ctx) {
                Ok(()) => match call_closure(fn_val, ctx) {
                    Ok(_) => {}
                    Err(Some(Flow::Throw(_))) => ok = false,
                    Err(other) => return Err(other),
                },
                Err(Some(Flow::Throw(_))) => ok = false,
                Err(other) => return Err(other),
            }
            match run_all_after_each(ctx) {
                Ok(()) => {}
                Err(Some(Flow::Throw(_))) => ok = false,
                Err(other) => return Err(other),
            }
            Ok(JsVal::Bool(ok))
        }
        JsVal::Builtin(BuiltinId::Before) => {
            let fn_val = args.first().ok_or(None)?;
            call_closure(fn_val, ctx).map(|_| JsVal::Undef)
        }
        JsVal::Builtin(BuiltinId::After) => {
            let fn_val = args.first().cloned().ok_or(None)?;
            ctx.suites.last_mut().ok_or(None)?.after.push(fn_val);
            Ok(JsVal::Undef)
        }
        JsVal::Builtin(BuiltinId::BeforeEach) => {
            let fn_val = args.first().cloned().ok_or(None)?;
            ctx.suites.last_mut().ok_or(None)?.before_each.push(fn_val);
            Ok(JsVal::Undef)
        }
        JsVal::Builtin(BuiltinId::AfterEach) => {
            let fn_val = args.first().cloned().ok_or(None)?;
            ctx.suites.last_mut().ok_or(None)?.after_each.push(fn_val);
            Ok(JsVal::Undef)
        }
        JsVal::Builtin(BuiltinId::Expect) => {
            let actual = args.first().cloned().unwrap_or(JsVal::Undef);
            Ok(JsVal::Matcher {
                actual: Box::new(actual),
            })
        }
        JsVal::BoundMatcher { kind, actual } => match kind {
            MatcherKind::ToBe => {
                let expected = args.first().cloned().unwrap_or(JsVal::Undef);
                if strict_eq(actual, &expected) {
                    Ok(JsVal::Undef)
                } else {
                    Err(Some(Flow::Throw(JsVal::Str(format!(
                        "expected {} to be {}",
                        display(actual),
                        display(&expected)
                    )))))
                }
            }
            MatcherKind::ToBeTruthy => {
                if to_boolean(actual) {
                    Ok(JsVal::Undef)
                } else {
                    Err(Some(Flow::Throw(JsVal::Str(format!(
                        "expected {} to be truthy",
                        display(actual)
                    )))))
                }
            }
            MatcherKind::ToBeFalsy => {
                if !to_boolean(actual) {
                    Ok(JsVal::Undef)
                } else {
                    Err(Some(Flow::Throw(JsVal::Str(format!(
                        "expected {} to be falsy",
                        display(actual)
                    )))))
                }
            }
        },
        JsVal::Closure { .. } => call_closure(callee, ctx).map(|_| JsVal::Undef),
        _ => Err(None),
    }
}

fn run_hook_list(hooks: &[JsVal], ctx: &mut EvalCtx) -> Result<(), Option<Flow>> {
    for h in hooks {
        call_closure(h, ctx)?;
    }
    Ok(())
}

fn run_all_before_each(ctx: &mut EvalCtx) -> Result<(), Option<Flow>> {
    let hooks: Vec<Vec<JsVal>> = ctx.suites.iter().map(|s| s.before_each.clone()).collect();
    for list in hooks {
        run_hook_list(&list, ctx)?;
    }
    Ok(())
}

fn run_all_after_each(ctx: &mut EvalCtx) -> Result<(), Option<Flow>> {
    let hooks: Vec<Vec<JsVal>> = ctx
        .suites
        .iter()
        .rev()
        .map(|s| s.after_each.clone())
        .collect();
    for list in hooks {
        run_hook_list(&list, ctx)?;
    }
    Ok(())
}

fn call_closure(fn_val: &JsVal, ctx: &mut EvalCtx) -> Result<JsVal, Option<Flow>> {
    let JsVal::Closure { body } = fn_val else {
        return Err(None);
    };
    let body = body.clone();
    match eval_body(&body, ctx) {
        Ok(Flow::Normal) => Ok(JsVal::Undef),
        Ok(Flow::Return(v)) => Ok(v),
        Ok(Flow::Throw(exc)) => Err(Some(Flow::Throw(exc))),
        Err(()) => Err(None),
    }
}

fn member_get(obj: &JsVal, key: &str) -> Result<JsVal, ()> {
    match obj {
        JsVal::Builtin(BuiltinId::GlobalThis) => match key {
            "describe" => Ok(JsVal::Builtin(BuiltinId::Describe)),
            "it" => Ok(JsVal::Builtin(BuiltinId::It)),
            "expect" => Ok(JsVal::Builtin(BuiltinId::Expect)),
            "before" => Ok(JsVal::Builtin(BuiltinId::Before)),
            "after" => Ok(JsVal::Builtin(BuiltinId::After)),
            "beforeEach" => Ok(JsVal::Builtin(BuiltinId::BeforeEach)),
            "afterEach" => Ok(JsVal::Builtin(BuiltinId::AfterEach)),
            "globalThis" => Ok(JsVal::Builtin(BuiltinId::GlobalThis)),
            _ => Err(()),
        },
        JsVal::Matcher { actual } => {
            let kind = match key {
                "toBe" => MatcherKind::ToBe,
                "toBeTruthy" => MatcherKind::ToBeTruthy,
                "toBeFalsy" => MatcherKind::ToBeFalsy,
                _ => return Err(()),
            };
            Ok(JsVal::BoundMatcher {
                kind,
                actual: actual.clone(),
            })
        }
        _ => Err(()),
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::Builtin(
            BuiltinId::Describe
            | BuiltinId::It
            | BuiltinId::Expect
            | BuiltinId::Before
            | BuiltinId::After
            | BuiltinId::BeforeEach
            | BuiltinId::AfterEach,
        )
        | JsVal::Closure { .. }
        | JsVal::BoundMatcher { .. } => "function".into(),
        JsVal::Builtin(BuiltinId::GlobalThis) | JsVal::Matcher { .. } => "object".into(),
    }
}

fn display(v: &JsVal) -> String {
    match v {
        JsVal::Num(n) => {
            if n.is_finite() && *n == n.trunc() {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        JsVal::Bool(b) => {
            if *b {
                "true".into()
            } else {
                "false".into()
            }
        }
        JsVal::Str(s) => {
            let mut out = String::from("\"");
            for c in s.chars() {
                match c {
                    '\\' => out.push_str("\\\\"),
                    '"' => out.push_str("\\\""),
                    _ => out.push(c),
                }
            }
            out.push('"');
            out
        }
        JsVal::Undef => "undefined".into(),
        _ => "object".into(),
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
