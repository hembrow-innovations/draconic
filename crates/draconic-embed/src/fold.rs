//! Outer Program fold for N07 eval/Function (fold-at-emit).
//!
//! One interpreter / value domain for the eval subset. LLVM only prints the
//! resulting observations.

use std::collections::{HashMap, HashSet};

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, Local, LocalId, Module, Pattern, Stmt};

use crate::{eval_function_call, eval_source_with_bindings, EmbedValue};

/// Observable value for native stdout after fold-at-emit.
#[derive(Debug, Clone, PartialEq)]
pub enum Observation {
    Number(f64),
    String(String),
    Bool(bool),
    /// Callable (eval, Function, user/dyn fn) — prints as `function`.
    Function,
}

/// Fold an IR module that uses `eval` / `Function`; return observation values
/// for top-level user `let` bindings in declaration order.
pub fn fold_eval_program(module: &Module) -> Result<Vec<Observation>, Diagnostic> {
    try_fold(module).map_err(diag)
}

/// True when this module is the supported eval/Function subset.
pub fn is_eval_fold_module(module: &Module) -> bool {
    match try_fold(module) {
        Ok(obs) => !obs.is_empty(),
        Err(_) => false,
    }
}

/// Runtime fold value: primitives are [`EmbedValue`]; callables / globalThis
/// are fold-only tags (no cross-crate mapping).
#[derive(Debug, Clone)]
enum Value {
    Prim(EmbedValue),
    BuiltinEval,
    BuiltinFunction,
    GlobalThis,
    UserFn(LocalId),
    DynFn(DynFunction),
}

#[derive(Debug, Clone)]
struct DynFunction {
    params: Vec<String>,
    body: String,
}

#[derive(Debug, Clone)]
struct UserFunction {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

struct Folder<'a> {
    by_id: HashMap<LocalId, &'a Local>,
    eval_id: Option<LocalId>,
    env: HashMap<LocalId, Value>,
    globals: HashMap<String, Value>,
    user_fns: HashMap<LocalId, UserFunction>,
}

fn try_fold(module: &Module) -> Result<Vec<Observation>, String> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let eval_id = module
        .locals
        .iter()
        .find(|l| l.name == "eval")
        .map(|l| l.id);
    let function_id = module
        .locals
        .iter()
        .find(|l| l.name == "Function")
        .map(|l| l.id);
    let global_this_id = module
        .locals
        .iter()
        .find(|l| l.name == "globalThis")
        .map(|l| l.id);

    let mut observe_ids = Vec::new();
    let mut seen = HashSet::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            if seen.insert(*local) {
                if let Some(loc) = by_id.get(local) {
                    if is_user_binding_name(&loc.name) {
                        observe_ids.push(*local);
                    }
                }
            }
        }
    }

    if observe_ids.is_empty() {
        return Err("no observation locals".into());
    }

    let mut folder = Folder {
        by_id,
        eval_id,
        env: HashMap::new(),
        globals: HashMap::new(),
        user_fns: HashMap::new(),
    };

    if let Some(id) = eval_id {
        folder.env.insert(id, Value::BuiltinEval);
    }
    if let Some(id) = function_id {
        folder.env.insert(id, Value::BuiltinFunction);
    }
    if let Some(id) = global_this_id {
        folder.env.insert(id, Value::GlobalThis);
    }
    if let Some(u) = module.locals.iter().find(|l| l.name == "undefined") {
        folder.env.insert(u.id, Value::Prim(EmbedValue::Undefined));
    }

    for stmt in &module.body {
        folder.exec_stmt(stmt)?;
    }

    let mut out = Vec::with_capacity(observe_ids.len());
    for id in &observe_ids {
        let v = folder
            .env
            .get(id)
            .cloned()
            .ok_or_else(|| format!("missing observation local {}", id.0))?;
        out.push(value_to_observation(v)?);
    }

    if !module_uses_eval_or_function(module, eval_id, function_id) {
        return Err("module does not use eval/Function".into());
    }
    Ok(out)
}

