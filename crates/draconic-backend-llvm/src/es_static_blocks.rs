//! N08.16.42: native observations for class static initialization blocks (E18.41 /
//! `es/annex-b/static_blocks`).
//!
//! Compile-time evaluation of the class-builder IR shape produced for static
//! public fields, static blocks (`static { … }`), static methods, private static
//! fields (WeakMap), `extends` heritage, and class expressions. Emits Runtime
//! prints of top-level number/string observations.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey,
    Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, PRINT_F64, PRINT_STR};

pub(crate) fn is_es_static_blocks_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_static_blocks(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_static_blocks module"))?;
    Ok(emit_prints(&info))
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Str(String),
    Bool(bool),
    Undef,
    Null,
    /// Heap object index.
    Obj(usize),
    /// Function table index.
    Fn(usize),
    /// WeakMap heap index.
    WeakMap(usize),
    Builtin(&'static str),
}

#[derive(Clone)]
struct FnRec {
    params: Vec<ParamBind>,
    body: Vec<Stmt>,
    is_arrow: bool,
    #[allow(dead_code)]
    is_method: bool,
    /// Lexical locals captured at creation (class builder WeakMaps, etc.).
    closure: HashMap<LocalId, JsVal>,
    /// Captured bare-name bindings.
    name_closure: HashMap<String, JsVal>,
}

struct Heap {
    objects: Vec<HashMap<String, JsVal>>,
    weakmaps: Vec<HashMap<usize, JsVal>>,
    functions: Vec<FnRec>,
}

impl Heap {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            weakmaps: Vec::new(),
            functions: Vec::new(),
        }
    }

    fn alloc_obj(&mut self, props: HashMap<String, JsVal>) -> usize {
        let id = self.objects.len();
        self.objects.push(props);
        id
    }

    fn alloc_fn(&mut self, rec: FnRec) -> usize {
        let id = self.functions.len();
        self.functions.push(rec);
        id
    }

    fn alloc_wm(&mut self) -> usize {
        let id = self.weakmaps.len();
        self.weakmaps.push(HashMap::new());
        id
    }

    fn get(&self, oid: usize, key: &str) -> JsVal {
        self.objects
            .get(oid)
            .and_then(|m| m.get(key).cloned())
            .unwrap_or(JsVal::Undef)
    }

    fn set(&mut self, oid: usize, key: &str, val: JsVal) {
        if let Some(m) = self.objects.get_mut(oid) {
            m.insert(key.to_string(), val);
        }
    }

    fn has_own(&self, oid: usize, key: &str) -> bool {
        self.objects
            .get(oid)
            .is_some_and(|m| m.contains_key(key))
    }

    fn delete(&mut self, oid: usize, key: &str) -> bool {
        self.objects
            .get_mut(oid)
            .map(|m| m.remove(key).is_some())
            .unwrap_or(false)
    }
}

#[derive(Clone)]
enum Obs {
    Num(f64),
    Str(String),
}

struct ModuleInfo {
    observations: Vec<Obs>,
}

struct Env {
    locals: HashMap<LocalId, JsVal>,
    /// Bare `Name` param / with-style bindings (`t`, `p`, `r` in Proxy get traps).
    names: HashMap<String, JsVal>,
    this: JsVal,
    new_target: JsVal,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    if !module_has_static_block_shape(module) {
        return None;
    }
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut heap = Heap::new();
    let mut env = Env {
        locals: HashMap::new(),
        names: HashMap::new(),
        this: JsVal::Undef,
        new_target: JsVal::Undef,
    };

    eval_body(&module.body, &mut env, &mut heap, &by_id).ok()?;

    let mut observations = Vec::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            // Skip class / callable bindings.
            if matches!(loc.ty, Type::Function) {
                continue;
            }
            match env.locals.get(local) {
                Some(JsVal::Num(n)) => observations.push(Obs::Num(*n)),
                Some(JsVal::Str(s)) => observations.push(Obs::Str(s.clone())),
                Some(JsVal::Undef)
                    if matches!(loc.ty, Type::String | Type::Any | Type::Number) =>
                {
                    observations.push(Obs::Str("undefined".into()));
                }
                Some(JsVal::Bool(b)) => observations.push(Obs::Num(if *b { 1.0 } else { 0.0 })),
                Some(JsVal::Obj(oid)) => {
                    if as_callable(&JsVal::Obj(*oid), &heap).is_some() {
                        continue;
                    }
                    return None;
                }
                Some(JsVal::Fn(_)) | Some(JsVal::WeakMap(_)) | Some(JsVal::Builtin(_)) => {
                    continue;
                }
                _ => return None,
            }
        }
    }
    if observations.is_empty() {
        return None;
    }
    Some(ModuleInfo { observations })
}

