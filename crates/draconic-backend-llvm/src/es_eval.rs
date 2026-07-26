//! N07.02–N07.04: lower `eval` / `Function` / indirect eval via Embed (fold at emit).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_embed::{eval_function_call, eval_source_with_bindings, EmbedValue};
use draconic_ir::{Arg, AssignTarget, Expr, Local, LocalId, Module, Pattern, Stmt};

/// True when this module is the supported eval/Function subset (E16 / N07.02–N07.04).
pub(crate) fn is_es_eval_module(module: &Module) -> bool {
    match try_fold(module) {
        Ok(obs) => !obs.is_empty(),
        Err(_) => false,
    }
}

pub(crate) fn emit_es_eval(module: &Module) -> Result<String, Diagnostic> {
    let obs = try_fold(module).map_err(diag)?;
    if obs.is_empty() {
        return Err(diag("internal: not an eval/Function module"));
    }
    emit_observations(&obs, classify_tag(module, &obs))
}

fn classify_tag(module: &Module, _obs: &[Observation]) -> &'static str {
    if module_has_indirect(module) {
        "N07.04 indirect eval via Embed"
    } else if module_has_function_ctor(module) && module_has_direct_eval(module) {
        "N07.02/N07.03 eval+Function via Embed"
    } else if module_has_function_ctor(module) {
        "N07.03 Function via Embed"
    } else {
        "N07.02 direct eval via Embed"
    }
}

fn module_has_direct_eval(module: &Module) -> bool {
    let eval_id = module.locals.iter().find(|l| l.name == "eval").map(|l| l.id);
    fn walk(stmt: &Stmt, eval_id: Option<LocalId>) -> bool {
        match stmt {
            Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Return { value: Some(e) } => {
                expr_has_direct_eval(e, eval_id)
            }
            Stmt::Function { body, .. } | Stmt::Block { body } => body.iter().any(|s| walk(s, eval_id)),
            _ => false,
        }
    }
    fn expr_has_direct_eval(e: &Expr, eval_id: Option<LocalId>) -> bool {
        match e {
            Expr::Call { callee, .. } => {
                if let Expr::Local { id, .. } = callee.as_ref() {
                    if Some(*id) == eval_id {
                        return true;
                    }
                }
                false
            }
            Expr::Unary { arg, .. } => expr_has_direct_eval(arg, eval_id),
            Expr::Binary { left, right, .. } => {
                expr_has_direct_eval(left, eval_id) || expr_has_direct_eval(right, eval_id)
            }
            _ => false,
        }
    }
    module.body.iter().any(|s| walk(s, eval_id))
}

fn module_has_function_ctor(module: &Module) -> bool {
    let function_id = module
        .locals
        .iter()
        .find(|l| l.name == "Function")
        .map(|l| l.id);
    fn walk(stmt: &Stmt, function_id: Option<LocalId>) -> bool {
        match stmt {
            Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } => {
                expr_has_fn_ctor(e, function_id)
            }
            Stmt::Block { body } | Stmt::Function { body, .. } => {
                body.iter().any(|s| walk(s, function_id))
            }
            _ => false,
        }
    }
    fn expr_has_fn_ctor(e: &Expr, function_id: Option<LocalId>) -> bool {
        match e {
            Expr::New { callee, .. } | Expr::Call { callee, .. } => {
                matches!(callee.as_ref(), Expr::Local { id, .. } if Some(*id) == function_id)
            }
            _ => false,
        }
    }
    module.body.iter().any(|s| walk(s, function_id))
}

fn module_has_indirect(module: &Module) -> bool {
    fn walk(stmt: &Stmt) -> bool {
        match stmt {
            Stmt::Declare { init: Some(e), .. }
            | Stmt::Expr { expr: e }
            | Stmt::Return { value: Some(e) } => expr_indirect(e),
            Stmt::Function { body, .. } | Stmt::Block { body } => body.iter().any(walk),
            _ => false,
        }
    }
    fn expr_indirect(e: &Expr) -> bool {
        match e {
            Expr::Call { callee, .. } => match callee.as_ref() {
                Expr::Binary {
                    op: BinaryOp::Comma,
                    ..
                } => true,
                Expr::Member {
                    object,
                    property,
                    computed,
                    optional,
                    ..
                } if !*computed && !*optional => {
                    if let (Expr::Local { .. }, Expr::String { value, .. }) =
                        (object.as_ref(), property.as_ref())
                    {
                        value.to_string_lossy() == "eval"
                    } else {
                        false
                    }
                }
                _ => false,
            },
            Expr::Unary { arg, .. } => expr_indirect(arg),
            Expr::Binary { left, right, .. } => expr_indirect(left) || expr_indirect(right),
            _ => false,
        }
    }
    module.body.iter().any(walk)
}

#[derive(Debug, Clone)]
enum FoldValue {
    Number(f64),
    String(String),
    Bool(bool),
    Func,
}