fn module_uses_eval_or_function(
    module: &Module,
    eval_id: Option<LocalId>,
    function_id: Option<LocalId>,
) -> bool {
    fn walk_stmt(s: &Stmt, eval_id: Option<LocalId>, function_id: Option<LocalId>) -> bool {
        match s {
            Stmt::Declare { init, .. } => init
                .as_ref()
                .is_some_and(|e| walk_expr(e, eval_id, function_id)),
            Stmt::Expr { expr } | Stmt::Return { value: Some(expr) } => {
                walk_expr(expr, eval_id, function_id)
            }
            Stmt::Return { value: None } => false,
            Stmt::Function { body, .. } | Stmt::Block { body } => {
                body.iter().any(|s| walk_stmt(s, eval_id, function_id))
            }
            _ => false,
        }
    }
    fn walk_expr(e: &Expr, eval_id: Option<LocalId>, function_id: Option<LocalId>) -> bool {
        match e {
            Expr::Local { id, .. } => Some(*id) == eval_id || Some(*id) == function_id,
            Expr::Unary { arg, .. } => walk_expr(arg, eval_id, function_id),
            Expr::Binary { left, right, .. } => {
                walk_expr(left, eval_id, function_id) || walk_expr(right, eval_id, function_id)
            }
            Expr::Member {
                object, property, ..
            } => {
                walk_expr(object, eval_id, function_id) || walk_expr(property, eval_id, function_id)
            }
            Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
                if walk_expr(callee, eval_id, function_id) {
                    return true;
                }
                args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => walk_expr(e, eval_id, function_id),
                })
            }
            Expr::Assign { target, value, .. } => {
                let t = match target {
                    AssignTarget::Member {
                        object, property, ..
                    } => {
                        walk_expr(object, eval_id, function_id)
                            || walk_expr(property, eval_id, function_id)
                    }
                    _ => false,
                };
                t || walk_expr(value, eval_id, function_id)
            }
            _ => false,
        }
    }
    module
        .body
        .iter()
        .any(|s| walk_stmt(s, eval_id, function_id))
}

fn value_to_observation(v: Value) -> Result<Observation, String> {
    match v {
        Value::Prim(EmbedValue::Number(n)) => Ok(Observation::Number(n)),
        Value::Prim(EmbedValue::String(s)) => Ok(Observation::String(s)),
        Value::Prim(EmbedValue::Boolean(b)) => Ok(Observation::Bool(b)),
        Value::Prim(EmbedValue::Undefined) => Ok(Observation::String("undefined".into())),
        Value::BuiltinEval | Value::BuiltinFunction | Value::UserFn(_) | Value::DynFn(_) => {
            Ok(Observation::Function)
        }
        other => Err(format!("cannot observe value {other:?}")),
    }
}

