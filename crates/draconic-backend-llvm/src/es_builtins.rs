//! N08.14.01–N08.14.03: native observations for global builtins + Error ctors + global functions.
//!
//! Compile-time evaluation of:
//! - E15.01: `undefined`, `globalThis`, `Object`/`Function`/`Array`/`String`/`Boolean`
//! - E15.02: `Error` / `TypeError` / `RangeError` / `ReferenceError` / `SyntaxError` /
//!   `URIError` / `EvalError` / `AggregateError` (`typeof`, `globalThis` identity,
//!   `new …(msg)`, `.name`/`.message`/`.errors.length`, throw+catch)
//! - E15.03: `parseInt` / `parseFloat` / `isNaN` / `isFinite` (`typeof`, `globalThis`
//!   identity, basic call behavior; `NaN` / `Infinity` globals)
//!
//! Emits Runtime prints of final top-level number/string/bool locals.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

pub(crate) fn is_es_builtins_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_builtins(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_builtins module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum BuiltinId {
    Undefined,
    GlobalThis,
    Object,
    Function,
    Array,
    String,
    Boolean,
    ObjectPrototype,
    ArrayIsArray,
    Error,
    TypeError,
    RangeError,
    ReferenceError,
    SyntaxError,
    UriError,
    EvalError,
    AggregateError,
    ParseInt,
    ParseFloat,
    IsNaN,
    IsFinite,
    Nan,
    Infinity,
}

#[derive(Clone, Debug, PartialEq)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Builtin(BuiltinId),
    /// Error instance: name, message, optional AggregateError `.errors` array.
    ErrorInst {
        name: String,
        message: String,
        errors: Option<Vec<JsVal>>,
    },
    Array(Vec<JsVal>),
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Throw(JsVal),
}

struct Emitter {
    out: String,
    body: String,
    str_consts: Vec<(String, String)>,
}

fn js_string_to_utf8(s: &JsString) -> String {
    s.to_string_lossy()
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_builtin_surface(module, &by_id) {
        return None;
    }
    if !body_ok(&module.body) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    for loc in &module.locals {
        if let Some(b) = builtin_for_name(&loc.name) {
            env.insert(
                loc.id,
                match b {
                    BuiltinId::Undefined => JsVal::Undef,
                    BuiltinId::Nan => JsVal::Num(f64::NAN),
                    BuiltinId::Infinity => JsVal::Num(f64::INFINITY),
                    other => JsVal::Builtin(other),
                },
            );
        }
    }

    match eval_body(&module.body, &mut env) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }

    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match env.get(local) {
                Some(v @ (JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_))) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String
                    ) {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(JsVal::Undef | JsVal::Builtin(_) | JsVal::ErrorInst { .. } | JsVal::Array(_)) => {
                }
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

fn builtin_for_name(name: &str) -> Option<BuiltinId> {
    match name {
        "undefined" => Some(BuiltinId::Undefined),
        "globalThis" => Some(BuiltinId::GlobalThis),
        "Object" => Some(BuiltinId::Object),
        "Function" => Some(BuiltinId::Function),
        "Array" => Some(BuiltinId::Array),
        "String" => Some(BuiltinId::String),
        "Boolean" => Some(BuiltinId::Boolean),
        "Error" => Some(BuiltinId::Error),
        "TypeError" => Some(BuiltinId::TypeError),
        "RangeError" => Some(BuiltinId::RangeError),
        "ReferenceError" => Some(BuiltinId::ReferenceError),
        "SyntaxError" => Some(BuiltinId::SyntaxError),
        "URIError" => Some(BuiltinId::UriError),
        "EvalError" => Some(BuiltinId::EvalError),
        "AggregateError" => Some(BuiltinId::AggregateError),
        "parseInt" => Some(BuiltinId::ParseInt),
        "parseFloat" => Some(BuiltinId::ParseFloat),
        "isNaN" => Some(BuiltinId::IsNaN),
        "isFinite" => Some(BuiltinId::IsFinite),
        "NaN" => Some(BuiltinId::Nan),
        "Infinity" => Some(BuiltinId::Infinity),
        _ => None,
    }
}

fn error_ctor_name(b: BuiltinId) -> Option<&'static str> {
    match b {
        BuiltinId::Error => Some("Error"),
        BuiltinId::TypeError => Some("TypeError"),
        BuiltinId::RangeError => Some("RangeError"),
        BuiltinId::ReferenceError => Some("ReferenceError"),
        BuiltinId::SyntaxError => Some("SyntaxError"),
        BuiltinId::UriError => Some("URIError"),
        BuiltinId::EvalError => Some("EvalError"),
        BuiltinId::AggregateError => Some("AggregateError"),
        _ => None,
    }
}