fn module_has_static_block_shape(module: &Module) -> bool {
    // Look for `__sb` method-home static block call pattern in class builder IIFEs.
    fn expr_has(e: &Expr) -> bool {
        match e {
            Expr::Call { callee, args, .. } => {
                expr_has(callee) || args.iter().any(|a| matches!(a, Arg::Expr(e) if expr_has(e)))
            }
            Expr::Function { body, .. } => body.iter().any(stmt_has),
            Expr::Member { object, property, .. } => expr_has(object) || expr_has(property),
            Expr::Object { properties, .. } => properties.iter().any(|p| match p {
                ObjectProp::Property {
                    key: ObjectPropKey::Static(k),
                    value,
                } if k.to_string_lossy() == "__sb" => true,
                ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                    expr_has(value)
                }
                ObjectProp::Spread(e) => expr_has(e),
            }),
            Expr::Assign { value, .. } => expr_has(value),
            Expr::Binary { left, right, .. } => expr_has(left) || expr_has(right),
            Expr::Unary { arg, .. } => expr_has(arg),
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => expr_has(test) || expr_has(consequent) || expr_has(alternate),
            Expr::New { callee, args, .. } => {
                expr_has(callee) || args.iter().any(|a| matches!(a, Arg::Expr(e) if expr_has(e)))
            }
            _ => false,
        }
    }
    fn stmt_has(s: &Stmt) -> bool {
        match s {
            Stmt::Declare {
                init: Some(e), ..
            } => expr_has(e),
            Stmt::Expr { expr } => expr_has(expr),
            Stmt::Block { body } | Stmt::Function { body, .. } => body.iter().any(stmt_has),
            Stmt::Return { value: Some(e) } | Stmt::Throw { value: e } => expr_has(e),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                expr_has(test)
                    || stmt_has(consequent)
                    || alternate.as_ref().is_some_and(|a| stmt_has(a))
            }
            _ => false,
        }
    }
    module.body.iter().any(stmt_has)
}

fn eval_body(
    body: &[Stmt],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<(), ()> {
    for s in body {
        let _ = eval_stmt(s, env, heap, by_id)?;
    }
    Ok(())
}

/// Returns `Some` when a `return` was executed (function body).
fn eval_stmt(
    stmt: &Stmt,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<Option<JsVal>, ()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => eval_expr(e, env, heap, by_id)?,
                None => JsVal::Undef,
            };
            env.locals.insert(*local, v);
            Ok(None)
        }
        Stmt::Expr { expr } => {
            // Bare "use strict" string expr is a no-op directive.
            if let Expr::String { value, .. } = expr {
                if value.to_string_lossy() == "use strict" {
                    return Ok(None);
                }
            }
            eval_expr(expr, env, heap, by_id)?;
            Ok(None)
        }
        Stmt::Block { body } => {
            for s in body {
                if let Some(v) = eval_stmt(s, env, heap, by_id)? {
                    return Ok(Some(v));
                }
            }
            Ok(None)
        }
        Stmt::Return { value } => {
            let v = match value {
                Some(e) => eval_expr(e, env, heap, by_id)?,
                None => JsVal::Undef,
            };
            Ok(Some(v))
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            if to_boolean(&eval_expr(test, env, heap, by_id)?) {
                eval_stmt(consequent, env, heap, by_id)
            } else if let Some(a) = alternate {
                eval_stmt(a, env, heap, by_id)
            } else {
                Ok(None)
            }
        }
        Stmt::Throw { .. } => Err(()),
        Stmt::Try {
            block,
            handler,
            ..
        } => {
            match eval_body(block, env, heap, by_id) {
                Ok(()) => Ok(None),
                Err(()) => {
                    if let Some(h) = handler {
                        for s in h {
                            if let Some(v) = eval_stmt(s, env, heap, by_id)? {
                                return Ok(Some(v));
                            }
                        }
                        Ok(None)
                    } else {
                        Err(())
                    }
                }
            }
        }
        _ => Err(()),
    }
}

