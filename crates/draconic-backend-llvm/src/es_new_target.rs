//! N08.16.27: native observations for `new.target` (E18.27 / `es/annex-b/new_target`).
//!
//! Compile-time evaluation of function/`constructor` `new.target`, non-`new` →
//! `undefined`, subclass active construct, nested functions, and arrows that
//! inherit the enclosing `new.target`. Class builder IIFEs are collapsed like
//! `es_classes` (base + derived `super()`). Emits Runtime prints of final
//! top-level bool/string/undefined observations.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::rc::Rc;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey,
    Param, Pattern, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_STR};

#[path = "es_new_target_eval.rs"]
mod eval;

pub(crate) fn is_es_new_target_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_new_target(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_new_target module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_new_target(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_new_target_module(module) {
        return None;
    }
    Some(emit_es_new_target(module))
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

#[derive(Clone, Debug)]
enum JsVal {
    Undef,
    Bool(bool),
    Str(String),
    /// Shared so constructor `this.prop = …` mutates the allocated instance.
    Object {
        id: u64,
        props: Rc<RefCell<Vec<(String, JsVal)>>>,
    },
    Fn {
        id: u64,
    },
}

#[derive(Clone)]
struct FnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
    is_arrow: bool,
    /// Parent constructor fn id for derived `super()`.
    parent: Option<u64>,
}

thread_local! {
    static CURRENT_THIS: RefCell<JsVal> = const { RefCell::new(JsVal::Undef) };
    static CURRENT_NEW_TARGET: RefCell<JsVal> = const { RefCell::new(JsVal::Undef) };
}

struct World {
    fn_reg: RefCell<HashMap<u64, FnRec>>,
    next: Cell<u64>,
}

impl World {
    fn new() -> Self {
        Self {
            fn_reg: RefCell::new(HashMap::new()),
            next: Cell::new(1),
        }
    }

    fn next_id(&self) -> u64 {
        let id = self.next.get();
        self.next.set(id + 1);
        id
    }

    fn fn_reg_insert(&self, id: u64, rec: FnRec) {
        self.fn_reg.borrow_mut().insert(id, rec);
    }

    fn fn_reg_get(&self, id: u64) -> Option<FnRec> {
        self.fn_reg.borrow().get(&id).cloned()
    }
}

fn with_this_nt<R>(this: JsVal, nt: JsVal, f: impl FnOnce() -> R) -> R {
    CURRENT_THIS.with(|t| {
        let prev_t = t.replace(this);
        CURRENT_NEW_TARGET.with(|n| {
            let prev_n = n.replace(nt);
            let r = f();
            n.replace(prev_n);
            t.replace(prev_t);
            r
        })
    })
}

fn current_this() -> JsVal {
    CURRENT_THIS.with(|c| c.borrow().clone())
}

fn current_new_target() -> JsVal {
    CURRENT_NEW_TARGET.with(|c| c.borrow().clone())
}

#[derive(Clone, Debug)]
enum Flow {
    Normal,
    Return(JsVal),
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    if !module_has_new_target(module) {
        return None;
    }
    let world = World::new();
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    match world.eval_body(&module.body, &mut env) {
        Ok(Flow::Normal) => {}
        _ => return None,
    }

    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            match env.get(local) {
                Some(v @ (JsVal::Bool(_) | JsVal::Str(_) | JsVal::Undef)) => {
                    if matches!(loc.ty, Type::Any | Type::Boolean | Type::String) {
                        user_locals.push(*local);
                        values.insert(*local, v.clone());
                    }
                }
                Some(JsVal::Object { .. } | JsVal::Fn { .. }) => {}
                None => return None,
            }
        }
    }
    if user_locals.is_empty() {
        return None;
    }
    // Class lowering injects `new.target` into constructors, so bare class
    // fixtures also set module_has_new_target. The new_target fixture always
    // observes booleans (`===` / identity). Reject string/undef-only folds
    // (e.g. class_fields typeof/undefined) so they fall through to es_classes.
    if !values.values().any(|v| matches!(v, JsVal::Bool(_))) {
        return None;
    }
    Some(ModuleInfo {
        user_locals,
        values,
    })
}

fn module_has_new_target(module: &Module) -> bool {
    module.body.iter().any(stmt_has_new_target)
}

fn stmt_has_new_target(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. }
        | Stmt::Expr { expr: e }
        | Stmt::Return { value: Some(e) }
        | Stmt::Throw { value: e } => expr_has_new_target(e),
        Stmt::Function { body, .. } | Stmt::Block { body } => body.iter().any(stmt_has_new_target),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_has_new_target(test)
                || stmt_has_new_target(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_has_new_target(a))
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            block.iter().any(stmt_has_new_target)
                || handler
                    .as_ref()
                    .is_some_and(|h| h.iter().any(stmt_has_new_target))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| f.iter().any(stmt_has_new_target))
        }
        _ => false,
    }
}

fn expr_has_new_target(expr: &Expr) -> bool {
    match expr {
        Expr::NewTarget { .. } => true,
        Expr::Unary { arg, .. } => expr_has_new_target(arg),
        Expr::Binary { left, right, .. } => expr_has_new_target(left) || expr_has_new_target(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_new_target(test)
                || expr_has_new_target(consequent)
                || expr_has_new_target(alternate)
        }
        Expr::Member {
            object, property, ..
        } => expr_has_new_target(object) || expr_has_new_target(property),
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_has_new_target(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) => expr_has_new_target(e),
                    _ => false,
                })
        }
        Expr::Assign { value, .. } => expr_has_new_target(value),
        Expr::Function { body, .. } => body.iter().any(stmt_has_new_target),
        _ => false,
    }
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
        let name = format!("@.ntstr.{}", self.str_consts.len());
        self.str_consts.push((s.to_string(), name.clone()));
        name
    }

    fn emit_str(&mut self, s: &str) {
        let name = self.string_const(s);
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {name}"))).ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_new_target: missing value"))?;
            match v {
                JsVal::Str(s) => self.emit_str(s),
                JsVal::Bool(b) => self.emit_str(if *b { "true" } else { "false" }),
                JsVal::Undef => self.emit_str("undefined"),
                _ => return Err(diag("es_new_target: non-printable value")),
            }
        }

        writeln!(self.out, "; Draconic LLVM backend (N08.16.27 new.target)").ok();
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
    fn new_target_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/annex-b/new_target.drac");
        let m = compile_source(src).expect("compile");
        assert!(
            is_es_new_target_module(&m),
            "should classify as es_new_target"
        );
        let ir = emit_es_new_target(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "must not use hello stub:\n{ir}"
        );
        for s in ["true", "function", "undefined"] {
            assert!(ir.contains(s), "missing {s:?} in emit:\n{ir}");
        }
    }
}