fn module_has_builtin_surface(module: &Module, by_id: &HashMap<LocalId, &Local>) -> bool {
    module.body.iter().any(|s| stmt_has_builtin_surface(s, by_id))
}

fn stmt_has_builtin_surface(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Throw { value: e } => {
            expr_has_builtin_surface(e, by_id)
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block
                .iter()
                .any(|s| stmt_has_builtin_surface(s, by_id))
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(|s| stmt_has_builtin_surface(s, by_id)))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(|s| stmt_has_builtin_surface(s, by_id)))
        }
        Stmt::Block { body } => body.iter().any(|s| stmt_has_builtin_surface(s, by_id)),
        _ => false,
    }
}

fn expr_has_builtin_surface(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, .. } => by_id.get(id).is_some_and(|l| builtin_for_name(&l.name).is_some()),
        Expr::Unary { arg, .. } => expr_has_builtin_surface(arg, by_id),
        Expr::Binary { left, right, .. } => {
            expr_has_builtin_surface(left, by_id) || expr_has_builtin_surface(right, by_id)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_builtin_surface(test, by_id)
                || expr_has_builtin_surface(consequent, by_id)
                || expr_has_builtin_surface(alternate, by_id)
        }
        Expr::Member { object, property, .. } => {
            expr_has_builtin_surface(object, by_id) || expr_has_builtin_surface(property, by_id)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_has_builtin_surface(callee, by_id)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_builtin_surface(e, by_id),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_has_builtin_surface(value, by_id),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_has_builtin_surface(e, by_id),
            ArrayElement::Elision => false,
        }),
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
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            match (handler.is_some(), handler_param) {
                (true, None) | (true, Some(Pattern::Local(_))) | (false, None) => {}
                _ => return false,
            }
            body_ok(block)
                && handler.as_ref().is_none_or(|h| body_ok(h))
                && finalizer.as_ref().is_none_or(|f| body_ok(f))
        }
        Stmt::Block { body } => body_ok(body),
        _ => false,
    }
}

fn expr_ok(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } => true,
        Expr::Local { .. } => true,
        Expr::Unary {
            op: UnaryOp::TypeOf | UnaryOp::Minus | UnaryOp::Plus,
            arg,
            ..
        } => expr_ok(arg),
        Expr::Binary { left, right, op, .. } => {
            matches!(
                op,
                BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
                    | BinaryOp::EqEq
                    | BinaryOp::NotEq
                    | BinaryOp::And
                    | BinaryOp::Or
            ) && expr_ok(left)
                && expr_ok(right)
        }
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
        Expr::New {
            callee,
            args,
            ..
        }
        | Expr::Call {
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
        Expr::Assign {
            target: AssignTarget::Local(_),
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(value),
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => expr_ok(e),
            ArrayElement::Elision => true,
            ArrayElement::Spread(_) => false,
        }),
        _ => false,
    }
}

