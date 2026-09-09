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
mod eval;
mod call;


pub(crate) fn is_es_private_in_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_private_in(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_private_in module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_private_in(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_private_in_module(module) {
        return None;
    }
    Some(emit_es_private_in(module))
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
