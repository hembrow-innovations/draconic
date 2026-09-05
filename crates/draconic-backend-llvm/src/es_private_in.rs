//! N08.16.41: native observations for private brand check `#x in obj` (E18.40 /
//! `es/annex-b/private_in`).
//!
//! Compile-time evaluation of class IIFEs that lower private fields/methods/accessors
//! to WeakMap/WeakSet brands plus `Object.defineProperty` methods. Supports brand
//! checks (`obj != null && typeof object-like && brand.has(obj)`), instance/static
//! private, inheritance (`extends` + `Reflect.construct` + `Proxy` heritage probe),
//! and prints top-level number/boolean observations via Runtime.

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};

pub(crate) fn is_es_private_in_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_private_in(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_private_in module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

thread_local! {
    static CURRENT_THIS: RefCell<JsVal> = const { RefCell::new(JsVal::Undef) };
    static CURRENT_NEW_TARGET: RefCell<JsVal> = const { RefCell::new(JsVal::Undef) };
}

const OBJECT_PROTOTYPE_IDX: usize = 0;
const FUNCTION_PROTOTYPE_IDX: usize = 1;

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Null,
    Object(usize),
    /// Callable + object props (`fn_idx` body, `obj_idx` props/prototype).
    Fn {
        fn_idx: usize,
        obj_idx: usize,
    },
    WeakMap(usize),
    WeakSet(usize),
    Proxy(usize),
    Builtin(Builtin),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Builtin {
    Object,
    Function,
    TypeError,
    ReferenceError,
    Undefined,
    Reflect,
    Proxy,
    WeakMap,
    WeakSet,
    ObjectDefineProperty,
    ObjectGetOwnPropertyDescriptor,
    ObjectIsExtensible,
    ObjectSetPrototypeOf,
    ReflectConstruct,
    ReflectGet,
}

#[derive(Clone, Debug)]
struct FnRec {
    params: Vec<ParamRec>,
    body: Vec<Stmt>,
    is_arrow: bool,
}

#[derive(Clone, Debug)]
enum ParamBind {
    Local(LocalId),
    Name(String),
}

#[derive(Clone, Debug)]
struct ParamRec {
    bind: ParamBind,
    rest: bool,
}

#[derive(Clone, Debug)]
struct ObjectRec {
    props: HashMap<String, JsVal>,
    keys: Vec<String>,
    proto: JsVal,
    extensible: bool,
}

#[derive(Clone, Debug)]
struct ProxyRec {
    target: JsVal,
    get_trap: Option<usize>,
}

#[derive(Clone, Debug)]
struct WeakMapRec {
    entries: Vec<(usize, JsVal)>, // object identity → value
}

#[derive(Clone, Debug)]
struct WeakSetRec {
    keys: Vec<usize>,
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

struct World {
    env: HashMap<LocalId, JsVal>,
    /// Dynamic name bindings (IR `Pattern::Name` / free IdentName params).
    name_env: HashMap<String, JsVal>,
    fns: Vec<FnRec>,
    objects: Vec<ObjectRec>,
    proxies: Vec<ProxyRec>,
    weak_maps: Vec<WeakMapRec>,
    weak_sets: Vec<WeakSetRec>,
    by_name: HashMap<String, LocalId>,
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
enum Flow {
    Normal,
    Return(JsVal),
    Throw(JsVal),
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    if !module_looks_like_private_in(module) {
        return None;
    }
    let mut w = World::new(module);
    match w.eval_body(&module.body) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match w.env.get(local) {
                Some(v @ (JsVal::Num(_) | JsVal::Bool(_) | JsVal::Str(_))) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String
                    ) {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(
                    JsVal::Undef
                    | JsVal::Null
                    | JsVal::Object(_)
                    | JsVal::Fn { .. }
                    | JsVal::WeakMap(_)
                    | JsVal::WeakSet(_)
                    | JsVal::Proxy(_)
                    | JsVal::Builtin(_),
                ) => {}
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

fn module_looks_like_private_in(module: &Module) -> bool {
    let names: HashMap<LocalId, &str> = module
        .locals
        .iter()
        .map(|l| (l.id, l.name.as_str()))
        .collect();
    let mut has_wm = false;
    let mut has_ws = false;
    let mut has_define = false;
    fn walk_expr(
        e: &Expr,
        names: &HashMap<LocalId, &str>,
        has_wm: &mut bool,
        has_ws: &mut bool,
        has_define: &mut bool,
    ) {
        match e {
            Expr::IdentName { name, .. } => {
                if name == "WeakMap" {
                    *has_wm = true;
                }
                if name == "WeakSet" {
                    *has_ws = true;
                }
            }
            Expr::Local { id, .. } => match names.get(id).copied() {
                Some("WeakMap") => *has_wm = true,
                Some("WeakSet") => *has_ws = true,
                _ => {}
            },
            Expr::New { callee, args, .. } => {
                walk_expr(callee, names, has_wm, has_ws, has_define);
                for a in args {
                    if let Arg::Expr(e) = a {
                        walk_expr(e, names, has_wm, has_ws, has_define);
                    }
                }
            }
            Expr::Call { callee, args, .. } => {
                if let Expr::Member {
                    object,
                    property: prop,
                    ..
                } = callee.as_ref()
                {
                    if matches!(
                        (object.as_ref(), prop.as_ref()),
                        (
                            Expr::IdentName { name, .. },
                            Expr::String { value, .. }
                        ) if name == "Object" && value.to_string_lossy() == "defineProperty"
                    ) {
                        *has_define = true;
                    }
                }
                walk_expr(callee, names, has_wm, has_ws, has_define);
                for a in args {
                    if let Arg::Expr(e) = a {
                        walk_expr(e, names, has_wm, has_ws, has_define);
                    }
                }
            }
            Expr::Member {
                object, property, ..
            } => {
                walk_expr(object, names, has_wm, has_ws, has_define);
                walk_expr(property, names, has_wm, has_ws, has_define);
            }
            Expr::Unary { arg, .. } => walk_expr(arg, names, has_wm, has_ws, has_define),
            Expr::Binary { left, right, .. } => {
                walk_expr(left, names, has_wm, has_ws, has_define);
                walk_expr(right, names, has_wm, has_ws, has_define);
            }
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                walk_expr(test, names, has_wm, has_ws, has_define);
                walk_expr(consequent, names, has_wm, has_ws, has_define);
                walk_expr(alternate, names, has_wm, has_ws, has_define);
            }
            Expr::Assign { value, .. } => walk_expr(value, names, has_wm, has_ws, has_define),
            Expr::Function { body, .. } => {
                for s in body {
                    walk_stmt(s, names, has_wm, has_ws, has_define);
                }
            }
            Expr::Object { properties, .. } => {
                for p in properties {
                    match p {
                        ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                            walk_expr(value, names, has_wm, has_ws, has_define)
                        }
                        ObjectProp::Spread(e) => walk_expr(e, names, has_wm, has_ws, has_define),
                    }
                }
            }
            Expr::Array { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                            walk_expr(e, names, has_wm, has_ws, has_define)
                        }
                        ArrayElement::Elision => {}
                    }
                }
            }
            _ => {}
        }
    }
    fn walk_stmt(
        s: &Stmt,
        names: &HashMap<LocalId, &str>,
        has_wm: &mut bool,
        has_ws: &mut bool,
        has_define: &mut bool,
    ) {
        match s {
            Stmt::Declare { init: Some(e), .. }
            | Stmt::Expr { expr: e }
            | Stmt::Throw { value: e } => walk_expr(e, names, has_wm, has_ws, has_define),
            Stmt::Block { body } | Stmt::Function { body, .. } => {
                for s in body {
                    walk_stmt(s, names, has_wm, has_ws, has_define);
                }
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                walk_expr(test, names, has_wm, has_ws, has_define);
                walk_stmt(consequent, names, has_wm, has_ws, has_define);
                if let Some(a) = alternate {
                    walk_stmt(a, names, has_wm, has_ws, has_define);
                }
            }
            Stmt::Return { value: Some(e) } => walk_expr(e, names, has_wm, has_ws, has_define),
            Stmt::Labeled { body, .. } => walk_stmt(body, names, has_wm, has_ws, has_define),
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                for s in block {
                    walk_stmt(s, names, has_wm, has_ws, has_define);
                }
                if let Some(h) = handler {
                    for s in h {
                        walk_stmt(s, names, has_wm, has_ws, has_define);
                    }
                }
                if let Some(f) = finalizer {
                    for s in f {
                        walk_stmt(s, names, has_wm, has_ws, has_define);
                    }
                }
            }
            _ => {}
        }
    }
    for s in &module.body {
        walk_stmt(s, &names, &mut has_wm, &mut has_ws, &mut has_define);
    }
    (has_wm || has_ws) && has_define
}