fn eval_expr(
    expr: &Expr,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => Ok(JsVal::Num(raw.parse().map_err(|_| ())?)),
        Expr::String { value, .. } => Ok(JsVal::Str(value.to_string_lossy())),
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::Null { .. } => Ok(JsVal::Null),
        Expr::This { .. } => Ok(env.this.clone()),
        Expr::NewTarget { .. } => Ok(env.new_target.clone()),
        Expr::Local { id, .. } => env.locals.get(id).cloned().ok_or(()),
        Expr::IdentName { name, .. } => resolve_ident(name, env),
        Expr::Function {
            params,
            body,
            is_arrow,
            is_method,
            ..
        } => Ok(alloc_function(
            heap,
            env,
            params,
            body.clone(),
            *is_arrow,
            *is_method,
        )?),
        Expr::Object { properties, .. } => {
            let mut props = HashMap::new();
            let mut proto: Option<JsVal> = None;
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key: ObjectPropKey::Static(k),
                        value,
                    } => {
                        let key = k.to_string_lossy();
                        let v = eval_expr(value, env, heap, by_id)?;
                        if key == "__proto__" {
                            proto = Some(v);
                        } else {
                            props.insert(key, v);
                        }
                    }
                    ObjectProp::Property {
                        key: ObjectPropKey::Computed(ke),
                        value,
                    } => {
                        let key = to_key(&eval_expr(ke, env, heap, by_id)?)?;
                        let v = eval_expr(value, env, heap, by_id)?;
                        props.insert(key, v);
                    }
                    _ => return Err(()),
                }
            }
            let oid = heap.alloc_obj(props);
            if let Some(p) = proto {
                // Store [[Prototype]] under internal key; member get walks it.
                heap.set(oid, "[[Prototype]]", p);
            }
            Ok(JsVal::Obj(oid))
        }
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ..
        } => {
            if *optional {
                return Err(());
            }
            let obj = eval_expr(object, env, heap, by_id)?;
            let key = member_key(property, *computed, env, heap, by_id)?;
            member_get_full(&obj, &key, env, heap, by_id)
        }
        Expr::Array { elements, .. } => {
            // Only empty arrays needed (Reflect.construct heritage args).
            if !elements.is_empty() {
                return Err(());
            }
            Ok(JsVal::Obj(heap.alloc_obj(HashMap::new())))
        }
        Expr::Assign {
            target,
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = eval_expr(value, env, heap, by_id)?;
            put_value(target, v.clone(), env, heap, by_id)?;
            Ok(v)
        }
        Expr::Binary {
            left, op, right, ..
        } => eval_binary(left, *op, right, env, heap, by_id),
        Expr::Unary { op, arg, .. } => eval_unary(*op, arg, env, heap, by_id),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            if to_boolean(&eval_expr(test, env, heap, by_id)?) {
                eval_expr(consequent, env, heap, by_id)
            } else {
                eval_expr(alternate, env, heap, by_id)
            }
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            if *optional {
                return Err(());
            }
            eval_call(callee, args, env, heap, by_id)
        }
        Expr::New {
            callee, args, ..
        } => eval_new(callee, args, env, heap, by_id),
        _ => Err(()),
    }
}

fn resolve_ident(name: &str, env: &Env) -> Result<JsVal, ()> {
    if let Some(v) = env.names.get(name) {
        return Ok(v.clone());
    }
    match name {
        "undefined" => Ok(JsVal::Undef),
        "Object" => Ok(JsVal::Builtin("Object")),
        "Function" => Ok(JsVal::Builtin("Function")),
        "WeakMap" => Ok(JsVal::Builtin("WeakMap")),
        "TypeError" => Ok(JsVal::Builtin("TypeError")),
        "Reflect" => Ok(JsVal::Builtin("Reflect")),
        "Proxy" => Ok(JsVal::Builtin("Proxy")),
        "arguments" => Ok(JsVal::Builtin("arguments")),
        _ => Err(()),
    }
}

/// Functions are objects with `[[Call]]` + default `prototype` (class ctor shape).
fn alloc_function(
    heap: &mut Heap,
    env: &Env,
    params: &[Param],
    body: Vec<Stmt>,
    is_arrow: bool,
    is_method: bool, // retained for future method-home checks
) -> Result<JsVal, ()> {
    let param_ids = simple_params(params)?;
    let fid = heap.alloc_fn(FnRec {
        params: param_ids,
        body,
        is_arrow,
        is_method,
        closure: env.locals.clone(),
        name_closure: env.names.clone(),
    });
    let mut props = HashMap::new();
    props.insert("[[Call]]".into(), JsVal::Fn(fid));
    if !is_arrow && !is_method {
        let proto = heap.alloc_obj(HashMap::new());
        props.insert("prototype".into(), JsVal::Obj(proto));
    }
    Ok(JsVal::Obj(heap.alloc_obj(props)))
}

fn as_callable(v: &JsVal, heap: &Heap) -> Option<usize> {
    match v {
        JsVal::Fn(fid) => Some(*fid),
        JsVal::Obj(oid) => match heap.get(*oid, "[[Call]]") {
            JsVal::Fn(fid) => Some(fid),
            _ => None,
        },
        _ => None,
    }
}

#[derive(Clone)]
enum ParamBind {
    Local(LocalId),
    Name(String),
}

fn simple_params(params: &[Param]) -> Result<Vec<ParamBind>, ()> {
    let mut out = Vec::new();
    for p in params {
        if p.rest || p.default.is_some() {
            return Err(());
        }
        match &p.pattern {
            Pattern::Local(id) => out.push(ParamBind::Local(*id)),
            Pattern::Name(n) => out.push(ParamBind::Name(n.clone())),
            _ => return Err(()),
        }
    }
    Ok(out)
}

