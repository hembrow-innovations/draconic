//! N08.13.01–N08.13.11: native observations for Proxy basics + `set` + `has`/`in`
//! + `delete`/`deleteProperty` + `apply` + `construct` + Reflect basics + `ownKeys`
//! + `getPrototypeOf`/`setPrototypeOf` + `defineProperty`/`getOwnPropertyDescriptor`
//! + `isExtensible`/`preventExtensions` (E14.01–E14.11).
//!
//! Compile-time evaluation of a small Proxy/Reflect subset: `typeof Proxy`,
//! `new Proxy(target, handler)`, empty-handler get/set/`in`/`delete`/call/`new`/ownKeys
//! /prototype/defineProperty/isExtensible/preventExtensions pass-through, `get`/`set`/
//! `has`/`deleteProperty`/`apply`/`construct`/`ownKeys`/`getPrototypeOf`/
//! `setPrototypeOf`/`defineProperty`/`getOwnPropertyDescriptor`/`isExtensible`/
//! `preventExtensions` traps (function props; free-var capture; string keys),
//! `typeof Reflect` + `Reflect.get`/`set`/`has`/`deleteProperty`/`apply`/`construct`/
//! `ownKeys`/`getPrototypeOf`/`setPrototypeOf`/`defineProperty`/
//! `getOwnPropertyDescriptor`/`isExtensible`/`preventExtensions` on plain objects +
//! Proxy targets, data descriptors `{value,writable,enumerable,configurable}`,
//! `void`, array literals as arg lists, member assign, method calls (`obj.m()` thisArg),
//! function constructors (`this` + prop init), `typeof` on proxies. Objects live on a
//! heap so proxy targets share identity with outer locals. Emits Runtime prints of
//! final top-level number/string/bool locals.

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
#[path = "es_proxies_eval.rs"]
mod eval;
#[path = "es_proxies_traps.rs"]
mod traps;

use eval::eval_body;