struct Observation {
    value: FoldValue,
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

#[derive(Debug, Clone)]
enum Value {
    Undefined,
    Null,
    Bool(bool),
    Number(f64),
    String(String),
    BuiltinEval,
    BuiltinFunction,
    GlobalThis,
    UserFn(LocalId),
    DynFn(DynFunction),
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
    let eval_id = module.locals.iter().find(|l| l.name == "eval").map(|l| l.id);
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
                // Skip builtin-only noise; only user-facing top-level lets.
                if let Some(loc) = by_id.get(local) {
                    if is_observation_local(loc) {
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

    // Seed builtins.
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
        folder.env.insert(u.id, Value::Undefined);
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

    // Must actually use eval or Function.
    let uses = folder_uses_eval_or_function(module, eval_id, function_id);
    if !uses {
        return Err("module does not use eval/Function".into());
    }
    Ok(out)
}

fn is_observation_local(loc: &Local) -> bool {
    is_user_binding_name(&loc.name)
}

fn folder_uses_eval_or_function(
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
            } => walk_expr(object, eval_id, function_id) || walk_expr(property, eval_id, function_id),
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
                    AssignTarget::Member { object, property, .. } => {
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
        Value::Number(n) => Ok(Observation {
            value: FoldValue::Number(n),
        }),
        Value::String(s) => Ok(Observation {
            value: FoldValue::String(s),
        }),
        Value::Bool(b) => Ok(Observation {
            value: FoldValue::Bool(b),
        }),
        Value::BuiltinEval
        | Value::BuiltinFunction
        | Value::UserFn(_)
        | Value::DynFn(_) => Ok(Observation {
            value: FoldValue::Func,
        }),
        Value::Undefined => Ok(Observation {
            value: FoldValue::String("undefined".into()),
        }),
        other => Err(format!("cannot observe value {other:?}")),
    }
}

impl<'a> Folder<'a> {
    fn exec_stmt(&mut self, stmt: &Stmt) -> Result<Option<Value>, String> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let v = match init {
                    Some(e) => self.eval_expr(e)?,
                    None => Value::Undefined,
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
                    None => Value::Undefined,
                };
                Ok(Some(v))
            }
            other => Err(format!("unsupported statement in eval fold: {other:?}")),
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Value, String> {
        match expr {
            Expr::Number { raw, .. } => Ok(Value::Number(parse_number(raw)?)),
            Expr::String { value, .. } => Ok(Value::String(value.to_string_lossy())),
            Expr::Boolean { value, .. } => Ok(Value::Bool(*value)),
            Expr::Null { .. } => Ok(Value::Null),
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
                Ok(Value::String(typeof_name(&v).into()))
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
            BinaryOp::EqEqEq | BinaryOp::EqEq => Ok(Value::Bool(values_eq(&l, &r))),
            BinaryOp::NotEqEq | BinaryOp::NotEq => Ok(Value::Bool(!values_eq(&l, &r))),
            _ => Err(format!("unsupported binary op in eval fold: {op:?}")),
        }
    }

    fn eval_call_or_new(
        &mut self,
        callee: &Expr,
        args: &[Arg],
        is_new: bool,
    ) -> Result<Value, String> {
        // Direct eval only when callee is the bare `eval` local (ECMA-262).
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
        Ok(embed_to_value(result))
    }

    fn direct_eval_bindings(&self) -> Vec<(String, EmbedValue)> {
        // Lexical env (function locals) shadow globals.
        let mut map: HashMap<String, EmbedValue> = HashMap::new();
        for (k, v) in &self.globals {
            if let Some(ev) = value_to_embed(v) {
                map.insert(k.clone(), ev);
            }
        }
        for (id, v) in &self.env {
            let Some(loc) = self.by_id.get(id) else {
                continue;
            };
            if let Some(ev) = value_to_embed(v) {
                if is_user_binding_name(&loc.name) {
                    map.insert(loc.name.clone(), ev);
                }
            }
        }
        map.into_iter().collect()
    }