impl World {
    fn new(module: &Module) -> Self {
        let mut env = HashMap::new();
        let mut by_name = HashMap::new();
        for loc in &module.locals {
            by_name.insert(loc.name.clone(), loc.id);
            if loc.name == "undefined" {
                env.insert(loc.id, JsVal::Undef);
            } else if let Some(b) = builtin_for_name(&loc.name) {
                env.insert(loc.id, JsVal::Builtin(b));
            }
        }
        let objects = vec![
            ObjectRec {
                props: HashMap::new(),
                keys: Vec::new(),
                proto: JsVal::Null,
                extensible: true,
            },
            ObjectRec {
                props: HashMap::new(),
                keys: Vec::new(),
                proto: JsVal::Object(OBJECT_PROTOTYPE_IDX),
                extensible: true,
            },
        ];
        Self {
            env,
            name_env: HashMap::new(),
            fns: Vec::new(),
            objects,
            proxies: Vec::new(),
            weak_maps: Vec::new(),
            weak_sets: Vec::new(),
            by_name,
        }
    }

    fn eval_body(&mut self, body: &[Stmt]) -> Result<Flow, ()> {
        for stmt in body {
            match self.eval_stmt(stmt)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    fn eval_stmt(&mut self, stmt: &Stmt) -> Result<Flow, ()> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let v = match init {
                    Some(e) => match self.eval_expr(e)? {
                        Ok(v) => v,
                        Err(flow) => return Ok(flow),
                    },
                    None => JsVal::Undef,
                };
                self.env.insert(*local, v);
                Ok(Flow::Normal)
            }
            Stmt::Expr { expr } => match self.eval_expr(expr)? {
                Ok(_) => Ok(Flow::Normal),
                Err(flow) => Ok(flow),
            },
            Stmt::Block { body } => self.eval_body(body),
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let t = match self.eval_expr(test)? {
                    Ok(v) => v,
                    Err(flow) => return Ok(flow),
                };
                if is_truthy(&t) {
                    self.eval_stmt(consequent)
                } else if let Some(a) = alternate {
                    self.eval_stmt(a)
                } else {
                    Ok(Flow::Normal)
                }
            }
            Stmt::Return { value: None } => Ok(Flow::Return(JsVal::Undef)),
            Stmt::Return { value: Some(e) } => match self.eval_expr(e)? {
                Ok(v) => Ok(Flow::Return(v)),
                Err(flow) => Ok(flow),
            },
            Stmt::Throw { value } => match self.eval_expr(value)? {
                Ok(v) => Ok(Flow::Throw(v)),
                Err(flow) => Ok(flow),
            },
            Stmt::Labeled { body, .. } => self.eval_stmt(body),
            Stmt::Function {
                local,
                params,
                body,
                is_async: false,
                is_generator: false,
            } => {
                let f = self.register_fn(params, body, false)?;
                self.env.insert(*local, f);
                Ok(Flow::Normal)
            }
            Stmt::Try {
                block,
                handler_param,
                handler,
                finalizer,
            } => {
                let mut flow = self.eval_body(block)?;
                if let Flow::Throw(exc) = flow {
                    if let Some(h) = handler {
                        if let Some(Pattern::Local(pid)) = handler_param {
                            self.env.insert(*pid, exc);
                        }
                        flow = self.eval_body(h)?;
                    } else {
                        flow = Flow::Throw(exc);
                    }
                }
                if let Some(f) = finalizer {
                    let fin = self.eval_body(f)?;
                    if !matches!(fin, Flow::Normal) {
                        return Ok(fin);
                    }
                }
                Ok(flow)
            }
            _ => Err(()),
        }
    }

    fn register_fn(
        &mut self,
        params: &[Param],
        body: &[Stmt],
        is_arrow: bool,
    ) -> Result<JsVal, ()> {
        let mut precs = Vec::new();
        for p in params {
            if p.default.is_some() {
                return Err(());
            }
            let bind = match &p.pattern {
                Pattern::Local(id) => ParamBind::Local(*id),
                Pattern::Name(n) => ParamBind::Name(n.clone()),
                _ => return Err(()),
            };
            precs.push(ParamRec { bind, rest: p.rest });
        }
        let fn_idx = self.fns.len();
        self.fns.push(FnRec {
            params: precs,
            body: body.to_vec(),
            is_arrow,
        });
        let mut rec = ObjectRec {
            props: HashMap::new(),
            keys: Vec::new(),
            proto: JsVal::Object(FUNCTION_PROTOTYPE_IDX),
            extensible: true,
        };
        // Default .prototype for constructors.
        let proto_idx = self.objects.len() + 1; // after we push fn object
        let fn_obj_idx = self.objects.len();
        // placeholder proto object
        let mut proto_rec = empty_object();
        object_set_prop(
            &mut proto_rec,
            "constructor".into(),
            JsVal::Fn {
                fn_idx,
                obj_idx: fn_obj_idx,
            },
        );
        object_set_prop(&mut rec, "prototype".into(), JsVal::Object(proto_idx));
        self.objects.push(rec);
        self.objects.push(proto_rec);
        // fix constructor circular — already set with correct fn_obj_idx
        let _ = proto_idx;
        Ok(JsVal::Fn {
            fn_idx,
            obj_idx: fn_obj_idx,
        })
    }

    fn eval_expr(&mut self, expr: &Expr) -> Result<Result<JsVal, Flow>, ()> {
        match expr {
            Expr::Number { raw, .. } => {
                let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
                let n: f64 = cleaned.parse().map_err(|_| ())?;
                Ok(Ok(JsVal::Num(n)))
            }
            Expr::Boolean { value, .. } => Ok(Ok(JsVal::Bool(*value))),
            Expr::String { value, .. } => Ok(Ok(JsVal::Str(js_string_to_utf8(value)))),
            Expr::Null { .. } => Ok(Ok(JsVal::Null)),
            Expr::Local { id, .. } => Ok(Ok(self.env.get(id).cloned().unwrap_or(JsVal::Undef))),
            Expr::IdentName { name, .. } => {
                if let Some(v) = self.name_env.get(name) {
                    return Ok(Ok(v.clone()));
                }
                if let Some(id) = self.by_name.get(name) {
                    if let Some(v) = self.env.get(id) {
                        return Ok(Ok(v.clone()));
                    }
                }
                if let Some(b) = builtin_for_name(name) {
                    return Ok(Ok(JsVal::Builtin(b)));
                }
                if name == "undefined" {
                    return Ok(Ok(JsVal::Undef));
                }
                Err(())
            }
            Expr::This { .. } => Ok(Ok(CURRENT_THIS.with(|c| c.borrow().clone()))),
            Expr::NewTarget { .. } => Ok(Ok(CURRENT_NEW_TARGET.with(|c| c.borrow().clone()))),
            Expr::Function {
                params,
                body,
                is_async: false,
                is_generator: false,
                is_arrow,
                ..
            } => Ok(Ok(self.register_fn(params, body, *is_arrow)?)),
            Expr::Unary { op, arg, .. } => {
                let v = match self.eval_expr(arg)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                match op {
                    UnaryOp::TypeOf => Ok(Ok(JsVal::Str(typeof_str(&v)))),
                    UnaryOp::Not => Ok(Ok(JsVal::Bool(!is_truthy(&v)))),
                    UnaryOp::Void => Ok(Ok(JsVal::Undef)),
                    UnaryOp::Delete => {
                        // delete obj.prop — arg should be Member
                        if let Expr::Member {
                            object,
                            property,
                            optional: false,
                            ..
                        } = arg.as_ref()
                        {
                            let obj = match self.eval_expr(object)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            let key = match self.eval_key(property)? {
                                Ok(k) => k,
                                Err(f) => return Ok(Err(f)),
                            };
                            Ok(Ok(JsVal::Bool(self.object_delete(&obj, &key)?)))
                        } else {
                            Ok(Ok(JsVal::Bool(true)))
                        }
                    }
                    UnaryOp::Minus => match v {
                        JsVal::Num(n) => Ok(Ok(JsVal::Num(-n))),
                        _ => Err(()),
                    },
                    UnaryOp::Plus => Ok(Ok(JsVal::Num(to_number(&v)?))),
                    _ => Err(()),
                }
            }
            Expr::Binary {
                left, op, right, ..
            } => match op {
                BinaryOp::And => {
                    let l = match self.eval_expr(left)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    if !is_truthy(&l) {
                        return Ok(Ok(l));
                    }
                    self.eval_expr(right)
                }
                BinaryOp::Or => {
                    let l = match self.eval_expr(left)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    if is_truthy(&l) {
                        return Ok(Ok(l));
                    }
                    self.eval_expr(right)
                }
                BinaryOp::Comma => {
                    let _ = match self.eval_expr(left)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    self.eval_expr(right)
                }
                BinaryOp::EqEqEq | BinaryOp::EqEq => {
                    let l = match self.eval_expr(left)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    let r = match self.eval_expr(right)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    Ok(Ok(JsVal::Bool(strict_eq(&l, &r))))
                }
                BinaryOp::NotEqEq | BinaryOp::NotEq => {
                    let l = match self.eval_expr(left)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    let r = match self.eval_expr(right)? {
                        Ok(v) => v,
                        Err(f) => return Ok(Err(f)),
                    };
                    Ok(Ok(JsVal::Bool(!strict_eq(&l, &r))))
                }
                _ => Err(()),
            },
            Expr::Conditional {
                test,
                consequent,
                alternate,
                ..
            } => {
                let t = match self.eval_expr(test)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                if is_truthy(&t) {
                    self.eval_expr(consequent)
                } else {
                    self.eval_expr(alternate)
                }
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let v = match self.eval_expr(value)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                self.env.insert(*id, v.clone());
                Ok(Ok(v))
            }
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object, property, ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let v = match self.eval_expr(value)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let obj = match self.eval_expr(object)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let key = match self.eval_key(property)? {
                    Ok(k) => k,
                    Err(f) => return Ok(Err(f)),
                };
                self.object_set(&obj, &key, v.clone())?;
                Ok(Ok(v))
            }
            Expr::Member {
                object,
                property,
                optional: false,
                ..
            } => {
                let obj = match self.eval_expr(object)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let key = match self.eval_key(property)? {
                    Ok(k) => k,
                    Err(f) => return Ok(Err(f)),
                };
                Ok(Ok(self.object_get(&obj, &key)?))
            }
            Expr::Object { properties, .. } => {
                let mut rec = empty_object();
                let mut proto = JsVal::Object(OBJECT_PROTOTYPE_IDX);
                for p in properties {
                    match p {
                        ObjectProp::Property {
                            key: ObjectPropKey::Static(k),
                            value,
                        } => {
                            let key = js_string_to_utf8(k);
                            let v = match self.eval_expr(value)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            if key == "__proto__" {
                                proto = v;
                            } else {
                                object_set_prop(&mut rec, key, v);
                            }
                        }
                        ObjectProp::Property {
                            key: ObjectPropKey::Computed(ke),
                            value,
                        } => {
                            let key = match self.eval_key(ke)? {
                                Ok(k) => k,
                                Err(f) => return Ok(Err(f)),
                            };
                            let v = match self.eval_expr(value)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            object_set_prop(&mut rec, key, v);
                        }
                        _ => return Err(()),
                    }
                }
                rec.proto = proto;
                let idx = self.objects.len();
                self.objects.push(rec);
                Ok(Ok(JsVal::Object(idx)))
            }
            Expr::Array { elements, .. } => {
                let mut rec = empty_object();
                let mut len = 0usize;
                for el in elements {
                    match el {
                        ArrayElement::Expr(e) => {
                            let v = match self.eval_expr(e)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            object_set_prop(&mut rec, len.to_string(), v);
                            len += 1;
                        }
                        ArrayElement::Elision => len += 1,
                        ArrayElement::Spread(_) => return Err(()),
                    }
                }
                object_set_prop(&mut rec, "length".into(), JsVal::Num(len as f64));
                let idx = self.objects.len();
                self.objects.push(rec);
                Ok(Ok(JsVal::Object(idx)))
            }
            Expr::New { callee, args, .. } => {
                let c = match self.eval_expr(callee)? {
                    Ok(v) => v,
                    Err(f) => return Ok(Err(f)),
                };
                let mut argv = Vec::new();
                for a in args {
                    match a {
                        Arg::Expr(e) => match self.eval_expr(e)? {
                            Ok(v) => argv.push(v),
                            Err(f) => return Ok(Err(f)),
                        },
                        Arg::Spread(e) => {
                            let arr = match self.eval_expr(e)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            argv.extend(self.array_to_vec(&arr)?);
                        }
                    }
                }
                match self.construct(&c, &argv, &c) {
                    Ok(v) => Ok(Ok(v)),
                    Err(()) => Err(()),
                }
            }
            Expr::Call {
                callee,
                args,
                optional: false,
                ..
            } => {
                let (func, this_arg) = match callee.as_ref() {
                    Expr::Member {
                        object,
                        property,
                        optional: false,
                        ..
                    } => {
                        let obj = match self.eval_expr(object)? {
                            Ok(v) => v,
                            Err(f) => return Ok(Err(f)),
                        };
                        let key = match self.eval_key(property)? {
                            Ok(k) => k,
                            Err(f) => return Ok(Err(f)),
                        };
                        let f = self.object_get(&obj, &key)?;
                        (f, obj)
                    }
                    _ => {
                        let f = match self.eval_expr(callee)? {
                            Ok(v) => v,
                            Err(f) => return Ok(Err(f)),
                        };
                        (f, JsVal::Undef)
                    }
                };
                let mut argv = Vec::new();
                for a in args {
                    match a {
                        Arg::Expr(e) => match self.eval_expr(e)? {
                            Ok(v) => argv.push(v),
                            Err(f) => return Ok(Err(f)),
                        },
                        Arg::Spread(e) => {
                            let arr = match self.eval_expr(e)? {
                                Ok(v) => v,
                                Err(f) => return Ok(Err(f)),
                            };
                            argv.extend(self.array_to_vec(&arr)?);
                        }
                    }
                }
                match self.call(&func, this_arg, &argv) {
                    Ok(v) => Ok(Ok(v)),
                    Err(()) => Err(()),
                }
            }
            _ => Err(()),
        }
    }

    fn eval_key(&mut self, expr: &Expr) -> Result<Result<String, Flow>, ()> {
        match expr {
            Expr::String { value, .. } => Ok(Ok(js_string_to_utf8(value))),
            e => match self.eval_expr(e)? {
                Ok(JsVal::Str(s)) => Ok(Ok(s)),
                Ok(JsVal::Num(n)) => Ok(Ok(format!("{}", n as i64))),
                Ok(_) => Err(()),
                Err(f) => Ok(Err(f)),
            },
        }
    }

    fn array_to_vec(&self, v: &JsVal) -> Result<Vec<JsVal>, ()> {
        let JsVal::Object(idx) = v else {
            return Err(());
        };
        let rec = self.objects.get(*idx).ok_or(())?;
        let len = match rec.props.get("length") {
            Some(JsVal::Num(n)) => *n as usize,
            _ => 0,
        };
        let mut out = Vec::new();
        for i in 0..len {
            out.push(
                rec.props
                    .get(&i.to_string())
                    .cloned()
                    .unwrap_or(JsVal::Undef),
            );
        }
        Ok(out)
    }

    fn object_id(v: &JsVal) -> Option<usize> {
        match v {
            JsVal::Object(i) => Some(*i),
            JsVal::Fn { obj_idx, .. } => Some(*obj_idx),
            JsVal::Proxy(i) => Some(10_000 + *i), // distinct namespace
            _ => None,
        }
    }

    fn object_get(&mut self, obj: &JsVal, key: &str) -> Result<JsVal, ()> {
        match obj {
            JsVal::Builtin(Builtin::Object) => match key {
                "defineProperty" => Ok(JsVal::Builtin(Builtin::ObjectDefineProperty)),
                "getOwnPropertyDescriptor" => {
                    Ok(JsVal::Builtin(Builtin::ObjectGetOwnPropertyDescriptor))
                }
                "isExtensible" => Ok(JsVal::Builtin(Builtin::ObjectIsExtensible)),
                "setPrototypeOf" => Ok(JsVal::Builtin(Builtin::ObjectSetPrototypeOf)),
                "prototype" => Ok(JsVal::Object(OBJECT_PROTOTYPE_IDX)),
                _ => Ok(JsVal::Undef),
            },
            JsVal::Builtin(Builtin::Function) => match key {
                "prototype" => Ok(JsVal::Object(FUNCTION_PROTOTYPE_IDX)),
                _ => Ok(JsVal::Undef),
            },
            JsVal::Builtin(Builtin::Reflect) => match key {
                "construct" => Ok(JsVal::Builtin(Builtin::ReflectConstruct)),
                "get" => Ok(JsVal::Builtin(Builtin::ReflectGet)),
                _ => Ok(JsVal::Undef),
            },
            JsVal::WeakMap(_) => match key {
                "set" | "get" | "has" | "delete" => Ok(JsVal::Str(format!("__wm_{key}"))),
                _ => Ok(JsVal::Undef),
            },
            JsVal::WeakSet(_) => match key {
                "add" | "has" | "delete" => Ok(JsVal::Str(format!("__ws_{key}"))),
                _ => Ok(JsVal::Undef),
            },
            JsVal::Fn { obj_idx, fn_idx } => {
                // Own data props (e.g. static method named `call`) win over
                // Function.prototype.call.
                if let Some(v) = self.objects.get(*obj_idx).and_then(|o| o.props.get(key)) {
                    return Ok(v.clone());
                }
                if key == "call" {
                    return Ok(JsVal::Str(format!("__fn_call_{fn_idx}")));
                }
                // Walk [[Prototype]] of the function object.
                let proto = self
                    .objects
                    .get(*obj_idx)
                    .map(|o| o.proto.clone())
                    .unwrap_or(JsVal::Null);
                if matches!(proto, JsVal::Null) {
                    return Ok(JsVal::Undef);
                }
                self.object_get(&proto, key)
            }
            JsVal::Object(idx) => {
                let mut cur = *idx;
                loop {
                    let rec = self.objects.get(cur).ok_or(())?;
                    if let Some(v) = rec.props.get(key) {
                        return Ok(v.clone());
                    }
                    match &rec.proto {
                        JsVal::Object(p) => cur = *p,
                        JsVal::Null => return Ok(JsVal::Undef),
                        JsVal::Fn { obj_idx, .. } => cur = *obj_idx,
                        _ => return Ok(JsVal::Undef),
                    }
                    if cur == OBJECT_PROTOTYPE_IDX && !self.objects[cur].props.contains_key(key) {
                        // also check Function.prototype for call
                        if key == "call" {
                            // not on plain objects
                        }
                        return Ok(JsVal::Undef);
                    }
                }
            }
            JsVal::Proxy(idx) => {
                let rec = self.proxies.get(*idx).ok_or(())?.clone();
                if let Some(trap) = rec.get_trap {
                    let args = vec![
                        rec.target.clone(),
                        JsVal::Str(key.to_string()),
                        JsVal::Proxy(*idx),
                    ];
                    return self.call_fn_idx(trap, JsVal::Undef, &args);
                }
                self.object_get(&rec.target, key)
            }
            _ => Ok(JsVal::Undef),
        }
    }

    fn object_set(&mut self, obj: &JsVal, key: &str, val: JsVal) -> Result<(), ()> {
        let idx = match obj {
            JsVal::Object(i) => *i,
            JsVal::Fn { obj_idx, .. } => *obj_idx,
            _ => return Err(()),
        };
        let rec = self.objects.get_mut(idx).ok_or(())?;
        object_set_prop(rec, key.to_string(), val);
        Ok(())
    }

    fn object_delete(&mut self, obj: &JsVal, key: &str) -> Result<bool, ()> {
        let idx = match obj {
            JsVal::Object(i) => *i,
            JsVal::Fn { obj_idx, .. } => *obj_idx,
            _ => return Ok(true),
        };
        let rec = self.objects.get_mut(idx).ok_or(())?;
        if rec.props.remove(key).is_some() {
            rec.keys.retain(|k| k != key);
            Ok(true)
        } else {
            Ok(true)
        }
    }

    fn construct(
        &mut self,
        callee: &JsVal,
        args: &[JsVal],
        new_target: &JsVal,
    ) -> Result<JsVal, ()> {
        match callee {
            JsVal::Builtin(Builtin::WeakMap) => {
                let idx = self.weak_maps.len();
                self.weak_maps.push(WeakMapRec {
                    entries: Vec::new(),
                });
                Ok(JsVal::WeakMap(idx))
            }
            JsVal::Builtin(Builtin::WeakSet) => {
                let idx = self.weak_sets.len();
                self.weak_sets.push(WeakSetRec { keys: Vec::new() });
                Ok(JsVal::WeakSet(idx))
            }
            JsVal::Builtin(Builtin::TypeError) | JsVal::Builtin(Builtin::ReferenceError) => {
                let msg = match args.first() {
                    Some(JsVal::Str(s)) => s.clone(),
                    _ => String::new(),
                };
                let mut rec = empty_object();
                object_set_prop(&mut rec, "message".into(), JsVal::Str(msg));
                let idx = self.objects.len();
                self.objects.push(rec);
                Ok(JsVal::Object(idx))
            }
            JsVal::Builtin(Builtin::Proxy) => {
                if args.len() < 2 {
                    return Err(());
                }
                let target = args[0].clone();
                let handler = &args[1];
                let get_trap = match self.object_get(handler, "get")? {
                    JsVal::Fn { fn_idx, .. } => Some(fn_idx),
                    JsVal::Undef => None,
                    _ => None,
                };
                let idx = self.proxies.len();
                self.proxies.push(ProxyRec { target, get_trap });
                Ok(JsVal::Proxy(idx))
            }
            JsVal::Fn { fn_idx, .. } => {
                // [[Prototype]] of instance = newTarget.prototype
                let proto = self.object_get(new_target, "prototype")?;
                let mut rec = empty_object();
                rec.proto = match proto {
                    JsVal::Object(_) | JsVal::Fn { .. } | JsVal::Null => proto,
                    _ => JsVal::Object(OBJECT_PROTOTYPE_IDX),
                };
                let this_idx = self.objects.len();
                self.objects.push(rec);
                let this_obj = JsVal::Object(this_idx);
                let ret =
                    self.call_fn_idx_new(*fn_idx, this_obj.clone(), args, new_target.clone())?;
                match ret {
                    JsVal::Object(_) | JsVal::Fn { .. } | JsVal::Proxy(_) => Ok(ret),
                    JsVal::Undef => Ok(this_obj),
                    _ => Ok(this_obj),
                }
            }
            JsVal::Proxy(idx) => {
                let rec = self.proxies.get(*idx).ok_or(())?.clone();
                self.construct(&rec.target, args, new_target)
            }
            _ => Err(()),
        }
    }

    fn call(&mut self, func: &JsVal, this_arg: JsVal, args: &[JsVal]) -> Result<JsVal, ()> {
        // WeakMap/WeakSet methods encoded as magic strings
        if let JsVal::Str(s) = func {
            if let Some(method) = s.strip_prefix("__wm_") {
                return self.weak_map_method(&this_arg, method, args);
            }
            if let Some(method) = s.strip_prefix("__ws_") {
                return self.weak_set_method(&this_arg, method, args);
            }
            if let Some(rest) = s.strip_prefix("__fn_call_") {
                let fn_idx: usize = rest.parse().map_err(|_| ())?;
                let this = args.first().cloned().unwrap_or(JsVal::Undef);
                let rest_args: Vec<JsVal> = args.iter().skip(1).cloned().collect();
                return self.call_fn_idx(fn_idx, this, &rest_args);
            }
        }
        match func {
            JsVal::Fn { fn_idx, .. } => self.call_fn_idx(*fn_idx, this_arg, args),
            JsVal::Builtin(Builtin::ObjectDefineProperty) => {
                if args.len() < 3 {
                    return Err(());
                }
                let key = match &args[1] {
                    JsVal::Str(s) => s.clone(),
                    JsVal::Num(n) => format!("{}", *n as i64),
                    _ => return Err(()),
                };
                let value = self.descriptor_value(&args[2])?;
                self.object_set(&args[0], &key, value)?;
                Ok(args[0].clone())
            }
            JsVal::Builtin(Builtin::ObjectGetOwnPropertyDescriptor) => {
                if args.len() < 2 {
                    return Err(());
                }
                let key = match &args[1] {
                    JsVal::Str(s) => s.clone(),
                    JsVal::Num(n) => format!("{}", *n as i64),
                    _ => return Err(()),
                };
                let val = match &args[0] {
                    JsVal::Object(i) | JsVal::Fn { obj_idx: i, .. } => {
                        self.objects.get(*i).ok_or(())?.props.get(&key).cloned()
                    }
                    _ => None,
                };
                match val {
                    Some(v) => Ok(self.make_data_descriptor(v)),
                    None => Ok(JsVal::Undef),
                }
            }
            JsVal::Builtin(Builtin::ObjectIsExtensible) => {
                let obj = args.first().ok_or(())?;
                let ext = match obj {
                    JsVal::Object(i) | JsVal::Fn { obj_idx: i, .. } => {
                        self.objects.get(*i).ok_or(())?.extensible
                    }
                    _ => true,
                };
                Ok(JsVal::Bool(ext))
            }
            JsVal::Builtin(Builtin::ObjectSetPrototypeOf) => {
                if args.len() < 2 {
                    return Err(());
                }
                let idx = match &args[0] {
                    JsVal::Object(i) | JsVal::Fn { obj_idx: i, .. } => *i,
                    _ => return Err(()),
                };
                self.objects.get_mut(idx).ok_or(())?.proto = args[1].clone();
                Ok(args[0].clone())
            }
            JsVal::Builtin(Builtin::ReflectConstruct) => {
                if args.len() < 2 {
                    return Err(());
                }
                let argv = self.array_to_vec(&args[1])?;
                let nt = if args.len() >= 3 {
                    args[2].clone()
                } else {
                    args[0].clone()
                };
                self.construct(&args[0], &argv, &nt)
            }
            JsVal::Builtin(Builtin::ReflectGet) => {
                if args.len() < 2 {
                    return Err(());
                }
                let key = match &args[1] {
                    JsVal::Str(s) => s.clone(),
                    _ => return Err(()),
                };
                self.object_get(&args[0], &key)
            }
            JsVal::Builtin(Builtin::TypeError) | JsVal::Builtin(Builtin::ReferenceError) => {
                // called without new — still make error object
                self.construct(func, args, func)
            }
            _ => Err(()),
        }
    }

    fn weak_map_method(
        &mut self,
        this_arg: &JsVal,
        method: &str,
        args: &[JsVal],
    ) -> Result<JsVal, ()> {
        let JsVal::WeakMap(idx) = this_arg else {
            return Err(());
        };
        let idx = *idx;
        match method {
            "set" => {
                let k = args.first().ok_or(())?;
                let kid = Self::object_id(k).ok_or(())?;
                let v = args.get(1).cloned().unwrap_or(JsVal::Undef);
                let wm = self.weak_maps.get_mut(idx).ok_or(())?;
                if let Some((_, slot)) = wm.entries.iter_mut().find(|(id, _)| *id == kid) {
                    *slot = v;
                } else {
                    wm.entries.push((kid, v));
                }
                Ok(this_arg.clone())
            }
            "get" => {
                let k = args.first().ok_or(())?;
                let Some(kid) = Self::object_id(k) else {
                    return Ok(JsVal::Undef);
                };
                let wm = self.weak_maps.get(idx).ok_or(())?;
                Ok(wm
                    .entries
                    .iter()
                    .find(|(id, _)| *id == kid)
                    .map(|(_, v)| v.clone())
                    .unwrap_or(JsVal::Undef))
            }
            "has" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                let Some(kid) = Self::object_id(k) else {
                    return Ok(JsVal::Bool(false));
                };
                let wm = self.weak_maps.get(idx).ok_or(())?;
                Ok(JsVal::Bool(wm.entries.iter().any(|(id, _)| *id == kid)))
            }
            "delete" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                let Some(kid) = Self::object_id(k) else {
                    return Ok(JsVal::Bool(false));
                };
                let wm = self.weak_maps.get_mut(idx).ok_or(())?;
                let before = wm.entries.len();
                wm.entries.retain(|(id, _)| *id != kid);
                Ok(JsVal::Bool(wm.entries.len() < before))
            }
            _ => Err(()),
        }
    }

    fn weak_set_method(
        &mut self,
        this_arg: &JsVal,
        method: &str,
        args: &[JsVal],
    ) -> Result<JsVal, ()> {
        let JsVal::WeakSet(idx) = this_arg else {
            return Err(());
        };
        let idx = *idx;
        match method {
            "add" => {
                let k = args.first().ok_or(())?;
                let kid = Self::object_id(k).ok_or(())?;
                let ws = self.weak_sets.get_mut(idx).ok_or(())?;
                if !ws.keys.contains(&kid) {
                    ws.keys.push(kid);
                }
                Ok(this_arg.clone())
            }
            "has" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                let Some(kid) = Self::object_id(k) else {
                    return Ok(JsVal::Bool(false));
                };
                let ws = self.weak_sets.get(idx).ok_or(())?;
                Ok(JsVal::Bool(ws.keys.contains(&kid)))
            }
            "delete" => {
                let k = args.first().unwrap_or(&JsVal::Undef);
                let Some(kid) = Self::object_id(k) else {
                    return Ok(JsVal::Bool(false));
                };
                let ws = self.weak_sets.get_mut(idx).ok_or(())?;
                let before = ws.keys.len();
                ws.keys.retain(|id| *id != kid);
                Ok(JsVal::Bool(ws.keys.len() < before))
            }
            _ => Err(()),
        }
    }

    fn call_fn_idx(&mut self, fn_idx: usize, this_arg: JsVal, args: &[JsVal]) -> Result<JsVal, ()> {
        let is_arrow = self.fns.get(fn_idx).map(|f| f.is_arrow).unwrap_or(false);
        // Arrows inherit outer `new.target`; non-arrows clear it on ordinary call.
        let nt = if is_arrow {
            CURRENT_NEW_TARGET.with(|c| c.borrow().clone())
        } else {
            JsVal::Undef
        };
        self.call_fn_idx_new(fn_idx, this_arg, args, nt)
    }

    fn call_fn_idx_new(
        &mut self,
        fn_idx: usize,
        this_arg: JsVal,
        args: &[JsVal],
        new_target: JsVal,
    ) -> Result<JsVal, ()> {
        let rec = self.fns.get(fn_idx).ok_or(())?.clone();
        let this_for_body = if rec.is_arrow {
            CURRENT_THIS.with(|c| c.borrow().clone())
        } else {
            this_arg
        };
        let mut saved_local: Vec<(LocalId, Option<JsVal>)> = Vec::new();
        let mut saved_name: Vec<(String, Option<JsVal>)> = Vec::new();
        let mut ai = 0usize;
        for p in &rec.params {
            let val = if p.rest {
                let rest: Vec<JsVal> = args.get(ai..).unwrap_or(&[]).to_vec();
                let mut arr = empty_object();
                for (i, v) in rest.iter().enumerate() {
                    object_set_prop(&mut arr, i.to_string(), v.clone());
                }
                object_set_prop(&mut arr, "length".into(), JsVal::Num(rest.len() as f64));
                let idx = self.objects.len();
                self.objects.push(arr);
                ai = args.len();
                JsVal::Object(idx)
            } else {
                let v = args.get(ai).cloned().unwrap_or(JsVal::Undef);
                ai += 1;
                v
            };
            match &p.bind {
                ParamBind::Local(id) => {
                    saved_local.push((*id, self.env.get(id).cloned()));
                    self.env.insert(*id, val);
                }
                ParamBind::Name(n) => {
                    saved_name.push((n.clone(), self.name_env.get(n).cloned()));
                    self.name_env.insert(n.clone(), val);
                }
            }
        }
        let flow = with_this_new(this_for_body, new_target, || self.eval_body(&rec.body))?;
        for (id, prev) in saved_local {
            match prev {
                Some(v) => {
                    self.env.insert(id, v);
                }
                None => {
                    self.env.remove(&id);
                }
            }
        }
        for (n, prev) in saved_name {
            match prev {
                Some(v) => {
                    self.name_env.insert(n, v);
                }
                None => {
                    self.name_env.remove(&n);
                }
            }
        }
        match flow {
            Flow::Normal => Ok(JsVal::Undef),
            Flow::Return(v) => Ok(v),
            Flow::Throw(_) => Err(()),
        }
    }

    fn descriptor_value(&self, desc: &JsVal) -> Result<JsVal, ()> {
        match desc {
            JsVal::Object(idx) => Ok(self
                .objects
                .get(*idx)
                .ok_or(())?
                .props
                .get("value")
                .cloned()
                .unwrap_or(JsVal::Undef)),
            JsVal::Fn { .. } => Ok(desc.clone()),
            _ => Ok(desc.clone()),
        }
    }

    fn make_data_descriptor(&mut self, value: JsVal) -> JsVal {
        let mut rec = empty_object();
        object_set_prop(&mut rec, "value".into(), value);
        object_set_prop(&mut rec, "writable".into(), JsVal::Bool(true));
        object_set_prop(&mut rec, "enumerable".into(), JsVal::Bool(true));
        object_set_prop(&mut rec, "configurable".into(), JsVal::Bool(true));
        let idx = self.objects.len();
        self.objects.push(rec);
        JsVal::Object(idx)
    }
}

