//! L01.01: native observations for UTF-8 TextEncoder / TextDecoder.
//!
//! Compile-time evaluation of TextEncoder/TextDecoder encode/decode plus
//! fatal invalid UTF-8 TypeError. Emits Runtime prints of final top-level
//! number/string/bool locals.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

pub(crate) fn is_es_encoding_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_encoding(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_encoding module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum BuiltinId {
    GlobalThis,
    TextEncoder,
    TextDecoder,
    Uint8Array,
    TypeError,
}

#[derive(Clone, Debug, PartialEq)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Builtin(BuiltinId),
    ErrorInst {
        name: String,
        message: String,
    },
    TextEncoderInst,
    TextDecoderInst {
        fatal: bool,
    },
    Uint8ArrayInst {
        bytes: Rc<RefCell<Vec<u8>>>,
    },
    Object {
        props: Vec<(String, JsVal)>,
    },
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

enum Flow {
    Normal,
    Throw(JsVal),
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_encoding_surface(module, &by_id) {
        return None;
    }
    if !body_ok(&module.body) {
        return None;
    }
    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    for loc in &module.locals {
        if let Some(b) = builtin_for_name(&loc.name) {
            env.insert(loc.id, JsVal::Builtin(b));
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

fn builtin_for_name(name: &str) -> Option<BuiltinId> {
    match name {
        "globalThis" => Some(BuiltinId::GlobalThis),
        "TextEncoder" => Some(BuiltinId::TextEncoder),
        "TextDecoder" => Some(BuiltinId::TextDecoder),
        "Uint8Array" => Some(BuiltinId::Uint8Array),
        "TypeError" => Some(BuiltinId::TypeError),
        _ => None,
    }
}

fn module_has_encoding_surface(module: &Module, by_id: &HashMap<LocalId, &Local>) -> bool {
    module
        .body
        .iter()
        .any(|s| stmt_has_encoding_surface(s, by_id))
}

fn stmt_has_encoding_surface(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Throw { value: e } => {
            expr_has_encoding_surface(e, by_id)
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(|s| stmt_has_encoding_surface(s, by_id))
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(|s| stmt_has_encoding_surface(s, by_id)))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(|s| stmt_has_encoding_surface(s, by_id)))
        }
        Stmt::Block { body } => body.iter().any(|s| stmt_has_encoding_surface(s, by_id)),
        _ => false,
    }
}

fn expr_has_encoding_surface(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, .. } => by_id
            .get(id)
            .is_some_and(|l| matches!(l.name.as_str(), "TextEncoder" | "TextDecoder")),
        Expr::Unary { arg, .. } => expr_has_encoding_surface(arg, by_id),
        Expr::Binary { left, right, .. } => {
            expr_has_encoding_surface(left, by_id) || expr_has_encoding_surface(right, by_id)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_encoding_surface(test, by_id)
                || expr_has_encoding_surface(consequent, by_id)
                || expr_has_encoding_surface(alternate, by_id)
        }
        Expr::Member { object, property, .. } => {
            expr_has_encoding_surface(object, by_id) || expr_has_encoding_surface(property, by_id)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_has_encoding_surface(callee, by_id)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_encoding_surface(e, by_id),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_has_encoding_surface(value, by_id),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_has_encoding_surface(e, by_id),
            ArrayElement::Elision => false,
        }),
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                expr_has_encoding_surface(value, by_id)
            }
            ObjectProp::Spread(e) => expr_has_encoding_surface(e, by_id),
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
        Expr::Number { .. }
        | Expr::Boolean { .. }
        | Expr::String { .. }
        | Expr::Null { .. }
        | Expr::Local { .. } => true,
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
        }
        | Expr::New { callee, args, .. } => {
            expr_ok(callee)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_ok(value),
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => expr_ok(e),
            ArrayElement::Elision => true,
            ArrayElement::Spread(_) => false,
        }),
        Expr::Object { properties, .. } => properties.iter().all(|p| match p {
            ObjectProp::Property {
                key: ObjectPropKey::Static(_),
                value,
            } => expr_ok(value),
            ObjectProp::Property {
                key: ObjectPropKey::Computed(k),
                value,
            } => expr_ok(k) && expr_ok(value),
            _ => false,
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

fn eval_expr(expr: &Expr, env: &mut HashMap<LocalId, JsVal>) -> Result<Result<JsVal, Flow>, ()> {
    match expr {
        Expr::Number { raw, .. } => Ok(Ok(JsVal::Num(raw.parse().map_err(|_| ())?))),
        Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
        Expr::String { value, .. } => Ok(Ok(JsVal::Str(js_string_to_utf8(value)))),
        Expr::Null { .. } => Ok(Ok(JsVal::Undef)),
        Expr::Local { id, .. } => Ok(Ok(env.get(id).cloned().ok_or(())?)),
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
            match eval_new(&c, &arg_vals) {
                Ok(v) => Ok(Ok(v)),
                Err(Some(flow)) => Ok(Err(flow)),
                Err(None) => Err(()),
            }
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
            if let Expr::Member {
                object,
                property,
                optional: false,
                ..
            } = callee.as_ref()
            {
                let obj = match eval_expr(object, env)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(Err(flow)),
                };
                let key = match eval_key(property, env)? {
                    Ok(k) => k,
                    Err(flow) => return Ok(Err(flow)),
                };
                return match eval_method_call(&obj, &key, &arg_vals) {
                    Ok(v) => Ok(Ok(v)),
                    Err(Some(flow)) => Ok(Err(flow)),
                    Err(None) => Err(()),
                };
            }
            Err(())
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
            Ok(Ok(JsVal::Object {
                props: out
                    .into_iter()
                    .enumerate()
                    .map(|(i, v)| (i.to_string(), v))
                    .collect(),
            }))
        }
        Expr::Object { properties, .. } => {
            let mut props = Vec::new();
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key: ObjectPropKey::Static(s),
                        value,
                    } => {
                        let v = match eval_expr(value, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        props.push((js_string_to_utf8(s), v));
                    }
                    ObjectProp::Property {
                        key: ObjectPropKey::Computed(ke),
                        value,
                    } => {
                        let key = match eval_key(ke, env)? {
                            Ok(k) => k,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        let v = match eval_expr(value, env)? {
                            Ok(v) => v,
                            Err(flow) => return Ok(Err(flow)),
                        };
                        props.push((key, v));
                    }
                    _ => return Err(()),
                }
            }
            Ok(Ok(JsVal::Object { props }))
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

fn eval_new(callee: &JsVal, args: &[JsVal]) -> Result<JsVal, Option<Flow>> {
    let JsVal::Builtin(b) = callee else {
        return Err(None);
    };
    match b {
        BuiltinId::TextEncoder => {
            if !args.is_empty() {
                return Err(None);
            }
            Ok(JsVal::TextEncoderInst)
        }
        BuiltinId::TextDecoder => {
            let mut fatal = false;
            if let Some(label) = args.first() {
                match label {
                    JsVal::Str(s) => {
                        let t = s.to_ascii_lowercase();
                        if !(t.is_empty() || t == "utf-8" || t == "utf8") {
                            return Err(None);
                        }
                    }
                    JsVal::Undef => {}
                    _ => return Err(None),
                }
            }
            if let Some(opts) = args.get(1) {
                match opts {
                    JsVal::Object { props } => {
                        if let Some((_, v)) = props.iter().find(|(k, _)| k == "fatal") {
                            fatal = match v {
                                JsVal::Bool(b) => *b,
                                _ => return Err(None),
                            };
                        }
                    }
                    JsVal::Undef => {}
                    _ => return Err(None),
                }
            }
            Ok(JsVal::TextDecoderInst { fatal })
        }
        BuiltinId::Uint8Array => {
            let first = args.first().ok_or(None)?;
            match first {
                JsVal::Object { props } => {
                    let mut pairs: Vec<(usize, u8)> = Vec::new();
                    for (k, v) in props {
                        let idx: usize = k.parse().map_err(|_| None)?;
                        let n = match v {
                            JsVal::Num(n) => *n as u8,
                            _ => return Err(None),
                        };
                        pairs.push((idx, n));
                    }
                    pairs.sort_by_key(|(i, _)| *i);
                    let mut bytes = vec![0u8; pairs.len()];
                    for (i, (idx, b)) in pairs.iter().enumerate() {
                        if *idx != i {
                            return Err(None);
                        }
                        bytes[i] = *b;
                    }
                    Ok(JsVal::Uint8ArrayInst {
                        bytes: Rc::new(RefCell::new(bytes)),
                    })
                }
                JsVal::Num(n) if *n >= 0.0 && n.is_finite() => Ok(JsVal::Uint8ArrayInst {
                    bytes: Rc::new(RefCell::new(vec![0u8; *n as usize])),
                }),
                _ => Err(None),
            }
        }
        BuiltinId::TypeError => {
            let message = match args.first() {
                Some(JsVal::Str(s)) => s.clone(),
                Some(JsVal::Undef) | None => String::new(),
                _ => return Err(None),
            };
            Ok(JsVal::ErrorInst {
                name: "TypeError".into(),
                message,
            })
        }
        _ => Err(None),
    }
}

fn eval_method_call(recv: &JsVal, key: &str, args: &[JsVal]) -> Result<JsVal, Option<Flow>> {
    match recv {
        JsVal::TextEncoderInst if key == "encode" => {
            let s = match args.first() {
                Some(JsVal::Str(s)) => s.as_str(),
                Some(JsVal::Undef) | None => "",
                _ => return Err(None),
            };
            Ok(JsVal::Uint8ArrayInst {
                bytes: Rc::new(RefCell::new(s.as_bytes().to_vec())),
            })
        }
        JsVal::TextDecoderInst { fatal } if key == "decode" => {
            let bytes = match args.first() {
                Some(JsVal::Uint8ArrayInst { bytes }) => bytes.borrow().clone(),
                Some(JsVal::Undef) | None => Vec::new(),
                _ => return Err(None),
            };
            match std::str::from_utf8(&bytes) {
                Ok(s) => Ok(JsVal::Str(s.to_string())),
                Err(_) if *fatal => Err(Some(Flow::Throw(JsVal::ErrorInst {
                    name: "TypeError".into(),
                    message: "The encoded data was not valid for encoding utf-8".into(),
                }))),
                Err(_) => Ok(JsVal::Str(String::from_utf8_lossy(&bytes).into_owned())),
            }
        }
        _ => Err(None),
    }
}

fn member_get(obj: &JsVal, key: &str) -> Result<JsVal, ()> {
    match obj {
        JsVal::Builtin(BuiltinId::GlobalThis) => match key {
            "TextEncoder" => Ok(JsVal::Builtin(BuiltinId::TextEncoder)),
            "TextDecoder" => Ok(JsVal::Builtin(BuiltinId::TextDecoder)),
            "Uint8Array" => Ok(JsVal::Builtin(BuiltinId::Uint8Array)),
            "TypeError" => Ok(JsVal::Builtin(BuiltinId::TypeError)),
            "globalThis" => Ok(JsVal::Builtin(BuiltinId::GlobalThis)),
            _ => Err(()),
        },
        JsVal::Uint8ArrayInst { bytes } if key == "length" => {
            Ok(JsVal::Num(bytes.borrow().len() as f64))
        }
        JsVal::Uint8ArrayInst { bytes } => {
            let idx: usize = key.parse().map_err(|_| ())?;
            let b = bytes.borrow();
            if idx >= b.len() {
                return Ok(JsVal::Undef);
            }
            Ok(JsVal::Num(b[idx] as f64))
        }
        JsVal::ErrorInst { name, message } => match key {
            "name" => Ok(JsVal::Str(name.clone())),
            "message" => Ok(JsVal::Str(message.clone())),
            _ => Err(()),
        },
        JsVal::Object { props } => props
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.clone())
            .ok_or(()),
        _ => Err(()),
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::ErrorInst { .. }
        | JsVal::TextEncoderInst
        | JsVal::TextDecoderInst { .. }
        | JsVal::Uint8ArrayInst { .. }
        | JsVal::Object { .. } => "object".into(),
        JsVal::Builtin(
            BuiltinId::TextEncoder
            | BuiltinId::TextDecoder
            | BuiltinId::Uint8Array
            | BuiltinId::TypeError,
        ) => "function".into(),
        JsVal::Builtin(BuiltinId::GlobalThis) => "object".into(),
    }
}

fn strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => x == y,
        (JsVal::Bool(x), JsVal::Bool(y)) => x == y,
        (JsVal::Str(x), JsVal::Str(y)) => x == y,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Builtin(x), JsVal::Builtin(y)) => x == y,
        (
            JsVal::ErrorInst {
                name: n1,
                message: m1,
            },
            JsVal::ErrorInst {
                name: n2,
                message: m2,
            },
        ) => n1 == n2 && m1 == m2,
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
        writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_encoding: missing value"))?;
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
                _ => return Err(diag("es_encoding: non-printable value")),
            }
        }
        writeln!(
            self.out,
            "; Draconic LLVM backend (L01.01 TextEncoder/TextDecoder UTF-8)"
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
    fn classifies_text_encoder_roundtrip() {
        let m = compile_src(
            r#"
            let bytes = new TextEncoder().encode("hi");
            let len = bytes.length;
            let s = new TextDecoder().decode(bytes);
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }

    #[test]
    fn classifies_fatal_invalid_utf8() {
        let m = compile_src(
            r#"
            let ok = 0;
            try {
              new TextDecoder("utf-8", { fatal: true }).decode(new Uint8Array([255]));
              ok = -1;
            } catch (e) {
              ok = e.name === "TypeError" ? 1 : -2;
            }
            "#,
        );
        assert!(is_es_encoding_module(&m));
        let ir = emit_es_encoding(&m).expect("emit");
        assert!(ir.contains("@main"));
    }
}