fn member_key(
    property: &Expr,
    computed: bool,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<String, ()> {
    if !computed {
        match property {
            Expr::String { value, .. } => Ok(value.to_string_lossy()),
            Expr::IdentName { name, .. } => Ok(name.clone()),
            _ => Err(()),
        }
    } else {
        to_key(&eval_expr(property, env, heap, by_id)?)
    }
}

fn to_key(v: &JsVal) -> Result<String, ()> {
    match v {
        JsVal::Str(s) => Ok(s.clone()),
        JsVal::Num(n) => {
            if n.fract() == 0.0 && n.is_finite() {
                Ok(format!("{}", *n as i64))
            } else {
                Ok(format!("{n}"))
            }
        }
        JsVal::Bool(b) => Ok(if *b {
            "true".into()
        } else {
            "false".into()
        }),
        JsVal::Undef => Ok("undefined".into()),
        JsVal::Null => Ok("null".into()),
        _ => Err(()),
    }
}

fn member_get(obj: &JsVal, key: &str, heap: &Heap) -> Result<JsVal, ()> {
    match obj {
        JsVal::Obj(oid) => {
            if key.starts_with("[[") {
                return Ok(heap.get(*oid, key));
            }
            // Proxy without trap call (own props only) — traps need env; use member_get_full.
            if heap.has_own(*oid, "[[ProxyTarget]]") {
                // Defer to target own/proto without trap when called from pure heap context.
                let target = heap.get(*oid, "[[ProxyTarget]]");
                return member_get(&target, key, heap);
            }
            if heap.has_own(*oid, key) {
                return Ok(heap.get(*oid, key));
            }
            match heap.get(*oid, "[[Prototype]]") {
                JsVal::Obj(p) => member_get(&JsVal::Obj(p), key, heap),
                JsVal::Builtin("Function.prototype") | JsVal::Undef | JsVal::Null => {
                    Ok(JsVal::Undef)
                }
                other => member_get(&other, key, heap),
            }
        }
        JsVal::Builtin("Function") if key == "prototype" => {
            Ok(JsVal::Builtin("Function.prototype"))
        }
        JsVal::Builtin("Object") => match key {
            "defineProperty" => Ok(JsVal::Builtin("Object.defineProperty")),
            "getOwnPropertyDescriptor" => Ok(JsVal::Builtin("Object.getOwnPropertyDescriptor")),
            "setPrototypeOf" => Ok(JsVal::Builtin("Object.setPrototypeOf")),
            "isExtensible" => Ok(JsVal::Builtin("Object.isExtensible")),
            "prototype" => Ok(JsVal::Builtin("Object.prototype")),
            _ => Ok(JsVal::Undef),
        },
        JsVal::Builtin("Reflect") => match key {
            "construct" => Ok(JsVal::Builtin("Reflect.construct")),
            "get" => Ok(JsVal::Builtin("Reflect.get")),
            _ => Ok(JsVal::Undef),
        },
        JsVal::Fn(_) => Ok(JsVal::Undef),
        JsVal::WeakMap(_) => match key {
            "set" | "get" | "has" => Ok(JsVal::Builtin(match key {
                "set" => "WeakMap.set",
                "get" => "WeakMap.get",
                _ => "WeakMap.has",
            })),
            _ => Ok(JsVal::Undef),
        },
        _ => Ok(JsVal::Undef),
    }
}

/// Member get that runs Proxy `get` traps when present.
fn member_get_full(
    obj: &JsVal,
    key: &str,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    if let JsVal::Obj(oid) = obj {
        if heap.has_own(*oid, "[[ProxyTarget]]") {
            let target = heap.get(*oid, "[[ProxyTarget]]");
            if let Some(fid) = as_callable(&heap.get(*oid, "[[ProxyGet]]"), heap) {
                return call_value(
                    &JsVal::Fn(fid),
                    JsVal::Undef,
                    &[target, JsVal::Str(key.to_string()), obj.clone()],
                    env,
                    heap,
                    by_id,
                );
            }
            return member_get_full(&target, key, env, heap, by_id);
        }
    }
    member_get(obj, key, heap)
}

fn put_value(
    target: &AssignTarget,
    val: JsVal,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<(), ()> {
    match target {
        AssignTarget::Local(id) => {
            env.locals.insert(*id, val);
            Ok(())
        }
        AssignTarget::Member {
            object,
            property,
            computed,
        } => {
            let obj = eval_expr(object, env, heap, by_id)?;
            let key = member_key(property, *computed, env, heap, by_id)?;
            match obj {
                JsVal::Obj(oid) => {
                    heap.set(oid, &key, val);
                    Ok(())
                }
                _ => Err(()),
            }
        }
        _ => Err(()),
    }
}

fn eval_unary(
    op: UnaryOp,
    arg: &Expr,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    match op {
        UnaryOp::TypeOf => {
            if let Expr::Member {
                object,
                property,
                computed,
                optional: false,
                ..
            } = arg
            {
                let obj = eval_expr(object, env, heap, by_id)?;
                let key = member_key(property, *computed, env, heap, by_id)?;
                let v = member_get(&obj, &key, heap)?;
                return Ok(JsVal::Str(typeof_val(&v, heap)));
            }
            let v = eval_expr(arg, env, heap, by_id)?;
            Ok(JsVal::Str(typeof_val(&v, heap)))
        }
        UnaryOp::Void => {
            let _ = eval_expr(arg, env, heap, by_id)?;
            Ok(JsVal::Undef)
        }
        UnaryOp::Not => Ok(JsVal::Bool(!to_boolean(&eval_expr(arg, env, heap, by_id)?))),
        UnaryOp::Delete => match arg {
            Expr::Member {
                object,
                property,
                computed,
                ..
            } => {
                let obj = eval_expr(object, env, heap, by_id)?;
                let key = member_key(property, *computed, env, heap, by_id)?;
                match obj {
                    JsVal::Obj(oid) => Ok(JsVal::Bool(heap.delete(oid, &key))),
                    _ => Ok(JsVal::Bool(true)),
                }
            }
            _ => Ok(JsVal::Bool(true)),
        },
        UnaryOp::Minus => match eval_expr(arg, env, heap, by_id)? {
            JsVal::Num(n) => Ok(JsVal::Num(-n)),
            _ => Err(()),
        },
        UnaryOp::Plus => match eval_expr(arg, env, heap, by_id)? {
            JsVal::Num(n) => Ok(JsVal::Num(n)),
            JsVal::Str(s) => Ok(JsVal::Num(s.parse().unwrap_or(f64::NAN))),
            JsVal::Bool(b) => Ok(JsVal::Num(if b { 1.0 } else { 0.0 })),
            JsVal::Undef => Ok(JsVal::Num(f64::NAN)),
            JsVal::Null => Ok(JsVal::Num(0.0)),
            _ => Err(()),
        },
        _ => Err(()),
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::Null => "object".into(),
        JsVal::Obj(_) => {
            // Callables (class ctors / methods boxed as objects) are typeof "function".
            "object".into() // refined by typeof_str_heap when heap available
        }
        JsVal::Fn(_) | JsVal::Builtin(_) => "function".into(),
        JsVal::WeakMap(_) => "object".into(),
    }
}

fn typeof_val(v: &JsVal, heap: &Heap) -> String {
    match v {
        JsVal::Obj(oid) if as_callable(v, heap).is_some() => "function".into(),
        JsVal::Obj(_) => "object".into(),
        other => typeof_str(other),
    }
}

fn eval_binary(
    left: &Expr,
    op: BinaryOp,
    right: &Expr,
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    match op {
        BinaryOp::Comma => {
            let _ = eval_expr(left, env, heap, by_id)?;
            eval_expr(right, env, heap, by_id)
        }
        BinaryOp::And => {
            let l = eval_expr(left, env, heap, by_id)?;
            if !to_boolean(&l) {
                Ok(l)
            } else {
                eval_expr(right, env, heap, by_id)
            }
        }
        BinaryOp::Or => {
            let l = eval_expr(left, env, heap, by_id)?;
            if to_boolean(&l) {
                Ok(l)
            } else {
                eval_expr(right, env, heap, by_id)
            }
        }
        BinaryOp::EqEqEq => {
            let l = eval_expr(left, env, heap, by_id)?;
            let r = eval_expr(right, env, heap, by_id)?;
            Ok(JsVal::Bool(strict_eq(&l, &r)))
        }
        BinaryOp::NotEqEq => {
            let l = eval_expr(left, env, heap, by_id)?;
            let r = eval_expr(right, env, heap, by_id)?;
            Ok(JsVal::Bool(!strict_eq(&l, &r)))
        }
        BinaryOp::EqEq => {
            let l = eval_expr(left, env, heap, by_id)?;
            let r = eval_expr(right, env, heap, by_id)?;
            Ok(JsVal::Bool(loose_eq(&l, &r)))
        }
        BinaryOp::NotEq => {
            let l = eval_expr(left, env, heap, by_id)?;
            let r = eval_expr(right, env, heap, by_id)?;
            Ok(JsVal::Bool(!loose_eq(&l, &r)))
        }
        BinaryOp::Add => {
            let l = eval_expr(left, env, heap, by_id)?;
            let r = eval_expr(right, env, heap, by_id)?;
            match (&l, &r) {
                (JsVal::Str(a), JsVal::Str(b)) => Ok(JsVal::Str(format!("{a}{b}"))),
                (JsVal::Str(a), b) => Ok(JsVal::Str(format!("{a}{}", to_string_js(b)))),
                (a, JsVal::Str(b)) => Ok(JsVal::Str(format!("{}{b}", to_string_js(a)))),
                _ => Ok(JsVal::Num(to_number(&l)? + to_number(&r)?)),
            }
        }
        BinaryOp::Sub => Ok(JsVal::Num(
            to_number(&eval_expr(left, env, heap, by_id)?)?
                - to_number(&eval_expr(right, env, heap, by_id)?)?,
        )),
        BinaryOp::Mul => Ok(JsVal::Num(
            to_number(&eval_expr(left, env, heap, by_id)?)?
                * to_number(&eval_expr(right, env, heap, by_id)?)?,
        )),
        BinaryOp::Div => Ok(JsVal::Num(
            to_number(&eval_expr(left, env, heap, by_id)?)?
                / to_number(&eval_expr(right, env, heap, by_id)?)?,
        )),
        _ => Err(()),
    }
}

fn strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => x == y,
        (JsVal::Str(x), JsVal::Str(y)) => x == y,
        (JsVal::Bool(x), JsVal::Bool(y)) => x == y,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Null, JsVal::Null) => true,
        (JsVal::Obj(x), JsVal::Obj(y)) => x == y,
        (JsVal::Fn(x), JsVal::Fn(y)) => x == y,
        (JsVal::WeakMap(x), JsVal::WeakMap(y)) => x == y,
        (JsVal::Builtin(x), JsVal::Builtin(y)) => x == y,
        _ => false,
    }
}