fn with_this_new<R>(this: JsVal, new_target: JsVal, f: impl FnOnce() -> R) -> R {
    CURRENT_THIS.with(|t| {
        CURRENT_NEW_TARGET.with(|n| {
            let pt = t.replace(this);
            let pn = n.replace(new_target);
            let r = f();
            *t.borrow_mut() = pt;
            *n.borrow_mut() = pn;
            r
        })
    })
}

fn builtin_for_name(name: &str) -> Option<Builtin> {
    match name {
        "Object" => Some(Builtin::Object),
        "Function" => Some(Builtin::Function),
        "TypeError" => Some(Builtin::TypeError),
        "ReferenceError" => Some(Builtin::ReferenceError),
        "undefined" => Some(Builtin::Undefined),
        "Reflect" => Some(Builtin::Reflect),
        "Proxy" => Some(Builtin::Proxy),
        "WeakMap" => Some(Builtin::WeakMap),
        "WeakSet" => Some(Builtin::WeakSet),
        _ => None,
    }
}

fn empty_object() -> ObjectRec {
    ObjectRec {
        props: HashMap::new(),
        keys: Vec::new(),
        proto: JsVal::Object(OBJECT_PROTOTYPE_IDX),
        extensible: true,
    }
}

fn object_set_prop(rec: &mut ObjectRec, key: String, value: JsVal) {
    if !rec.props.contains_key(&key) {
        rec.keys.push(key.clone());
    }
    rec.props.insert(key, value);
}

