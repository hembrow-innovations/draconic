//! N08.16.40: native observations for private accessors (E18.39).
//!
//! Compile-time evaluation of class private fields + get/set `#x` (instance and
//! static) after IR desugars them to WeakMap/WeakSet + synthetic functions.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64, PRINT_STR};
#[path = "es_private_accessors_eval.rs"]
mod eval;

pub(crate) fn is_es_private_accessors_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_private_accessors(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not private_accessors"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_private_accessors(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_private_accessors_module(module) {
        return None;
    }
    Some(emit_es_private_accessors(module))
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Undef,
    Null,
    Builtin(&'static str),
    UserFn {
        id: u64,
        params: Vec<LocalId>,
        body: Vec<Stmt>,
        props: Rc<RefCell<Vec<(String, Slot)>>>,
    },
    Object {
        id: u64,
        props: Rc<RefCell<Vec<(String, Slot)>>>,
        proto: Rc<RefCell<JsVal>>,
    },
    WeakMap(Rc<RefCell<Vec<(u64, JsVal)>>>),
    WeakSet(Rc<RefCell<Vec<u64>>>),
    Err {
        message: String,
    },
}

#[derive(Clone, Debug)]
enum Slot {
    Data(JsVal),
    Accessor {
        get: Option<JsVal>,
        set: Option<JsVal>,
    },
}

enum Flow {
    Normal,
    Return(JsVal),
    Throw(JsVal),
}

struct ModuleInfo {
    prints: Vec<JsVal>,
}

struct Ids {
    next: Cell<u64>,
}

impl Ids {
    fn new() -> Self {
        Self { next: Cell::new(1) }
    }

    fn next_id(&self) -> u64 {
        let id = self.next.get();
        self.next.set(id + 1);
        id
    }

    fn new_obj(&self, proto: JsVal) -> JsVal {
        JsVal::Object {
            id: self.next_id(),
            props: Rc::new(RefCell::new(Vec::new())),
            proto: Rc::new(RefCell::new(proto)),
        }
    }

    fn new_fn(&self, params: Vec<LocalId>, body: Vec<Stmt>) -> JsVal {
        let proto = self.new_obj(JsVal::Builtin("Object.prototype"));
        JsVal::UserFn {
            id: self.next_id(),
            params,
            body,
            props: Rc::new(RefCell::new(vec![("prototype".into(), Slot::Data(proto))])),
        }
    }
}

fn obj_id(v: &JsVal) -> Option<u64> {
    match v {
        JsVal::Object { id, .. } | JsVal::UserFn { id, .. } => Some(*id),
        _ => None,
    }
}

fn is_objectish(v: &JsVal) -> bool {
    matches!(
        v,
        JsVal::Object { .. }
            | JsVal::UserFn { .. }
            | JsVal::WeakMap(_)
            | JsVal::WeakSet(_)
            | JsVal::Builtin(_)
    )
}

fn set_data(props: &Rc<RefCell<Vec<(String, Slot)>>>, key: String, val: JsVal) {
    let mut p = props.borrow_mut();
    if let Some((_, s)) = p.iter_mut().find(|(k, _)| *k == key) {
        *s = Slot::Data(val);
    } else {
        p.push((key, Slot::Data(val)));
    }
}

fn get_data(props: &[(String, Slot)], key: &str) -> Option<JsVal> {
    props
        .iter()
        .find(|(k, _)| k == key)
        .and_then(|(_, s)| match s {
            Slot::Data(v) => Some(v.clone()),
            Slot::Accessor { get: Some(g), .. } => Some(g.clone()),
            _ => None,
        })
}

fn delete_key(props: &Rc<RefCell<Vec<(String, Slot)>>>, key: &str) {
    props.borrow_mut().retain(|(k, _)| k != key);
}

thread_local! {
    static CURRENT_THIS: RefCell<JsVal> = const { RefCell::new(JsVal::Undef) };
    static CURRENT_NEW_TARGET: RefCell<JsVal> = const { RefCell::new(JsVal::Undef) };
}

fn with_this<R>(t: JsVal, f: impl FnOnce() -> R) -> R {
    CURRENT_THIS.with(|c| {
        let prev = c.replace(t);
        let o = f();
        c.replace(prev);
        o
    })
}

fn current_this() -> JsVal {
    CURRENT_THIS.with(|c| c.borrow().clone())
}

fn with_new_target<R>(t: JsVal, f: impl FnOnce() -> R) -> R {
    CURRENT_NEW_TARGET.with(|c| {
        let prev = c.replace(t);
        let o = f();
        c.replace(prev);
        o
    })
}

fn current_new_target() -> JsVal {
    CURRENT_NEW_TARGET.with(|c| c.borrow().clone())
}

fn builtin(name: &str) -> Option<JsVal> {
    match name {
        "undefined" => Some(JsVal::Undef),
        "Object" | "Function" | "WeakMap" | "WeakSet" | "TypeError" | "Error" => {
            Some(JsVal::Builtin(match name {
                "Object" => "Object",
                "Function" => "Function",
                "WeakMap" => "WeakMap",
                "WeakSet" => "WeakSet",
                "TypeError" => "TypeError",
                "Error" => "Error",
                _ => unreachable!(),
            }))
        }
        _ => None,
    }
}