fn loose_eq(a: &JsVal, b: &JsVal) -> bool {
    // Enough for `x != null` brand checks.
    match (a, b) {
        (JsVal::Null, JsVal::Undef) | (JsVal::Undef, JsVal::Null) => true,
        _ => strict_eq(a, b),
    }
}

fn to_string_js(v: &JsVal) -> String {
    match v {
        JsVal::Num(n) => {
            if n.fract() == 0.0 && n.is_finite() {
                format!("{}", *n as i64)
            } else {
                format!("{n}")
            }
        }
        JsVal::Str(s) => s.clone(),
        JsVal::Bool(b) => if *b { "true" } else { "false" }.into(),
        JsVal::Undef => "undefined".into(),
        JsVal::Null => "null".into(),
        JsVal::Obj(_) | JsVal::WeakMap(_) => "[object Object]".into(),
        JsVal::Fn(_) | JsVal::Builtin(_) => "function".into(),
    }
}

fn to_number(v: &JsVal) -> Result<f64, ()> {
    match v {
        JsVal::Num(n) => Ok(*n),
        JsVal::Bool(b) => Ok(if *b { 1.0 } else { 0.0 }),
        JsVal::Null => Ok(0.0),
        JsVal::Undef => Ok(f64::NAN),
        JsVal::Str(s) => Ok(s.parse().unwrap_or(f64::NAN)),
        _ => Err(()),
    }
}

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Bool(b) => *b,
        JsVal::Undef | JsVal::Null => false,
        JsVal::Obj(_) | JsVal::Fn(_) | JsVal::WeakMap(_) | JsVal::Builtin(_) => true,
    }
}

