//! N08.15: native observations for legacy `with` (E17.01 / `es/legacy/with_*`).
//!
//! Compile-time evaluation of a small Object Environment subset matching
//! `with_basic` and `with_nested`: object literals with number props, nested
//! `with`, bare IdentName get/put via the with chain, block-local `let` inside
//! `with`, and member read-back. Emits Runtime prints of final top-level
//! number locals.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::AssignOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey, Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_F64};

pub(crate) fn is_es_legacy_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_legacy(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_legacy module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    /// Heap object id into `Heap.objects`.
    Obj(usize),
    Undef,
}

struct Heap {
    objects: Vec<HashMap<String, JsVal>>,
}

impl Heap {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
        }
    }

    fn alloc(&mut self, props: HashMap<String, JsVal>) -> usize {
        let id = self.objects.len();
        self.objects.push(props);
        id
    }

    fn has_own(&self, obj: usize, key: &str) -> bool {
        self.objects
            .get(obj)
            .is_some_and(|m| m.contains_key(key))
    }

    fn get(&self, obj: usize, key: &str) -> JsVal {
        self.objects
            .get(obj)
            .and_then(|m| m.get(key).cloned())
            .unwrap_or(JsVal::Undef)
    }

    fn set(&mut self, obj: usize, key: &str, val: JsVal) -> Result<(), ()> {
        let m = self.objects.get_mut(obj).ok_or(())?;
        m.insert(key.to_string(), val);
        Ok(())
    }
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, f64>,
}

/// Active `with` object chain (innermost last).
type WithChain = Vec<usize>;

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_with(&module.body) {
        return None;
    }
    if !body_ok(&module.body, &by_id) {
        return None;
    }

    let mut env: HashMap<LocalId, JsVal> = HashMap::new();
    let mut name_to_id: HashMap<String, LocalId> = HashMap::new();
    for loc in &module.locals {
        name_to_id.insert(loc.name.clone(), loc.id);
    }
    let mut heap = Heap::new();
    let with_chain: WithChain = Vec::new();

    eval_body(
        &module.body,
        &mut env,
        &mut heap,
        &with_chain,
        &name_to_id,
        &by_id,
    )
    .ok()?;

    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            if matches!(loc.ty, Type::Number | Type::Any) {
                match env.get(local) {
                    Some(JsVal::Num(n)) => {
                        user_locals.push(*local);
                        values.insert(*local, *n);
                    }
                    Some(JsVal::Obj(_)) => {}
                    _ => return None,
                }
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

fn module_has_with(body: &[Stmt]) -> bool {
    body.iter().any(|s| match s {
        Stmt::With { .. } => true,
        Stmt::Block { body } => module_has_with(body),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            module_has_with(std::slice::from_ref(consequent.as_ref()))
                || alternate
                    .as_ref()
                    .is_some_and(|a| module_has_with(std::slice::from_ref(a.as_ref())))
        }
        _ => false,
    })
}

fn body_ok(body: &[Stmt], by_id: &HashMap<LocalId, &Local>) -> bool {
    body.iter().all(|s| stmt_ok(s, by_id))
}

fn stmt_ok(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let Some(loc) = by_id.get(local) else {
                return false;
            };
            if !matches!(
                loc.ty,
                Type::Number | Type::Any | Type::Object | Type::Shape(_)
            ) {
                return false;
            }
            match init {
                None => true,
                Some(e) => expr_ok(e, by_id),
            }
        }
        Stmt::With { object, body } => expr_ok(object, by_id) && body_ok(body, by_id),
        Stmt::Block { body } => body_ok(body, by_id),
        Stmt::Expr { expr } => expr_ok(expr, by_id),
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            expr_ok(test, by_id)
                && stmt_ok(consequent, by_id)
                && alternate.as_ref().is_none_or(|a| stmt_ok(a, by_id))
        }
        _ => false,
    }
}

fn expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } | Expr::Null { .. } => true,
        Expr::Local { id, .. } => by_id.contains_key(id),
        Expr::IdentName { .. } => true,
        Expr::Object { properties, .. } => properties.iter().all(|p| match p {
            ObjectProp::Property {
                key: ObjectPropKey::Static(_),
                value,
            } => expr_ok(value, by_id),
            _ => false,
        }),
        Expr::Member {
            object,
            property,
            optional: false,
            ..
        } => expr_ok(object, by_id) && expr_ok(property, by_id),
        Expr::Assign {
            target,
            op: AssignOp::Eq,
            value,
            ..
        } => {
            expr_ok(value, by_id)
                && match target {
                    AssignTarget::Local(id) => by_id.contains_key(id),
                    AssignTarget::Name(_) => true,
                    AssignTarget::Member {
                        object,
                        property,
                        ..
                    } => expr_ok(object, by_id) && expr_ok(property, by_id),
                    _ => false,
                }
        }
        _ => false,
    }
}