    fn indirect_eval_bindings(&self) -> Vec<(String, EmbedValue)> {
        let mut out = Vec::new();
        for (k, v) in &self.globals {
            if let Some(ev) = value_to_embed(v) {
                out.push((k.clone(), ev));
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
        Ok(embed_to_value(result))
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
        // Save and restore env entries for params / function-local declares.
        let saved: HashMap<LocalId, Option<Value>> = HashMap::new();
        let _ = saved;
        let mut shadowed = Vec::new();
        // Collect locals that will be written inside the function body.
        let mut body_locals = HashSet::new();
        collect_decl_locals(&uf.body, &mut body_locals);
        for lid in &body_locals {
            shadowed.push((*lid, self.env.get(lid).cloned()));
        }
        for pid in &uf.params {
            shadowed.push((*pid, self.env.get(pid).cloned()));
            self.env.insert(*pid, Value::Undefined);
        }

        let mut ret = Value::Undefined;
        for stmt in &uf.body {
            if let Some(v) = self.exec_stmt(stmt)? {
                ret = v;
                break;
            }
        }

        // Restore.
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
            | "eval"
            | "escape"
            | "unescape"
            | "arguments"
    )
}

fn value_to_embed(v: &Value) -> Option<EmbedValue> {
    match v {
        Value::Undefined => Some(EmbedValue::Undefined),
        Value::Null => Some(EmbedValue::Null),
        Value::Bool(b) => Some(EmbedValue::Boolean(*b)),
        Value::Number(n) => Some(EmbedValue::Number(*n)),
        Value::String(s) => Some(EmbedValue::String(s.clone())),
        _ => None,
    }
}

fn embed_to_value(v: EmbedValue) -> Value {
    match v {
        EmbedValue::Undefined => Value::Undefined,
        EmbedValue::Null => Value::Null,
        EmbedValue::Boolean(b) => Value::Bool(b),
        EmbedValue::Number(n) => Value::Number(n),
        EmbedValue::String(s) => Value::String(s),
    }
}

fn typeof_name(v: &Value) -> &'static str {
    match v {
        Value::Undefined => "undefined",
        Value::Null => "object",
        Value::Bool(_) => "boolean",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::BuiltinEval
        | Value::BuiltinFunction
        | Value::UserFn(_)
        | Value::DynFn(_) => "function",
        Value::GlobalThis => "object",
    }
}

fn values_eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::BuiltinEval, Value::BuiltinEval) => true,
        (Value::BuiltinFunction, Value::BuiltinFunction) => true,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Number(x), Value::Number(y)) => x == y,
        (Value::String(x), Value::String(y)) => x == y,
        (Value::Null, Value::Null) => true,
        (Value::Undefined, Value::Undefined) => true,
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
    Ok(DynFunction {
        params: strs,
        body,
    })
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

fn emit_observations(obs: &[Observation], tag: &str) -> Result<String, Diagnostic> {
    let mut out = String::new();
    let mut body = String::new();
    let mut str_globals: HashMap<String, String> = HashMap::new();
    let mut tmp = 0u32;

    writeln!(out, "; Draconic LLVM backend ({tag})").ok();
    writeln!(out, "declare void @draconic_rt_gc_init()").ok();
    writeln!(out, "declare void @draconic_rt_print_i64(i64)").ok();
    writeln!(out, "declare void @draconic_rt_print_bool(i8)").ok();
    writeln!(out, "declare void @draconic_rt_print_str(ptr)").ok();
    writeln!(out).ok();

    for o in obs {
        match &o.value {
            FoldValue::Number(n) => {
                if n.fract() != 0.0 || !n.is_finite() || n.abs() >= (i64::MAX as f64) {
                    return Err(diag(format!("number not representable as i64: {n}")));
                }
                let v = *n as i64;
                writeln!(body, "  call void @draconic_rt_print_i64(i64 {v})").ok();
            }
            FoldValue::Bool(b) => {
                let v = if *b { 1 } else { 0 };
                writeln!(body, "  call void @draconic_rt_print_bool(i8 {v})").ok();
            }
            FoldValue::String(s) => {
                let gname = if let Some(g) = str_globals.get(s) {
                    g.clone()
                } else {
                    let g = format!(".str.{}", str_globals.len());
                    str_globals.insert(s.clone(), g.clone());
                    g
                };
                let t = format!("%t{tmp}");
                tmp += 1;
                let n = s.len() + 1;
                writeln!(
                    body,
                    "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
                )
                .ok();
                writeln!(body, "  call void @draconic_rt_print_str(ptr {t})").ok();
            }
            FoldValue::Func => {
                let s = "function";
                let gname = if let Some(g) = str_globals.get(s) {
                    g.clone()
                } else {
                    let g = format!(".str.{}", str_globals.len());
                    str_globals.insert(s.to_string(), g.clone());
                    g
                };
                let t = format!("%t{tmp}");
                tmp += 1;
                let n = s.len() + 1;
                writeln!(
                    body,
                    "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
                )
                .ok();
                writeln!(body, "  call void @draconic_rt_print_str(ptr {t})").ok();
            }
        }
    }

    for (content, gname) in &str_globals {
        let n = content.len() + 1;
        let esc = escape_llvm_string(content);
        writeln!(
            out,
            "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
        )
        .ok();
    }
    if !str_globals.is_empty() {
        writeln!(out).ok();
    }

    writeln!(out, "define i32 @main() {{").ok();
    writeln!(out, "entry:").ok();
    writeln!(out, "  call void @draconic_rt_gc_init()").ok();
    out.push_str(&body);
    writeln!(out, "  ret i32 0").ok();
    writeln!(out, "}}").ok();
    Ok(out)
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) && c != b'\\' => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