fn is_truthy(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Undef | JsVal::Null => false,
        _ => true,
    }
}

fn to_number(v: &JsVal) -> Result<f64, ()> {
    match v {
        JsVal::Num(n) => Ok(*n),
        JsVal::Bool(true) => Ok(1.0),
        JsVal::Bool(false) => Ok(0.0),
        JsVal::Null => Ok(0.0),
        JsVal::Undef => Ok(f64::NAN),
        JsVal::Str(s) => Ok(s.parse().unwrap_or(f64::NAN)),
        _ => Err(()),
    }
}

fn typeof_str(v: &JsVal) -> String {
    match v {
        JsVal::Num(_) => "number".into(),
        JsVal::Bool(_) => "boolean".into(),
        JsVal::Str(_) => "string".into(),
        JsVal::Undef | JsVal::Builtin(Builtin::Undefined) => "undefined".into(),
        JsVal::Fn { .. }
        | JsVal::Builtin(
            Builtin::Object
            | Builtin::Function
            | Builtin::TypeError
            | Builtin::ReferenceError
            | Builtin::WeakMap
            | Builtin::WeakSet
            | Builtin::Proxy,
        ) => "function".into(),
        JsVal::Null
        | JsVal::Object(_)
        | JsVal::WeakMap(_)
        | JsVal::WeakSet(_)
        | JsVal::Proxy(_)
        | JsVal::Builtin(_) => "object".into(),
    }
}