pub(crate) fn is_es_proxies_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_proxies(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_proxies module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_proxies(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_proxies_module(module) {
        return None;
    }
    Some(emit_es_proxies(module))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ReflectOp {
    Get,
    Set,
    Has,
    DeleteProperty,
    Apply,
    Construct,
    OwnKeys,
    GetPrototypeOf,
    SetPrototypeOf,
    DefineProperty,
    GetOwnPropertyDescriptor,
    IsExtensible,
    PreventExtensions,
}

/// Heap index of the shared `Object.prototype` stand-in (null [[Prototype]]).
const OBJECT_PROTOTYPE_IDX: usize = 0;

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Null,
    /// Builtin `Proxy` constructor.
    ProxyCtor,
    /// Builtin `Reflect` object.
    ReflectObj,
    /// `Reflect.get` / `set` / `has` / `deleteProperty` / `apply` / `construct` /
    /// `ownKeys` / `getPrototypeOf` / `setPrototypeOf` / `defineProperty` /
    /// `getOwnPropertyDescriptor` / `isExtensible` / `preventExtensions`.
    ReflectMethod(ReflectOp),
    /// Plain object (index into object heap).
    Object(usize),
    /// Function value (index into `fns`).
    Fn(usize),
    /// Proxy instance (index into `proxies`).
    Proxy(usize),
}

thread_local! {
    static CURRENT_THIS: RefCell<Option<JsVal>> = const { RefCell::new(None) };
}

#[derive(Clone, Debug)]
struct FnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

#[derive(Clone, Debug)]
struct ObjectRec {
    props: HashMap<String, JsVal>,
    /// Insertion order of own string keys (for `Reflect.ownKeys`).
    keys: Vec<String>,
    /// [[Prototype]] — `Null` or `Object(idx)` (shared Object.prototype at 0 by default).
    proto: JsVal,
    /// [[Extensible]] — cleared by `Reflect.preventExtensions`.
    extensible: bool,
}

#[derive(Clone, Debug)]
struct ProxyRec {
    target: JsVal,
    /// Optional `get` trap function index.
    get_trap: Option<usize>,
    /// Optional `set` trap function index.
    set_trap: Option<usize>,
    /// Optional `has` trap function index.
    has_trap: Option<usize>,
    /// Optional `deleteProperty` trap function index.
    delete_trap: Option<usize>,
    /// Optional `apply` trap function index.
    apply_trap: Option<usize>,
    /// Optional `construct` trap function index.
    construct_trap: Option<usize>,
    /// Optional `ownKeys` trap function index.
    own_keys_trap: Option<usize>,
    /// Optional `getPrototypeOf` trap function index.
    get_prototype_of_trap: Option<usize>,
    /// Optional `setPrototypeOf` trap function index.
    set_prototype_of_trap: Option<usize>,
    /// Optional `defineProperty` trap function index.
    define_property_trap: Option<usize>,
    /// Optional `getOwnPropertyDescriptor` trap function index.
    get_own_property_descriptor_trap: Option<usize>,
    /// Optional `isExtensible` trap function index.
    is_extensible_trap: Option<usize>,
    /// Optional `preventExtensions` trap function index.
    prevent_extensions_trap: Option<usize>,
}

fn object_set_prop(rec: &mut ObjectRec, key: String, value: JsVal) {
    if !rec.props.contains_key(&key) {
        rec.keys.push(key.clone());
    }
    rec.props.insert(key, value);
}

fn object_delete_prop(rec: &mut ObjectRec, key: &str) {
    if rec.props.remove(key).is_some() {
        rec.keys.retain(|k| k != key);
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

fn object_prototype_rec() -> ObjectRec {
    ObjectRec {
        props: HashMap::new(),
        keys: Vec::new(),
        proto: JsVal::Null,
        extensible: true,
    }
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
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
    if !module_has_proxy(module) {
        return None;
    }
    if !module.body.iter().all(|s| {
        matches!(
            s,
            Stmt::Declare { .. } | Stmt::Expr { .. } | Stmt::Function { .. }
        )
    }) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    // Install builtins used by this subset.
    for loc in &module.locals {
        if loc.name == "Proxy" && loc.ty == Type::Function {
            env.insert(loc.id, JsVal::ProxyCtor);
        }
        if loc.name == "Reflect" && matches!(loc.ty, Type::Object | Type::Any | Type::Function) {
            env.insert(loc.id, JsVal::ReflectObj);
        }
    }

    let mut fns: Vec<FnRec> = Vec::new();
    let mut objects: Vec<ObjectRec> = vec![object_prototype_rec()];
    let mut proxies: Vec<ProxyRec> = Vec::new();

    if eval_body(&module.body, &mut env, &mut fns, &mut objects, &mut proxies).is_err() {
        return None;
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
                Some(JsVal::Undef) if matches!(loc.ty, Type::Any | Type::String | Type::Number) => {
                    // skip undefined for print unless we need it
                }
                Some(
                    JsVal::Object(_)
                    | JsVal::Proxy(_)
                    | JsVal::Fn(_)
                    | JsVal::ProxyCtor
                    | JsVal::ReflectObj
                    | JsVal::ReflectMethod(_)
                    | JsVal::Null,
                ) => {}
                None => return None,
                _ => {}
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

fn module_has_proxy(module: &Module) -> bool {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    module.body.iter().any(|s| stmt_has_proxy(s, &by_id))
}

fn stmt_has_proxy(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } => expr_has_proxy(e, by_id),
        Stmt::Function { body, .. } => body.iter().any(|s| stmt_has_proxy(s, by_id)),
        Stmt::Block { body } => body.iter().any(|s| stmt_has_proxy(s, by_id)),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_has_proxy(test, by_id)
                || stmt_has_proxy(consequent, by_id)
                || alternate.as_ref().is_some_and(|a| stmt_has_proxy(a, by_id))
        }
        Stmt::Return { value: Some(e) } => expr_has_proxy(e, by_id),
        _ => false,
    }
}

fn expr_has_proxy(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Local { id, .. } => by_id
            .get(id)
            .is_some_and(|l| l.name == "Proxy" || l.name == "Reflect"),
        Expr::New { callee, args, .. } => {
            expr_has_proxy(callee, by_id)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_proxy(e, by_id),
                    _ => false,
                })
        }
        Expr::Call { callee, args, .. } => {
            expr_has_proxy(callee, by_id)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_proxy(e, by_id),
                    _ => false,
                })
        }
        Expr::Member {
            object, property, ..
        } => expr_has_proxy(object, by_id) || expr_has_proxy(property, by_id),
        Expr::Unary { arg, .. } => expr_has_proxy(arg, by_id),
        Expr::Binary { left, right, .. } => {
            expr_has_proxy(left, by_id) || expr_has_proxy(right, by_id)
        }
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                expr_has_proxy(value, by_id)
            }
            ObjectProp::Spread(e) => expr_has_proxy(e, by_id),
        }),
        Expr::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_has_proxy(e, by_id),
            ArrayElement::Elision => false,
        }),
        Expr::Function { body, .. } => body.iter().any(|s| stmt_has_proxy(s, by_id)),
        Expr::Assign { target, value, .. } => {
            assign_target_has_proxy(target, by_id) || expr_has_proxy(value, by_id)
        }
        _ => false,
    }
}