fn eval_body(body: &[Stmt], env: &mut HashMap<LocalId, JsVal>) -> Result<Flow, ()> {
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

/// `Ok(Ok(v))` = value; `Ok(Err(flow))` = abrupt throw; `Err(())` = unsupported.
fn eval_expr(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<JsVal, Flow>, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| ())?;
            Ok(Ok(JsVal::Num(n)))
        }
        Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
        Expr::String { value, .. } => Ok(Ok(JsVal::Str(js_string_to_utf8(value)))),
        Expr::Local { id, .. } => {
            let v = env.get(id).cloned().ok_or(())?;
            Ok(Ok(v))
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
            left,
            op,
            right,
            ..
        } => {
            let l = match eval_expr(left, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
            match op {
                BinaryOp::And => {
                    if !to_boolean(&l) {
                        return Ok(Ok(l));
                    }
                    eval_expr(right, env)
                }
                BinaryOp::Or => {
                    if to_boolean(&l) {
                        return Ok(Ok(l));
                    }
                    eval_expr(right, env)
                }
                BinaryOp::EqEqEq | BinaryOp::EqEq => {
                    let r = match eval_expr(right, env)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(Err(flow)),
                    };
                    Ok(Ok(JsVal::Bool(strict_eq(&l, &r))))
                }
                BinaryOp::NotEqEq | BinaryOp::NotEq => {
                    let r = match eval_expr(right, env)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(Err(flow)),
                    };
                    Ok(Ok(JsVal::Bool(!strict_eq(&l, &r))))
                }
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
            Ok(Ok(member_get(&obj, &key)?))
        }
        Expr::New { callee, args, .. } => {
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
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
            Ok(Ok(eval_new(&c, &arg_vals)?))
        }
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } => {
            let c = match eval_expr(callee, env)? {
                Ok(v) => v,
                Err(flow) => return Ok(Err(flow)),
            };
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
            Ok(Ok(eval_call(&c, &arg_vals)?))
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
        Expr::Array { elements, .. } => {
            let mut out = Vec::new();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) => match eval_expr(e, env)? {
                        Ok(v) => out.push(v),
                        Err(flow) => return Ok(Err(flow)),
                    },
                    ArrayElement::Elision => out.push(JsVal::Undef),
                    ArrayElement::Spread(_) => return Err(()),
                }
            }
            Ok(Ok(JsVal::Array(out)))
        }
        _ => Err(()),
    }
}