fn strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => {
            if x.is_nan() && y.is_nan() {
                false
            } else {
                x == y
            }
        }
        (JsVal::Bool(x), JsVal::Bool(y)) => x == y,
        (JsVal::Str(x), JsVal::Str(y)) => x == y,
        (JsVal::Undef, JsVal::Undef)
        | (JsVal::Undef, JsVal::Builtin(Builtin::Undefined))
        | (JsVal::Builtin(Builtin::Undefined), JsVal::Undef)
        | (JsVal::Builtin(Builtin::Undefined), JsVal::Builtin(Builtin::Undefined)) => true,
        (JsVal::Null, JsVal::Null) => true,
        (JsVal::Object(x), JsVal::Object(y)) => x == y,
        (JsVal::Fn { obj_idx: x, .. }, JsVal::Fn { obj_idx: y, .. }) => x == y,
        (JsVal::WeakMap(x), JsVal::WeakMap(y)) => x == y,
        (JsVal::WeakSet(x), JsVal::WeakSet(y)) => x == y,
        (JsVal::Proxy(x), JsVal::Proxy(y)) => x == y,
        (JsVal::Builtin(x), JsVal::Builtin(y)) => x == y,
        _ => false,
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
        if let Some((_, name)) = self.str_consts.iter().find(|(c, _)| c == s) {
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
                .ok_or_else(|| diag("es_private_in: missing value"))?;
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
                _ => return Err(diag("es_private_in: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.41 private brand check #x in obj)"
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

    #[test]
    fn private_in_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/annex-b/private_in.drac");
        let m = compile_source(src).expect("compile");
        assert!(
            is_es_private_in_module(&m),
            "should classify as es_private_in"
        );
        let ir = emit_es_private_in(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("true") && ir.contains("false"),
            "should print boolean observations:\n{ir}"
        );
    }
}