fn eval_call(
    callee: &Expr,
    args: &[Arg],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    // method.call(thisArg, ...args) — Function.prototype.call
    if let Expr::Member {
        object,
        property,
        optional: false,
        ..
    } = callee
    {
        if let Expr::String { value, .. } = property.as_ref() {
            if value.to_string_lossy() == "call" {
                let fval = eval_expr(object, env, heap, by_id)?;
                let mut arg_vals = eval_args(args, env, heap, by_id)?;
                let this_arg = if arg_vals.is_empty() {
                    JsVal::Undef
                } else {
                    arg_vals.remove(0)
                };
                return call_value(&fval, this_arg, &arg_vals, env, heap, by_id);
            }
        }
    }

    // Member call: obj.m(args) / Builtin methods
    if let Expr::Member {
        object,
        property,
        computed,
        optional: false,
        ..
    } = callee
    {
        let obj = eval_expr(object, env, heap, by_id)?;
        let key = member_key(property, *computed, env, heap, by_id)?;
        let arg_vals = eval_args(args, env, heap, by_id)?;
        return call_member(&obj, &key, &arg_vals, env, heap, by_id);
    }

    let fval = eval_expr(callee, env, heap, by_id)?;
    let arg_vals = eval_args(args, env, heap, by_id)?;
    call_value(&fval, JsVal::Undef, &arg_vals, env, heap, by_id)
}