fn eval_key(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<String, Flow>, ()> {
    match expr {
        Expr::String { value, .. } => Ok(Ok(js_string_to_utf8(value))),
        e => match eval_expr(e, env)? {
            Ok(JsVal::Str(s)) => Ok(Ok(s)),
            Ok(JsVal::Num(n)) => Ok(Ok(format!("{}", n as i64))),
            Ok(_) => Err(()),
            Err(flow) => Ok(Err(flow)),
        },
    }
}

fn eval_new(callee: &JsVal, args: &[JsVal]) -> Result<JsVal, ()> {
    let JsVal::Builtin(b) = callee else {
        return Err(());
    };
    let name = error_ctor_name(*b).ok_or(())?;
    if *b == BuiltinId::AggregateError {
        let errors = match args.first() {
            Some(JsVal::Array(a)) => a.clone(),
            _ => return Err(()),
        };
        let message = match args.get(1) {
            Some(JsVal::Str(s)) => s.clone(),
            Some(JsVal::Undef) | None => String::new(),
            _ => return Err(()),
        };
        return Ok(JsVal::ErrorInst {
            name: name.into(),
            message,
            errors: Some(errors),
        });
    }
    let message = match args.first() {
        Some(JsVal::Str(s)) => s.clone(),
        Some(JsVal::Undef) | None => String::new(),
        _ => return Err(()),
    };
    Ok(JsVal::ErrorInst {
        name: name.into(),
        message,
        errors: None,
    })
}

fn eval_call(callee: &JsVal, args: &[JsVal]) -> Result<JsVal, ()> {
    let JsVal::Builtin(b) = callee else {
        return Err(());
    };
    match b {
        BuiltinId::ParseInt => {
            let s = match args.first() {
                Some(JsVal::Str(s)) => s.as_str(),
                Some(JsVal::Num(n)) => {
                    // ToString(number) for fixture depth; only decimals we need.
                    return Ok(JsVal::Num(js_parse_int(&format!("{n}"), args.get(1))?));
                }
                _ => return Err(()),
            };
            Ok(JsVal::Num(js_parse_int(s, args.get(1))?))
        }
        BuiltinId::ParseFloat => {
            let s = match args.first() {
                Some(JsVal::Str(s)) => s.as_str(),
                Some(JsVal::Num(n)) => return Ok(JsVal::Num(*n)),
                _ => return Err(()),
            };
            Ok(JsVal::Num(js_parse_float(s)))
        }
        BuiltinId::IsNaN => {
            let n = to_number(args.first().unwrap_or(&JsVal::Undef))?;
            Ok(JsVal::Bool(n.is_nan()))
        }
        BuiltinId::IsFinite => {
            let n = to_number(args.first().unwrap_or(&JsVal::Undef))?;
            Ok(JsVal::Bool(n.is_finite()))
        }
        _ => Err(()),
    }
}

fn to_number(v: &JsVal) -> Result<f64, ()> {
    match v {
        JsVal::Num(n) => Ok(*n),
        JsVal::Bool(true) => Ok(1.0),
        JsVal::Bool(false) => Ok(0.0),
        JsVal::Undef => Ok(f64::NAN),
        JsVal::Str(s) => Ok(js_string_to_number(s)),
        JsVal::Builtin(BuiltinId::Nan) => Ok(f64::NAN),
        JsVal::Builtin(BuiltinId::Infinity) => Ok(f64::INFINITY),
        _ => Err(()),
    }
}

/// ECMA-262 ToNumber on string (subset used by E15.03 fixtures).
fn js_string_to_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if t.eq_ignore_ascii_case("infinity") || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// ECMA-262 parseInt (string, radix) for fixture cases.
fn js_parse_int(input: &str, radix_arg: Option<&JsVal>) -> Result<f64, ()> {
    let s = input.trim_start();
    if s.is_empty() {
        return Ok(f64::NAN);
    }
    let mut radix = match radix_arg {
        None | Some(JsVal::Undef) => 0i32,
        Some(JsVal::Num(n)) => {
            if !n.is_finite() {
                return Ok(f64::NAN);
            }
            *n as i32
        }
        _ => return Err(()),
    };
    let mut chars = s.chars().peekable();
    let mut sign = 1.0f64;
    if let Some(&c) = chars.peek() {
        if c == '+' {
            chars.next();
        } else if c == '-' {
            sign = -1.0;
            chars.next();
        }
    }
    let rest: String = chars.collect();
    let mut body = rest.as_str();
    if radix == 0 {
        if body.starts_with("0x") || body.starts_with("0X") {
            radix = 16;
            body = &body[2..];
        } else {
            radix = 10;
        }
    } else if radix == 16 && (body.starts_with("0x") || body.starts_with("0X")) {
        body = &body[2..];
    }
    if !(2..=36).contains(&radix) {
        return Ok(f64::NAN);
    }
    let radix_u = radix as u32;
    let mut acc: i64 = 0;
    let mut any = false;
    for c in body.chars() {
        let dig = match c.to_digit(radix_u) {
            Some(d) => d as i64,
            None => break,
        };
        any = true;
        acc = acc
            .checked_mul(radix as i64)
            .and_then(|a| a.checked_add(dig))
            .unwrap_or(i64::MAX);
    }
    if !any {
        return Ok(f64::NAN);
    }
    Ok(sign * acc as f64)
}

/// ECMA-262 parseFloat (string) for fixture cases.
fn js_parse_float(input: &str) -> f64 {
    let s = input.trim_start();
    if s.is_empty() {
        return f64::NAN;
    }
    // Scan a JS-like float prefix: optional sign, digits, optional fraction/exponent.
    let bytes = s.as_bytes();
    let mut i = 0usize;
    if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
        i += 1;
    }
    let start = i;
    let mut saw_digit = false;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        saw_digit = true;
        i += 1;
    }
    if i < bytes.len() && bytes[i] == b'.' {
        i += 1;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            saw_digit = true;
            i += 1;
        }
    }
    if !saw_digit {
        // Infinity?
        let rest = &s[start.min(s.len())..];
        if rest.len() >= 8 && rest[..8].eq_ignore_ascii_case("Infinity") {
            return if s.starts_with('-') {
                f64::NEG_INFINITY
            } else {
                f64::INFINITY
            };
        }
        return f64::NAN;
    }
    if i < bytes.len() && (bytes[i] == b'e' || bytes[i] == b'E') {
        let e_pos = i;
        i += 1;
        if i < bytes.len() && (bytes[i] == b'+' || bytes[i] == b'-') {
            i += 1;
        }
        let exp_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if exp_start == i {
            i = e_pos; // no exponent digits → stop before e
        }
    }
    s[..i].parse::<f64>().unwrap_or(f64::NAN)
}