impl<'a> Folder<'a> {
    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Option<Value>, String> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let v = match init {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Prim(EmbedValue::Undefined),
                };
                self.env.insert(*local, v);
                Ok(None)
            }
            Stmt::Expr { expr } => {
                let _ = self.eval_expr(expr)?;
                Ok(None)
            }
            Stmt::Block { body } => {
                let mut ret = None;
                for s in body {
                    if let Some(v) = self.exec_stmt(s)? {
                        ret = Some(v);
                        break;
                    }
                }
                Ok(ret)
            }
            Stmt::Function {
                local,
                params,
                body,
                is_async,
                is_generator,
            } => {
                if *is_async || *is_generator {
                    return Err("async/generator functions not supported in eval fold".into());
                }
                let mut param_ids = Vec::new();
                for p in params {
                    let Pattern::Local(id) = &p.pattern else {
                        return Err("only simple ident params in eval fold".into());
                    };
                    param_ids.push(*id);
                }
                self.user_fns.insert(
                    *local,
                    UserFunction {
                        params: param_ids,
                        body: body.clone(),
                    },
                );
                self.env.insert(*local, Value::UserFn(*local));
                Ok(None)
            }
            Stmt::Return { value } => {
                let v = match value {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Prim(EmbedValue::Undefined),
                };
                Ok(Some(v))
            }
            other => Err(format!("unsupported statement in eval fold: {other:?}")),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Number { raw, .. } => Ok(Value::Prim(EmbedValue::Number(parse_number(raw)?))),
            Expr::String { value, .. } => {
                Ok(Value::Prim(EmbedValue::String(value.to_string_lossy())))
            }
            Expr::Boolean { value, .. } => Ok(Value::Prim(EmbedValue::Boolean(*value))),
            Expr::Null { .. } => Ok(Value::Prim(EmbedValue::Null)),
            Expr::Local { id, .. } => self
                .env
                .get(id)
                .cloned()
                .ok_or_else(|| format!("unbound local %{}", id.0)),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => {
                let v = self.eval_expr(arg)?;
                Ok(Value::Prim(EmbedValue::String(typeof_name(&v).into())))
            }
            Expr::Binary {
                left, op, right, ..
            } => self.eval_binary(left, *op, right),
            Expr::Member {
                object,
                property,
                computed,
                optional,
                ..
            } => {
                if *optional || *computed {
                    return Err("optional/computed member not supported in eval fold".into());
                }
                let obj = self.eval_expr(object)?;
                let Expr::String { value, .. } = property.as_ref() else {
                    return Err("member property must be string".into());
                };
                let name = value.to_string_lossy();
                match obj {
                    Value::GlobalThis => {
                        if name == "eval" {
                            return Ok(Value::BuiltinEval);
                        }
                        if name == "Function" {
                            return Ok(Value::BuiltinFunction);
                        }
                        self.globals
                            .get(&name)
                            .cloned()
                            .ok_or_else(|| format!("missing globalThis.{name}"))
                    }
                    _ => Err("unsupported member object in eval fold".into()),
                }
            }
            Expr::Assign {
                target,
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let v = self.eval_expr(value)?;
                match target {
                    AssignTarget::Local(id) => {
                        self.env.insert(*id, v.clone());
                        Ok(v)
                    }
                    AssignTarget::Member {
                        object,
                        property,
                        computed,
                    } if !computed => {
                        let obj = self.eval_expr(object)?;
                        let Expr::String { value: prop, .. } = property.as_ref() else {
                            return Err("assign member property must be string".into());
                        };
                        let name = prop.to_string_lossy();
                        if !matches!(obj, Value::GlobalThis) {
                            return Err("only globalThis property assign supported".into());
                        }
                        self.globals.insert(name, v.clone());
                        Ok(v)
                    }
                    _ => Err("unsupported assign target in eval fold".into()),
                }
            }
            Expr::New { callee, args, .. } => self.eval_call_or_new(callee, args, true),
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err("optional call not supported".into());
                }
                self.eval_call_or_new(callee, args, false)
            }
            other => Err(format!("unsupported expr in eval fold: {other:?}")),
        }
    }

    fn eval_binary(&mut self, left: &Expr, op: BinaryOp, right: &Expr) -> Result<Value, String> {
        if op == BinaryOp::Comma {
            let _ = self.eval_expr(left)?;
            return self.eval_expr(right);
        }
        let l = self.eval_expr(left)?;
        let r = self.eval_expr(right)?;
        match op {
            BinaryOp::EqEqEq | BinaryOp::EqEq => {
                Ok(Value::Prim(EmbedValue::Boolean(values_eq(&l, &r))))
            }
            BinaryOp::NotEqEq | BinaryOp::NotEq => {
                Ok(Value::Prim(EmbedValue::Boolean(!values_eq(&l, &r))))
            }
            _ => Err(format!("unsupported binary op in eval fold: {op:?}")),
        }
    }

    fn eval_call_or_new(
        &mut self,
        callee: &Expr,
        args: &[Arg],
        is_new: bool,
    ) -> Result<Value, String> {
        let direct_eval = matches!(callee, Expr::Local { id, .. } if Some(*id) == self.eval_id);
        let callee_v = self.eval_expr(callee)?;

        match callee_v {
            Value::BuiltinEval => {
                if is_new {
                    return Err("new eval not supported".into());
                }
                self.call_eval(args, direct_eval)
            }
            Value::BuiltinFunction => self.call_function_ctor(args),
            Value::UserFn(id) => {
                if is_new {
                    return Err("new user fn not supported in eval fold".into());
                }
                self.call_user_fn(id, args)
            }
            Value::DynFn(df) => {
                if is_new {
                    return Err("new dyn fn not supported".into());
                }
                self.call_dyn_fn(&df, args)
            }
            other => Err(format!("not callable: {other:?}")),
        }
    }

    fn call_eval(&mut self, args: &[Arg], direct: bool) -> Result<Value, String> {
        if args.len() != 1 {
            return Err("eval expects 1 argument".into());
        }
        let Arg::Expr(arg) = &args[0] else {
            return Err("spread not supported in eval".into());
        };
        let Expr::String { value, .. } = arg else {
            return Err("only constant-string eval supported".into());
        };
        let src = value.to_string_lossy();
        let bindings = if direct {
            self.direct_eval_bindings()
        } else {
            self.indirect_eval_bindings()
        };
        let result = eval_source_with_bindings(&src, &bindings)
            .map_err(|e| format!("embed eval failed for {src:?}: {e}"))?;
        Ok(Value::Prim(result))
    }

    fn direct_eval_bindings(&self) -> Vec<(String, EmbedValue)> {
        let mut map: HashMap<String, EmbedValue> = HashMap::new();
        for (k, v) in &self.globals {
            if let Value::Prim(ev) = v {
                map.insert(k.clone(), ev.clone());
            }
        }
        for (id, v) in &self.env {
            let Some(loc) = self.by_id.get(id) else {
                continue;
            };
            if let Value::Prim(ev) = v {
                if is_user_binding_name(&loc.name) {
                    map.insert(loc.name.clone(), ev.clone());
                }
            }
        }
        map.into_iter().collect()
    }

    fn indirect_eval_bindings(&self) -> Vec<(String, EmbedValue)> {
        let mut out = Vec::new();
        for (k, v) in &self.globals {
            if let Value::Prim(ev) = v {
                out.push((k.clone(), ev.clone()));
            }
        }
        out
    }

    fn call_function_ctor(&mut self, args: &[Arg]) -> Result<Value, String> {
        let df = parse_function_ctor_args(args)?;
        Ok(Value::DynFn(df))
    }

    fn call_dyn_fn(&mut self, df: &DynFunction, args: &[Arg]) -> Result<Value, String> {
        let arg_vals = const_arg_values(args)?;
        let param_refs: Vec<&str> = df.params.iter().map(|s| s.as_str()).collect();
        let result = eval_function_call(&param_refs, &df.body, &arg_vals)
            .map_err(|e| format!("embed Function call failed: {e}"))?;
        Ok(Value::Prim(result))
    }

    fn call_user_fn(&mut self, id: LocalId, args: &[Arg]) -> Result<Value, String> {
        let uf = self
            .user_fns
            .get(&id)
            .cloned()
            .ok_or_else(|| format!("missing user fn {}", id.0))?;
        if !args.is_empty() {
            return Err("user fn args not supported in eval fold subset".into());
        }
        let mut shadowed = Vec::new();
        let mut body_locals = HashSet::new();
        collect_decl_locals(&uf.body, &mut body_locals);
        for lid in &body_locals {
            shadowed.push((*lid, self.env.get(lid).cloned()));
        }
        for pid in &uf.params {
            shadowed.push((*pid, self.env.get(pid).cloned()));
            self.env.insert(*pid, Value::Prim(EmbedValue::Undefined));
        }

        let mut ret = Value::Prim(EmbedValue::Undefined);
        for stmt in &uf.body {
            if let Some(v) = self.exec_stmt(stmt)? {
                ret = v;
                break;
            }
        }

        for (lid, prev) in shadowed {
            match prev {
                Some(v) => {
                    self.env.insert(lid, v);
                }
                None => {
                    self.env.remove(&lid);
                }
            }
        }
        Ok(ret)
    }
}