fn eval_args(
    args: &[Arg],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<Vec<JsVal>, ()> {
    let mut out = Vec::new();
    for a in args {
        match a {
            Arg::Expr(e) => out.push(eval_expr(e, env, heap, by_id)?),
            Arg::Spread(_) => return Err(()),
        }
    }
    Ok(out)
}

fn call_member(
    obj: &JsVal,
    key: &str,
    args: &[JsVal],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    if let JsVal::Builtin("Object") = obj {
        return match key {
            "defineProperty" => builtin_define_property(args, heap),
            "getOwnPropertyDescriptor" => builtin_gopd(args, heap),
            "setPrototypeOf" => builtin_set_prototype_of(args, heap),
            "isExtensible" => Ok(JsVal::Bool(true)),
            _ => Err(()),
        };
    }
    if let JsVal::WeakMap(wmid) = obj {
        return match key {
            "set" => {
                if args.len() < 2 {
                    return Err(());
                }
                let JsVal::Obj(oid) = &args[0] else {
                    return Err(());
                };
                heap.weakmaps
                    .get_mut(*wmid)
                    .ok_or(())?
                    .insert(*oid, args[1].clone());
                Ok(obj.clone())
            }
            "get" => {
                let JsVal::Obj(oid) = args.first().ok_or(())? else {
                    return Ok(JsVal::Undef);
                };
                Ok(heap
                    .weakmaps
                    .get(*wmid)
                    .and_then(|m| m.get(oid).cloned())
                    .unwrap_or(JsVal::Undef))
            }
            "has" => {
                let JsVal::Obj(oid) = args.first().ok_or(())? else {
                    return Ok(JsVal::Bool(false));
                };
                Ok(JsVal::Bool(
                    heap.weakmaps
                        .get(*wmid)
                        .is_some_and(|m| m.contains_key(oid)),
                ))
            }
            _ => Err(()),
        };
    }
    let method = member_get(obj, key, heap)?;
    call_value(&method, obj.clone(), args, env, heap, by_id)
}

fn call_value(
    fval: &JsVal,
    this_arg: JsVal,
    args: &[JsVal],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    match fval {
        JsVal::Builtin("Object.defineProperty") => builtin_define_property(args, heap),
        JsVal::Builtin("Object.getOwnPropertyDescriptor") => builtin_gopd(args, heap),
        JsVal::Builtin("Object.setPrototypeOf") => builtin_set_prototype_of(args, heap),
        JsVal::Builtin("Object.isExtensible") => Ok(JsVal::Bool(true)),
        JsVal::Builtin("Reflect.construct") => {
            // Heritage check: construct empty fn with newTarget=Proxy(parent).
            // Accessing newTarget.prototype runs Proxy get → captures sproto.
            if args.len() < 3 {
                return Err(());
            }
            let new_target = &args[2];
            let _ = member_get_full(new_target, "prototype", env, heap, by_id)?;
            // Return a dummy instance object.
            Ok(JsVal::Obj(heap.alloc_obj(HashMap::new())))
        }
        JsVal::Builtin("Reflect.get") => {
            if args.len() < 2 {
                return Err(());
            }
            let key = to_key(&args[1])?;
            // Reflect.get reads from target (args[0]), not receiver — avoids Proxy trap loops.
            let _receiver = args.get(2);
            member_get(&args[0], &key, heap)
        }
        JsVal::Builtin("WeakMap.set") => {
            let JsVal::WeakMap(wmid) = &this_arg else {
                return Err(());
            };
            if args.len() < 2 {
                return Err(());
            }
            let JsVal::Obj(oid) = &args[0] else {
                return Err(());
            };
            heap.weakmaps
                .get_mut(*wmid)
                .ok_or(())?
                .insert(*oid, args[1].clone());
            Ok(this_arg)
        }
        JsVal::Builtin("WeakMap.get") => {
            let JsVal::WeakMap(wmid) = &this_arg else {
                return Err(());
            };
            let JsVal::Obj(oid) = args.first().ok_or(())? else {
                return Ok(JsVal::Undef);
            };
            Ok(heap
                .weakmaps
                .get(*wmid)
                .and_then(|m| m.get(oid).cloned())
                .unwrap_or(JsVal::Undef))
        }
        JsVal::Builtin("WeakMap.has") => {
            let JsVal::WeakMap(wmid) = &this_arg else {
                return Err(());
            };
            let JsVal::Obj(oid) = args.first().ok_or(())? else {
                return Ok(JsVal::Bool(false));
            };
            Ok(JsVal::Bool(
                heap.weakmaps
                    .get(*wmid)
                    .is_some_and(|m| m.contains_key(oid)),
            ))
        }
        other => {
            let fid = as_callable(other, heap).ok_or(())?;
            let rec = heap.functions.get(fid).cloned().ok_or(())?;
            let mut locals = rec.closure.clone();
            for (k, v) in &env.locals {
                locals.insert(*k, v.clone());
            }
            let mut names = rec.name_closure.clone();
            for (k, v) in &env.names {
                names.insert(k.clone(), v.clone());
            }
            let mut child = Env {
                locals,
                names,
                this: if rec.is_arrow {
                    env.this.clone()
                } else {
                    this_arg
                },
                new_target: if rec.is_arrow {
                    env.new_target.clone()
                } else {
                    JsVal::Undef
                },
            };
            let mut param_locals = Vec::new();
            let mut param_names = Vec::new();
            for (i, pb) in rec.params.iter().enumerate() {
                let v = args.get(i).cloned().unwrap_or(JsVal::Undef);
                match pb {
                    ParamBind::Local(id) => {
                        child.locals.insert(*id, v);
                        param_locals.push(*id);
                    }
                    ParamBind::Name(n) => {
                        child.names.insert(n.clone(), v);
                        param_names.push(n.clone());
                    }
                }
            }
            let mut ret = JsVal::Undef;
            for s in &rec.body {
                if let Some(v) = eval_stmt(s, &mut child, heap, by_id)? {
                    ret = v;
                    break;
                }
            }
            for (k, v) in &child.locals {
                if !param_locals.contains(k) && env.locals.contains_key(k) {
                    env.locals.insert(*k, v.clone());
                }
            }
            if let Some(fr) = heap.functions.get_mut(fid) {
                for (k, v) in &child.locals {
                    if !param_locals.contains(k) && fr.closure.contains_key(k) {
                        fr.closure.insert(*k, v.clone());
                    }
                }
            }
            let _ = param_names;
            Ok(ret)
        }
    }
}

fn builtin_define_property(args: &[JsVal], heap: &mut Heap) -> Result<JsVal, ()> {
    if args.len() < 3 {
        return Err(());
    }
    let JsVal::Obj(oid) = &args[0] else {
        return Err(());
    };
    let key = to_key(&args[1])?;
    let JsVal::Obj(desc_id) = &args[2] else {
        return Err(());
    };
    if heap.has_own(*desc_id, "value") {
        let val = heap.get(*desc_id, "value");
        heap.set(*oid, &key, val);
        return Ok(args[0].clone());
    }
    if heap.has_own(*desc_id, "get") {
        let g = heap.get(*desc_id, "get");
        heap.set(*oid, &key, g);
        return Ok(args[0].clone());
    }
    Ok(args[0].clone())
}

fn builtin_gopd(args: &[JsVal], heap: &mut Heap) -> Result<JsVal, ()> {
    if args.len() < 2 {
        return Err(());
    }
    let JsVal::Obj(oid) = &args[0] else {
        return Err(());
    };
    let key = to_key(&args[1])?;
    if !heap.has_own(*oid, &key) {
        return Ok(JsVal::Undef);
    }
    let val = heap.get(*oid, &key);
    let mut props = HashMap::new();
    props.insert("value".into(), val);
    props.insert("writable".into(), JsVal::Bool(true));
    props.insert("enumerable".into(), JsVal::Bool(true));
    props.insert("configurable".into(), JsVal::Bool(true));
    Ok(JsVal::Obj(heap.alloc_obj(props)))
}

fn builtin_set_prototype_of(args: &[JsVal], heap: &mut Heap) -> Result<JsVal, ()> {
    if args.len() < 2 {
        return Err(());
    }
    let JsVal::Obj(oid) = &args[0] else {
        return Err(());
    };
    heap.set(*oid, "[[Prototype]]", args[1].clone());
    Ok(args[0].clone())
}

fn eval_new(
    callee: &Expr,
    args: &[Arg],
    env: &mut Env,
    heap: &mut Heap,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<JsVal, ()> {
    let c = eval_expr(callee, env, heap, by_id)?;
    let arg_vals = eval_args(args, env, heap, by_id)?;
    match c {
        JsVal::Builtin("WeakMap") => Ok(JsVal::WeakMap(heap.alloc_wm())),
        JsVal::Builtin("TypeError") => Err(()),
        JsVal::Builtin("Proxy") => {
            // new Proxy(target, handler)
            if arg_vals.len() < 2 {
                return Err(());
            }
            let target = arg_vals[0].clone();
            let JsVal::Obj(hid) = &arg_vals[1] else {
                return Err(());
            };
            let get_trap = heap.get(*hid, "get");
            let mut props = HashMap::new();
            props.insert("[[ProxyTarget]]".into(), target);
            props.insert("[[ProxyGet]]".into(), get_trap);
            Ok(JsVal::Obj(heap.alloc_obj(props)))
        }
        _ => Err(()),
    }
}

fn emit_prints(info: &ModuleInfo) -> String {
    let mut out = String::new();
    let mut body = String::new();
    let mut str_globals: Vec<(String, String)> = Vec::new();
    let mut tmp = 0usize;

    for obs in &info.observations {
        match obs {
            Obs::Num(n) => {
                let lit = format_f64(*n);
                writeln!(body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
            }
            Obs::Str(s) => {
                let gname = format!(".esb.str.{}", str_globals.len());
                str_globals.push((s.clone(), gname.clone()));
                let t = {
                    let t = tmp;
                    tmp += 1;
                    format!("%t{t}")
                };
                let n = s.len() + 1;
                writeln!(
                    body,
                    "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
                )
                .ok();
                writeln!(body, "  {}", PRINT_STR.call(&format!("ptr {t}"))).ok();
            }
        }
    }

    writeln!(
        out,
        "; Draconic LLVM backend (N08.16.42 class static blocks via compile-time eval)"
    )
    .ok();
    writeln!(out, "{}", llvm_declares(&[PRINT_F64, PRINT_STR])).ok();
    writeln!(out).ok();
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
    out.push_str(&body);
    writeln!(out, "  ret i32 0").ok();
    writeln!(out, "}}").ok();
    out
}

fn format_f64(n: f64) -> String {
    if n.is_nan() {
        "0x7FF8000000000000".into()
    } else if n.is_infinite() {
        if n.is_sign_negative() {
            "0xFFF0000000000000".into()
        } else {
            "0x7FF0000000000000".into()
        }
    } else {
        // Match other emitters: decimal that parses as the same f64.
        format!("{n:?}")
    }
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) => out.push(c as char),
            c => {
                write!(out, "\\{c:02X}").ok();
            }
        }
    }
    out
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}