fn member_get(obj: &JsVal, key: &str) -> Result<JsVal, ()> {
    match obj {
        JsVal::Builtin(BuiltinId::GlobalThis) => match key {
            "Object" => Ok(JsVal::Builtin(BuiltinId::Object)),
            "Function" => Ok(JsVal::Builtin(BuiltinId::Function)),
            "Array" => Ok(JsVal::Builtin(BuiltinId::Array)),
            "String" => Ok(JsVal::Builtin(BuiltinId::String)),
            "Boolean" => Ok(JsVal::Builtin(BuiltinId::Boolean)),
            "Error" => Ok(JsVal::Builtin(BuiltinId::Error)),
            "TypeError" => Ok(JsVal::Builtin(BuiltinId::TypeError)),
            "RangeError" => Ok(JsVal::Builtin(BuiltinId::RangeError)),
            "ReferenceError" => Ok(JsVal::Builtin(BuiltinId::ReferenceError)),
            "SyntaxError" => Ok(JsVal::Builtin(BuiltinId::SyntaxError)),
            "URIError" => Ok(JsVal::Builtin(BuiltinId::UriError)),
            "EvalError" => Ok(JsVal::Builtin(BuiltinId::EvalError)),
            "AggregateError" => Ok(JsVal::Builtin(BuiltinId::AggregateError)),
            "parseInt" => Ok(JsVal::Builtin(BuiltinId::ParseInt)),
            "parseFloat" => Ok(JsVal::Builtin(BuiltinId::ParseFloat)),
            "isNaN" => Ok(JsVal::Builtin(BuiltinId::IsNaN)),
            "isFinite" => Ok(JsVal::Builtin(BuiltinId::IsFinite)),
            "NaN" => Ok(JsVal::Num(f64::NAN)),
            "Infinity" => Ok(JsVal::Num(f64::INFINITY)),
            "undefined" => Ok(JsVal::Undef),
            "globalThis" => Ok(JsVal::Builtin(BuiltinId::GlobalThis)),
            _ => Err(()),
        },
        JsVal::Builtin(BuiltinId::Object) if key == "prototype" => {
            Ok(JsVal::Builtin(BuiltinId::ObjectPrototype))
        }
        JsVal::Builtin(BuiltinId::Array) if key == "isArray" => {
            Ok(JsVal::Builtin(BuiltinId::ArrayIsArray))
        }
        JsVal::ErrorInst {
            name,
            message,
            errors,
        } => match key {
            "name" => Ok(JsVal::Str(name.clone())),
            "message" => Ok(JsVal::Str(message.clone())),
            "errors" => match errors {
                Some(a) => Ok(JsVal::Array(a.clone())),
                None => Err(()),
            },
            _ => Err(()),
        },
        JsVal::Array(elems) if key == "length" => Ok(JsVal::Num(elems.len() as f64)),
        _ => Err(()),
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::Array(_) | JsVal::ErrorInst { .. } => "object".into(),
        JsVal::Builtin(BuiltinId::Undefined) => "undefined".into(),
        JsVal::Builtin(BuiltinId::Nan | BuiltinId::Infinity) => "number".into(),
        JsVal::Builtin(BuiltinId::GlobalThis | BuiltinId::ObjectPrototype) => "object".into(),
        JsVal::Builtin(
            BuiltinId::Object
            | BuiltinId::Function
            | BuiltinId::Array
            | BuiltinId::String
            | BuiltinId::Boolean
            | BuiltinId::ArrayIsArray
            | BuiltinId::Error
            | BuiltinId::TypeError
            | BuiltinId::RangeError
            | BuiltinId::ReferenceError
            | BuiltinId::SyntaxError
            | BuiltinId::UriError
            | BuiltinId::EvalError
            | BuiltinId::AggregateError
            | BuiltinId::ParseInt
            | BuiltinId::ParseFloat
            | BuiltinId::IsNaN
            | BuiltinId::IsFinite,
        ) => "function".into(),
    }
}

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef => false,
        JsVal::Builtin(_) | JsVal::ErrorInst { .. } | JsVal::Array(_) => true,
    }
}

