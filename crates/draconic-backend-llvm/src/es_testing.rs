//! L05 / L05.01 / L05.02 / L05.03: native observations for `describe` / `it` / `expect` + hooks.
//!
//! Compile-time evaluation: `describe` runs its callback; `it` runs its callback
//! and yields `true` on success or `false` if the callback throws. `expect`
//! matchers throw a string message on failure. Nested `describe` plus `before` /
//! `after` / `beforeEach` / `afterEach` share a suite stack. Emits Runtime prints
//! of final top-level number/string/bool locals.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Pattern, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

pub(crate) fn is_es_testing_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_testing(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_testing module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum BuiltinId {
    GlobalThis,
    Describe,
    It,
    Expect,
    Before,
    After,
    BeforeEach,
    AfterEach,
}

#[derive(Clone, Debug, Default)]
struct SuiteHooks {
    before_each: Vec<JsVal>,
    after_each: Vec<JsVal>,
    after: Vec<JsVal>,
}

struct EvalCtx {
    env: HashMap<LocalId, JsVal>,
    suites: Vec<SuiteHooks>,
}

impl EvalCtx {
    fn new() -> Self {
        Self {
            env: HashMap::new(),
            suites: vec![SuiteHooks::default()],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum MatcherKind {
    ToBe,
    ToBeTruthy,
    ToBeFalsy,
}

#[derive(Clone, Debug, PartialEq)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Builtin(BuiltinId),
    Closure {
        body: Vec<Stmt>,
    },
    Matcher {
        actual: Box<JsVal>,
    },
    BoundMatcher {
        kind: MatcherKind,
        actual: Box<JsVal>,
    },
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Return(JsVal),
    Throw(JsVal),
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_testing_surface(module, &by_id) {
        return None;
    }
    if !body_ok(&module.body) {
        return None;
    }
    let mut ctx = EvalCtx::new();
    for loc in &module.locals {
        if loc.name == "globalThis" {
            ctx.env
                .insert(loc.id, JsVal::Builtin(BuiltinId::GlobalThis));
        }
    }
    match eval_body(&module.body, &mut ctx) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }
    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match ctx.env.get(local) {
                Some(v @ (JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_))) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String
                    ) {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(_) => {}
                None => return None,
            }
        }
    }
    if user_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        user_locals,
        values,
    })
}

fn ident_builtin(name: &str) -> Option<BuiltinId> {
    match name {
        "globalThis" => Some(BuiltinId::GlobalThis),
        "describe" => Some(BuiltinId::Describe),
        "it" => Some(BuiltinId::It),
        "expect" => Some(BuiltinId::Expect),
        "before" => Some(BuiltinId::Before),
        "after" => Some(BuiltinId::After),
        "beforeEach" => Some(BuiltinId::BeforeEach),
        "afterEach" => Some(BuiltinId::AfterEach),
        _ => None,
    }
}

fn module_has_testing_surface(module: &Module, _by_id: &HashMap<LocalId, &Local>) -> bool {
    module.body.iter().any(stmt_has_testing_surface)
}

fn stmt_has_testing_surface(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Throw { value: e } => {
            expr_has_testing_surface(e)
        }
        Stmt::Return { value: Some(e) } => expr_has_testing_surface(e),
        Stmt::Block { body } => body.iter().any(stmt_has_testing_surface),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_has_testing_surface(test)
                || stmt_has_testing_surface(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_has_testing_surface(a))
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(stmt_has_testing_surface)
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(stmt_has_testing_surface))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(stmt_has_testing_surface))
        }
        _ => false,
    }
}

fn expr_has_testing_surface(expr: &Expr) -> bool {
    match expr {
        Expr::IdentName { name, .. } => matches!(
            name.as_str(),
            "describe" | "it" | "expect" | "before" | "after" | "beforeEach" | "afterEach"
        ),
        Expr::Unary { arg, .. } => expr_has_testing_surface(arg),
        Expr::Binary { left, right, .. } => {
            expr_has_testing_surface(left) || expr_has_testing_surface(right)
        }
        Expr::Call { callee, args, .. } => {
            expr_has_testing_surface(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_testing_surface(e),
                    _ => false,
                })
        }
        Expr::Member {
            object, property, ..
        } => expr_has_testing_surface(object) || expr_has_testing_surface(property),
        Expr::Assign { value, .. } => expr_has_testing_surface(value),
        Expr::Function { body, .. } => body.iter().any(stmt_has_testing_surface),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_testing_surface(test)
                || expr_has_testing_surface(consequent)
                || expr_has_testing_surface(alternate)
        }
        _ => false,
    }
}

fn body_ok(body: &[Stmt]) -> bool {
    body.iter().all(stmt_ok)
}

fn stmt_ok(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init, .. } => match init {
            None => true,
            Some(e) => expr_ok(e),
        },
        Stmt::Expr { expr } => expr_ok(expr),
        Stmt::Throw { value } => expr_ok(value),
        Stmt::Return { value } => value.as_ref().is_none_or(expr_ok),
        Stmt::Block { body } => body_ok(body),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => expr_ok(test) && stmt_ok(consequent) && alternate.as_ref().is_none_or(|a| stmt_ok(a)),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            body_ok(block)
                && handler.as_ref().is_none_or(|h| body_ok(h))
                && finalizer.as_ref().is_none_or(|f| body_ok(f))
        }
        _ => false,
    }
}