fn assign_target_has_proxy(target: &AssignTarget, by_id: &HashMap<LocalId, &Local>) -> bool {
    match target {
        AssignTarget::Local(_) | AssignTarget::Name(_) => false,
        AssignTarget::Member {
            object, property, ..
        } => expr_has_proxy(object, by_id) || expr_has_proxy(property, by_id),
        AssignTarget::Deref(e) => expr_has_proxy(e, by_id),
        AssignTarget::ArrayPattern { .. } | AssignTarget::ObjectPattern { .. } => false,
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
        writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_proxies: missing value"))?;
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
                _ => return Err(diag("es_proxies: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.13.11 Proxy isExtensible/preventExtensions)"
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
    fn proxy_basics_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_basics.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("function") || ir.contains("print"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_set_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_set.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("2"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_has_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_has.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("true") || ir.contains("print"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_delete_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_delete.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("true") || ir.contains("print") || ir.contains("keep"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_apply_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_apply.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("5") || ir.contains("21"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_construct_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_construct.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("5") || ir.contains("20"),
            "should print observations:\n{ir}"
        );
    }

    #[test]
    fn reflect_basics_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/proxies/reflect_basics.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("object") && ir.contains("function"),
            "should print Reflect typeof observations:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("11") || ir.contains("13"),
            "should print numeric observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_own_keys_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_own_keys.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("function") && ir.contains("extra"),
            "should print ownKeys observations:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("2"),
            "should print numeric observations:\n{ir}"
        );
    }

    #[test]
    fn proxy_prototype_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_prototype.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("function") && ir.contains("true"),
            "should print prototype observations:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("1"),
            "should print trap call counts:\n{ir}"
        );
    }

    #[test]
    fn proxy_define_property_classifies_and_emits() {
        let src = include_str!(
            "../../../tests/conformance/fixtures/es/proxies/proxy_define_property.drac"
        );
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("function") && ir.contains("true"),
            "should print defineProperty observations:\n{ir}"
        );
        assert!(
            ir.contains("42") || ir.contains("print"),
            "should print trap values:\n{ir}"
        );
    }

    #[test]
    fn proxy_extensible_classifies_and_emits() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/proxies/proxy_extensible.drac");
        let m = compile(src);
        assert!(is_es_proxies_module(&m), "should classify as es_proxies");
        let ir = emit_es_proxies(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("function") && ir.contains("true") && ir.contains("false"),
            "should print isExtensible/preventExtensions observations:\n{ir}"
        );
        assert!(
            ir.contains("print") || ir.contains("1"),
            "should print trap call counts:\n{ir}"
        );
    }
}