fn strict_eq(l: &JsVal, r: &JsVal) -> bool {
    match (l, r) {
        (JsVal::Num(a), JsVal::Num(b)) => a == b,
        (JsVal::Bool(a), JsVal::Bool(b)) => a == b,
        (JsVal::Str(a), JsVal::Str(b)) => a == b,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Builtin(a), JsVal::Builtin(b)) => a == b,
        (JsVal::Undef, JsVal::Builtin(BuiltinId::Undefined))
        | (JsVal::Builtin(BuiltinId::Undefined), JsVal::Undef) => true,
        (
            JsVal::ErrorInst {
                name: n1,
                message: m1,
                errors: e1,
            },
            JsVal::ErrorInst {
                name: n2,
                message: m2,
                errors: e2,
            },
        ) => n1 == n2 && m1 == m2 && e1 == e2,
        (JsVal::Array(a), JsVal::Array(b)) => a == b,
        _ => false,
    }
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
        let lit = if n.is_nan() {
            "0x7FF8000000000000".to_string()
        } else if n.is_infinite() {
            if n.is_sign_negative() {
                "0xFFF0000000000000".into()
            } else {
                "0x7FF0000000000000".into()
            }
        } else {
            format!("{n:?}")
        };
        writeln!(
            self.body,
            "  {}",
            PRINT_F64.call(&format!("double {lit}"))
        )
        .ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_builtins: missing value"))?;
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
                _ => return Err(diag("es_builtins: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.14.01–N08.14.03 global builtins / Error ctors / functions)"
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

    fn compile(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn global_basics_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/builtins/global_basics.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("undefined") && ir.contains("object") && ir.contains("function"),
            "should print typeof observations:\n{ir}"
        );
        assert!(
            ir.contains("true"),
            "should print boolean identity observations:\n{ir}"
        );
    }

    #[test]
    fn error_ctors_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/builtins/error_ctors.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in [
            "function",
            "true",
            "Error",
            "msg",
            "TypeError",
            "RangeError",
            "ReferenceError",
            "SyntaxError",
            "URIError",
            "EvalError",
            "AggregateError",
            "a",
        ] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
        // thr final value 1 and agl 2 as f64 prints
        assert!(
            ir.contains("double 1") || ir.contains("double 1.0"),
            "should print thr=1:\n{ir}"
        );
        assert!(
            ir.contains("double 2") || ir.contains("double 2.0"),
            "should print agl=2:\n{ir}"
        );
    }

    #[test]
    fn global_functions_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/builtins/global_functions.drac");
        let m = compile(src);
        assert!(is_es_builtins_module(&m), "should classify as es_builtins");
        let ir = emit_es_builtins(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["function", "true", "false"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
        assert!(
            ir.contains("double 42") || ir.contains("double 42.0"),
            "should print parseInt 42:\n{ir}"
        );
        assert!(
            ir.contains("double 16") || ir.contains("double 16.0"),
            "should print parseInt hex 16:\n{ir}"
        );
        assert!(
            ir.contains("double 3.14") || ir.contains("3.14"),
            "should print parseFloat 3.14:\n{ir}"
        );
        assert!(
            ir.contains("double 100") || ir.contains("double 100.0"),
            "should print parseFloat 1e2 → 100:\n{ir}"
        );
    }
}