fn collect_decl_locals(body: &[Stmt], out: &mut HashSet<LocalId>) {
    for s in body {
        match s {
            Stmt::Declare { local, .. } => {
                out.insert(*local);
            }
            Stmt::Block { body } => collect_decl_locals(body, out),
            _ => {}
        }
    }
}

fn is_user_binding_name(name: &str) -> bool {
    !matches!(
        name,
        "Math"
            | "Number"
            | "NaN"
            | "Infinity"
            | "Symbol"
            | "Promise"
            | "Proxy"
            | "Reflect"
            | "undefined"
            | "globalThis"
            | "Object"
            | "Function"
            | "Array"
            | "String"
            | "Boolean"
            | "Error"
            | "TypeError"
            | "RangeError"
            | "ReferenceError"
            | "SyntaxError"
            | "URIError"
            | "EvalError"
            | "AggregateError"
            | "parseInt"
            | "parseFloat"
            | "isNaN"
            | "isFinite"
            | "encodeURI"
            | "decodeURI"
            | "encodeURIComponent"
            | "decodeURIComponent"
            | "JSON"
            | "Date"
            | "RegExp"
            | "Map"
            | "Set"
            | "WeakMap"
            | "WeakSet"
            | "ArrayBuffer"
            | "DataView"
            | "Int8Array"
            | "Uint8Array"
            | "Uint8ClampedArray"
            | "Int16Array"
            | "Uint16Array"
            | "Int32Array"
            | "Uint32Array"
            | "Float32Array"
            | "Float64Array"
            | "BigInt64Array"
            | "BigUint64Array"
            | "TextEncoder"
            | "TextDecoder"
            | "eval"
            | "escape"
            | "unescape"
            | "arguments"
    )
}

fn typeof_name(v: &Value) -> &'static str {
    match v {
        Value::Prim(p) => p.typeof_name(),
        Value::BuiltinEval | Value::BuiltinFunction | Value::UserFn(_) | Value::DynFn(_) => {
            "function"
        }
        Value::GlobalThis => "object",
    }
}

fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::BuiltinEval, Value::BuiltinEval) => true,
        (Value::BuiltinFunction, Value::BuiltinFunction) => true,
        (Value::Prim(EmbedValue::Boolean(x)), Value::Prim(EmbedValue::Boolean(y))) => x == y,
        (Value::Prim(EmbedValue::Number(x)), Value::Prim(EmbedValue::Number(y))) => x == y,
        (Value::Prim(EmbedValue::String(x)), Value::Prim(EmbedValue::String(y))) => x == y,
        (Value::Prim(EmbedValue::Null), Value::Prim(EmbedValue::Null)) => true,
        (Value::Prim(EmbedValue::Undefined), Value::Prim(EmbedValue::Undefined)) => true,
        _ => false,
    }
}

fn parse_function_ctor_args(args: &[Arg]) -> Result<DynFunction, String> {
    if args.is_empty() {
        return Ok(DynFunction {
            params: vec![],
            body: String::new(),
        });
    }
    let mut strs = Vec::with_capacity(args.len());
    for a in args {
        let Arg::Expr(Expr::String { value, .. }) = a else {
            return Err("Function(...) requires constant string arguments".into());
        };
        strs.push(value.to_string_lossy());
    }
    let body = strs.pop().unwrap_or_default();
    Ok(DynFunction { params: strs, body })
}

fn const_arg_values(args: &[Arg]) -> Result<Vec<EmbedValue>, String> {
    let mut out = Vec::with_capacity(args.len());
    for a in args {
        let Arg::Expr(e) = a else {
            return Err("spread args not supported".into());
        };
        out.push(match e {
            Expr::Number { raw, .. } => EmbedValue::Number(parse_number(raw)?),
            Expr::String { value, .. } => EmbedValue::String(value.to_string_lossy()),
            Expr::Boolean { value, .. } => EmbedValue::Boolean(*value),
            Expr::Null { .. } => EmbedValue::Null,
            _ => return Err("Function call args must be constant literals".into()),
        });
    }
    Ok(out)
}

fn parse_number(raw: &str) -> Result<f64, String> {
    let s = raw.replace('_', "");
    s.parse::<f64>()
        .map_err(|_| format!("bad number literal: {raw}"))
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn module_of(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn fold_direct_eval_observations() {
        let obs = fold_eval_program(&module_of(
            r#"
            let t = typeof eval;
            let g = globalThis.eval === eval;
            let a = eval("1 + 2");
            let b = eval("typeof undefined");
            let c = eval("3 * 4");
            let d = eval("'hi'");
            "#,
        ))
        .expect("fold");
        assert_eq!(
            obs,
            vec![
                Observation::String("function".into()),
                Observation::Bool(true),
                Observation::Number(3.0),
                Observation::String("undefined".into()),
                Observation::Number(12.0),
                Observation::String("hi".into()),
            ]
        );
    }

    #[test]
    fn fold_new_function_observations() {
        let obs = fold_eval_program(&module_of(
            r#"
            let tf = typeof Function;
            let same = globalThis.Function === Function;
            let f = new Function("a", "b", "return a + b");
            let g = Function("x", "return x * 2");
            let h = new Function("return 7");
            let r1 = f(1, 2);
            let r2 = g(3);
            let r3 = h();
            let t1 = typeof f;
            let t2 = typeof g;
            "#,
        ))
        .expect("fold");
        assert_eq!(
            obs,
            vec![
                Observation::String("function".into()),
                Observation::Bool(true),
                Observation::Function,
                Observation::Function,
                Observation::Function,
                Observation::Number(3.0),
                Observation::Number(6.0),
                Observation::Number(7.0),
                Observation::String("function".into()),
                Observation::String("function".into()),
            ]
        );
    }

    #[test]
    fn fold_indirect_eval_observations() {
        let obs = fold_eval_program(&module_of(
            r#"
            globalThis.gx = 100;
            function probeDirect() {
              let gx = 200;
              return eval("gx");
            }
            function probeIndirectComma() {
              let gx = 200;
              return (0, eval)("gx");
            }
            function probeIndirectGlobalThis() {
              let gx = 200;
              return globalThis.eval("gx");
            }
            let d = probeDirect();
            let i = probeIndirectComma();
            let g = probeIndirectGlobalThis();
            let t = typeof (0, eval);
            let same = globalThis.eval === eval;
            let a = (0, eval)("1 + 2");
            let b = globalThis.eval("'hi'");
            "#,
        ))
        .expect("fold");
        assert_eq!(
            obs,
            vec![
                Observation::Number(200.0),
                Observation::Number(100.0),
                Observation::Number(100.0),
                Observation::String("function".into()),
                Observation::Bool(true),
                Observation::Number(3.0),
                Observation::String("hi".into()),
            ]
        );
    }
}