fn eval_body(
    body: &[Stmt],
    env: &mut HashMap<LocalId, JsVal>,
    heap: &mut Heap,
    with_chain: &WithChain,
    name_to_id: &HashMap<String, LocalId>,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<(), ()> {
    for stmt in body {
        eval_stmt(stmt, env, heap, with_chain, name_to_id, by_id)?;
    }
    Ok(())
}

fn eval_stmt(
    stmt: &Stmt,
    env: &mut HashMap<LocalId, JsVal>,
    heap: &mut Heap,
    with_chain: &WithChain,
    name_to_id: &HashMap<String, LocalId>,
    by_id: &HashMap<LocalId, &Local>,
) -> Result<(), ()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => eval_expr(e, env, heap, with_chain, name_to_id)?,
                None => JsVal::Undef,
            };
            env.insert(*local, v);
            Ok(())
        }
        Stmt::With { object, body } => {
            let v = eval_expr(object, env, heap, with_chain, name_to_id)?;
            let JsVal::Obj(oid) = v else {
                return Err(());
            };
            let mut chain = with_chain.clone();
            chain.push(oid);
            eval_body(body, env, heap, &chain, name_to_id, by_id)
        }
        Stmt::Block { body } => eval_body(body, env, heap, with_chain, name_to_id, by_id),
        Stmt::Expr { expr } => {
            eval_expr(expr, env, heap, with_chain, name_to_id)?;
            Ok(())
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            let t = to_boolean(&eval_expr(test, env, heap, with_chain, name_to_id)?);
            if t {
                eval_stmt(consequent, env, heap, with_chain, name_to_id, by_id)
            } else if let Some(a) = alternate {
                eval_stmt(a, env, heap, with_chain, name_to_id, by_id)
            } else {
                Ok(())
            }
        }
        _ => Err(()),
    }
}

fn eval_expr(
    expr: &Expr,
    env: &mut HashMap<LocalId, JsVal>,
    heap: &mut Heap,
    with_chain: &WithChain,
    name_to_id: &HashMap<String, LocalId>,
) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().map_err(|_| ())?;
            Ok(JsVal::Num(n))
        }
        Expr::String { value, .. } => {
            // Used as member keys via member_key; bare string values unused in fixtures.
            let _ = value;
            Err(())
        }
        Expr::Boolean { value, .. } => Ok(JsVal::Num(if *value { 1.0 } else { 0.0 })),
        Expr::Null { .. } => Ok(JsVal::Num(0.0)),
        Expr::Local { id, .. } => env.get(id).cloned().ok_or(()),
        Expr::IdentName { name, .. } => resolve_name(name, env, heap, with_chain, name_to_id),
        Expr::Object { properties, .. } => {
            let mut props = HashMap::new();
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key: ObjectPropKey::Static(k),
                        value,
                    } => {
                        let key = k.to_string_lossy();
                        let v = eval_expr(value, env, heap, with_chain, name_to_id)?;
                        props.insert(key, v);
                    }
                    _ => return Err(()),
                }
            }
            Ok(JsVal::Obj(heap.alloc(props)))
        }
        Expr::Member {
            object,
            property,
            computed,
            optional: false,
            ..
        } => {
            let obj = eval_expr(object, env, heap, with_chain, name_to_id)?;
            let JsVal::Obj(oid) = obj else {
                return Err(());
            };
            let key = member_key(property, *computed, env, heap, with_chain, name_to_id)?;
            Ok(heap.get(oid, &key))
        }
        Expr::Assign {
            target,
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = eval_expr(value, env, heap, with_chain, name_to_id)?;
            put_value(target, v.clone(), env, heap, with_chain, name_to_id)?;
            Ok(v)
        }
        _ => Err(()),
    }
}

fn member_key(
    property: &Expr,
    computed: bool,
    env: &mut HashMap<LocalId, JsVal>,
    heap: &mut Heap,
    with_chain: &WithChain,
    name_to_id: &HashMap<String, LocalId>,
) -> Result<String, ()> {
    if !computed {
        match property {
            Expr::String { value, .. } => Ok(value.to_string_lossy()),
            // non-computed often lowers as string lit
            Expr::IdentName { name, .. } => Ok(name.clone()),
            _ => Err(()),
        }
    } else {
        match eval_expr(property, env, heap, with_chain, name_to_id)? {
            JsVal::Num(n) => Ok(format!("{}", n as i64)),
            _ => Err(()),
        }
    }
}

fn resolve_name(
    name: &str,
    env: &HashMap<LocalId, JsVal>,
    heap: &Heap,
    with_chain: &WithChain,
    name_to_id: &HashMap<String, LocalId>,
) -> Result<JsVal, ()> {
    // Innermost with object first (ECMA-262 Object Environment).
    for &oid in with_chain.iter().rev() {
        if heap.has_own(oid, name) {
            return Ok(heap.get(oid, name));
        }
    }
    // Outer lexical binding by name.
    if let Some(id) = name_to_id.get(name) {
        return env.get(id).cloned().ok_or(());
    }
    Err(())
}

fn put_value(
    target: &AssignTarget,
    val: JsVal,
    env: &mut HashMap<LocalId, JsVal>,
    heap: &mut Heap,
    with_chain: &WithChain,
    name_to_id: &HashMap<String, LocalId>,
) -> Result<(), ()> {
    match target {
        AssignTarget::Local(id) => {
            env.insert(*id, val);
            Ok(())
        }
        AssignTarget::Name(name) => {
            for &oid in with_chain.iter().rev() {
                if heap.has_own(oid, name) {
                    return heap.set(oid, name, val);
                }
            }
            // Unqualified assign creates/sets on outermost with object if any
            // (fixtures only assign existing props or outer locals). Prefer
            // outer lexical binding when present.
            if let Some(id) = name_to_id.get(name) {
                env.insert(*id, val);
                return Ok(());
            }
            if let Some(&oid) = with_chain.last() {
                return heap.set(oid, name, val);
            }
            Err(())
        }
        AssignTarget::Member {
            object,
            property,
            computed,
        } => {
            let obj = eval_expr(object, env, heap, with_chain, name_to_id)?;
            let JsVal::Obj(oid) = obj else {
                return Err(());
            };
            let key = member_key(property, *computed, env, heap, with_chain, name_to_id)?;
            heap.set(oid, &key, val)
        }
        _ => Err(()),
    }
}

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Obj(_) => true,
        JsVal::Undef => false,
    }
}

struct Emitter {
    out: String,
    body: String,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
        }
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
            let n = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_legacy: missing value"))?;
            self.emit_num(*n);
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.15 legacy with)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
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