fn expr_ok(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Null { .. }
        | Expr::Local { .. } => true,
        Expr::IdentName { name, .. } => ident_builtin(name).is_some(),
        Expr::Unary {
            op: UnaryOp::TypeOf | UnaryOp::Minus | UnaryOp::Plus,
            arg,
            ..
        } => expr_ok(arg),
        Expr::Binary {
            op:
                BinaryOp::EqEqEq
                | BinaryOp::NotEqEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div,
            left,
            right,
            ..
        } => expr_ok(left) && expr_ok(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => expr_ok(test) && expr_ok(consequent) && expr_ok(alternate),
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => expr_ok(object) && expr_ok(property),
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            expr_ok(callee)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_ok(value),
        Expr::Function {
            is_async: false,
            is_generator: false,
            body,
            ..
        } => body_ok(body),
        _ => false,
    }
}

fn eval_body(body: &[Stmt], ctx: &mut EvalCtx) -> Result<Flow, ()> {
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

struct Emitter {
    out: String,
    body: String,
    str_consts: Vec<(String, String)>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
            str_consts: Vec::new(),
        }
    }

    fn string_const(&mut self, s: &str) -> String {
        if let Some((_, name)) = self.str_consts.iter().find(|(v, _)| v == s) {
            return name.clone();
        }
        let name = format!("@.gstr.{}", self.str_consts.len());
        self.str_consts.push((s.to_string(), name.clone()));
        name
    }

    fn emit_num(&mut self, n: f64) {
        let lit = format!("{n:?}");
        writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_testing: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => {
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                JsVal::Bool(b) => {
                    let s = if *b { "true" } else { "false" };
                    let name = self.string_const(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                _ => return Err(diag("es_testing: non-printable value")),
            }
        }
        writeln!(
            self.out,
            "; Draconic LLVM backend (L05.03 describe/it/expect/hooks)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        for (s, name) in &self.str_consts {
            let n = s.len() + 1;
            let mut esc = String::new();
            for b in s.bytes() {
                match b {
                    b'\\' => esc.push_str("\\5C"),
                    b'"' => esc.push_str("\\22"),
                    c if (0x20..0x7f).contains(&c) => esc.push(c as char),
                    c => esc.push_str(&format!("\\{c:02X}")),
                }
            }
            writeln!(
                self.out,
                "{name} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        writeln!(self.out, "\ndefine i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        self.out.push_str(&self.body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn finish(self) -> String {
        self.out
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn compile_src(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn classifies_describe_it_run() {
        let m = compile_src(
            r#"
            let ran = 0;
            let passed;
            describe("suite", () => {
              passed = it("case", () => {
                ran = 1;
              });
            });
            "#,
        );
        assert!(is_es_testing_module(&m));
        let ir = emit_es_testing(&m).expect("emit");
        assert!(ir.contains("define i32 @main()"), "{ir}");
        assert!(ir.contains("1"), "{ir}");
    }

    #[test]
    fn classifies_expect_matchers() {
        let m = compile_src(
            r#"
            let te = typeof expect;
            let ok;
            describe("e", () => {
              ok = it("eq", () => {
                expect(1).toBe(1);
                expect(1).toBeTruthy();
                expect(0).toBeFalsy();
              });
            });
            "#,
        );
        assert!(is_es_testing_module(&m));
        let ir = emit_es_testing(&m).expect("emit");
        assert!(ir.contains("function"), "{ir}");
        assert!(ir.contains("true"), "{ir}");
    }

    #[test]
    fn classifies_expect_fail_messages() {
        let m = compile_src(
            r#"
            let eqHas = false;
            try {
              expect(1).toBe(2);
            } catch (e) {
              eqHas = e === "expected 1 to be 2";
            }
            "#,
        );
        assert!(is_es_testing_module(&m));
        let ir = emit_es_testing(&m).expect("emit");
        assert!(ir.contains("true"), "{ir}");
    }

    #[test]
    fn classifies_nested_hooks() {
        let m = compile_src(
            r#"
            let nestedOk;
            let hookOk;
            let order = "";
            describe("outer", () => {
              before(() => { order = order + "B"; });
              after(() => { order = order + "A"; });
              beforeEach(() => { order = order + "b"; });
              afterEach(() => { order = order + "a"; });
              describe("inner", () => {
                beforeEach(() => { order = order + "i"; });
                afterEach(() => { order = order + "j"; });
                nestedOk = it("case", () => {
                  order = order + "T";
                });
              });
            });
            hookOk = order === "BbiTjaA";
            "#,
        );
        assert!(is_es_testing_module(&m));
        let ir = emit_es_testing(&m).expect("emit");
        assert!(ir.contains("true"), "{ir}");
        assert!(ir.contains("BbiTjaA"), "{ir}");
    }
}