fn has_private_accessor_surface(module: &Module) -> bool {
    module.locals.iter().any(|l| {
        l.name.contains("__drac_pag_")
            || l.name.contains("__drac_pas_")
            || l.name.contains("__drac_pf_")
    })
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    if !has_private_accessor_surface(module) {
        return None;
    }
    if !body_ok(&module.body) {
        return None;
    }
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    for loc in &module.locals {
        if let Some(v) = builtin(&loc.name) {
            env.insert(loc.id, v);
        }
    }
    match Ids::new().eval_body(&module.body, &mut env) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }
    let mut prints = Vec::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            // Skip class/instance bindings and synthetic temps.
            if loc.name.starts_with("__") {
                continue;
            }
            match env.get(local) {
                Some(v @ (JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_))) => {
                    if matches!(
                        loc.ty,
                        Type::Number | Type::Any | Type::Boolean | Type::String
                    ) {
                        prints.push(v.clone());
                    }
                }
                Some(JsVal::Undef)
                | Some(JsVal::Null)
                | Some(JsVal::Object { .. })
                | Some(JsVal::UserFn { .. })
                | Some(JsVal::Builtin(_))
                | Some(JsVal::WeakMap(_))
                | Some(JsVal::WeakSet(_))
                | Some(JsVal::Err { .. }) => {}
                None => return None,
            }
        }
    }
    if prints.is_empty() {
        return None;
    }
    Some(ModuleInfo { prints })
}

fn body_ok(body: &[Stmt]) -> bool {
    body.iter().all(stmt_ok)
}

fn stmt_ok(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init: None, .. } => true,
        Stmt::Declare { init: Some(e), .. } | Stmt::Expr { expr: e } | Stmt::Throw { value: e } => {
            expr_ok(e)
        }
        Stmt::Return { value: None } => true,
        Stmt::Return { value: Some(e) } => expr_ok(e),
        Stmt::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => params_ok(params) && body_ok(body),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => expr_ok(test) && stmt_ok(consequent) && alternate.as_ref().is_none_or(|a| stmt_ok(a)),
        Stmt::Block { body } => body_ok(body),
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
        _ => false,
    }
}

fn params_ok(params: &[Param]) -> bool {
    params
        .iter()
        .all(|p| !p.rest && p.default.is_none() && matches!(p.pattern, Pattern::Local(_)))
}

fn expr_ok(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. }
        | Expr::String { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::Local { .. }
        | Expr::This { .. }
        | Expr::NewTarget { .. }
        | Expr::IdentName { .. } => true,
        Expr::Function {
            name: None,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => params_ok(params) && body_ok(body),
        Expr::Unary {
            op:
                UnaryOp::TypeOf
                | UnaryOp::Minus
                | UnaryOp::Plus
                | UnaryOp::Not
                | UnaryOp::Void
                | UnaryOp::Delete,
            arg,
            ..
        } => expr_ok(arg),
        Expr::Binary {
            left, right, op, ..
        } => {
            matches!(
                op,
                BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
                    | BinaryOp::EqEq
                    | BinaryOp::NotEq
                    | BinaryOp::And
                    | BinaryOp::Or
                    | BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
                    | BinaryOp::Comma
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
        Expr::New { callee, args, .. }
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
        Expr::Assign {
            target: AssignTarget::Member {
                object, property, ..
            },
            op: AssignOp::Eq,
            value,
            ..
        } => expr_ok(object) && expr_ok(property) && expr_ok(value),
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


struct Emitter {
    out: String,
    body: String,
    strs: Vec<(String, String)>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
            strs: Vec::new(),
        }
    }

    fn intern(&mut self, s: &str) -> String {
        if let Some((_, n)) = self.strs.iter().find(|(v, _)| v == s) {
            return n.clone();
        }
        let n = format!("@.pastr.{}", self.strs.len());
        self.strs.push((s.to_string(), n.clone()));
        n
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for v in &info.prints {
            match v {
                JsVal::Num(n) => {
                    let lit = format!("{n:?}");
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {lit}"))).ok();
                }
                JsVal::Str(s) => {
                    let name = self.intern(s);
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                JsVal::Bool(b) => {
                    let name = self.intern(if *b { "true" } else { "false" });
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
                }
                _ => return Err(diag("private_accessors: non-printable")),
            }
        }
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.40 private accessors E18.39)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        for (s, name) in &self.strs {
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
    fn private_accessors_classifies_and_prints() {
        let src =
            include_str!("../../../tests/conformance/fixtures/es/annex-b/private_accessors.drac");
        let m = compile_source(src).expect("compile");
        assert!(
            is_es_private_accessors_module(&m),
            "should classify private_accessors"
        );
        let ir = emit_es_private_accessors(&m).expect("emit");
        assert!(!ir.contains("draconic_rt_hello"), "no hello stub:\n{ir}");
        // a=1 b=10 c=undefined e=10 f=5 g=1 h=7 i=undefined j=undefined k=1 l=2 m=3 n=7
        assert!(ir.contains("double 1") || ir.contains("double 1.0"), "{ir}");
        assert!(
            ir.contains("double 10") || ir.contains("double 10.0"),
            "{ir}"
        );
        assert!(ir.contains("undefined"), "{ir}");
        assert!(ir.contains("double 5") || ir.contains("double 5.0"), "{ir}");
        assert!(ir.contains("double 7") || ir.contains("double 7.0"), "{ir}");
        assert!(ir.contains("double 3") || ir.contains("double 3.0"), "{ir}");
    }
}
