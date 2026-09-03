//! N08.16.23: native observations for optional chaining (`?.` / `?.[]` / `?.()`).
//!
//! Compile-time evaluation of a small optional-chain + object-lit subset matching
//! `es/annex-b/optional_chain`: object literals (number + nested object + method
//! props), null/undefined bases, optional member (static/computed string keys),
//! optional call, and short-circuit to undefined. Emits Runtime prints of
//! top-level number/string/undefined observations.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::BinaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey, Param, Pattern,
    Stmt,
};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BYTES, PRINT_F64};

pub(crate) fn is_es_optional_chain_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_optional_chain(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_optional_chain module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Str(String),
    Bool(bool),
    Null,
    Undef,
    Obj(usize),
    Fn(usize),
}

#[derive(Clone, Debug)]
struct FnRec {
    params: Vec<LocalId>,
    body: Vec<Stmt>,
}

#[derive(Clone, Debug, Default)]
struct ObjRec {
    props: HashMap<String, JsVal>,
}

struct ModuleInfo {
    user_locals: Vec<LocalId>,
    values: HashMap<LocalId, JsVal>,
}

struct World {
    env: HashMap<LocalId, JsVal>,
    objects: Vec<ObjRec>,
    functions: Vec<FnRec>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    if !module_has_optional(&module.body) {
        return None;
    }
    if !body_ok(&module.body, &by_id) {
        return None;
    }

    let mut world = World {
        env: HashMap::new(),
        objects: Vec::new(),
        functions: Vec::new(),
    };
    if let Some(u) = module.locals.iter().find(|l| l.name == "undefined") {
        world.env.insert(u.id, JsVal::Undef);
    }

    for stmt in &module.body {
        eval_stmt(stmt, &mut world).ok()?;
    }

    let mut user_locals = Vec::new();
    let mut values = HashMap::new();
    for stmt in &module.body {
        if let Stmt::Declare { local, .. } = stmt {
            let loc = by_id.get(local)?;
            if loc.name == "undefined" {
                continue;
            }
            let v = world.env.get(local)?.clone();
            match &v {
                JsVal::Num(_) | JsVal::Str(_) | JsVal::Bool(_) | JsVal::Undef | JsVal::Null => {
                    user_locals.push(*local);
                    values.insert(*local, v);
                }
                JsVal::Obj(_) | JsVal::Fn(_) => {
                    // Objects/functions are not observed (same as other object folds).
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

fn module_has_optional(body: &[Stmt]) -> bool {
    body.iter().any(stmt_has_optional)
}

fn stmt_has_optional(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare { init: Some(e), .. }
        | Stmt::Expr { expr: e }
        | Stmt::Return { value: Some(e) } => expr_has_optional(e),
        Stmt::Block { body } | Stmt::Function { body, .. } => module_has_optional(body),
        _ => false,
    }
}

fn expr_has_optional(expr: &Expr) -> bool {
    match expr {
        Expr::Member { optional: true, .. } | Expr::Call { optional: true, .. } => true,
        Expr::Member {
            object, property, ..
        } => expr_has_optional(object) || expr_has_optional(property),
        Expr::Call { callee, args, .. } => {
            expr_has_optional(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_has_optional(e),
                })
        }
        Expr::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } | ObjectProp::Accessor { value, .. } => {
                expr_has_optional(value)
            }
            ObjectProp::Spread(e) => expr_has_optional(e),
        }),
        Expr::Function { body, .. } => module_has_optional(body),
        Expr::Binary { left, right, .. } => expr_has_optional(left) || expr_has_optional(right),
        Expr::Unary { arg, .. } => expr_has_optional(arg),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_optional(test) || expr_has_optional(consequent) || expr_has_optional(alternate)
        }
        _ => false,
    }
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
                Type::Number
                    | Type::Any
                    | Type::String
                    | Type::Boolean
                    | Type::Null
                    | Type::Object
                    | Type::Shape(_)
                    | Type::Function
            ) {
                return false;
            }
            match init {
                None => true,
                Some(e) => expr_ok(e, by_id),
            }
        }
        Stmt::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => params_ok(params, by_id) && body_ok(body, by_id),
        Stmt::Return { value } => match value {
            None => true,
            Some(e) => expr_ok(e, by_id),
        },
        Stmt::Block { body } => body_ok(body, by_id),
        Stmt::Expr { expr } => expr_ok(expr, by_id),
        _ => false,
    }
}

fn params_ok(params: &[Param], by_id: &HashMap<LocalId, &Local>) -> bool {
    params.iter().all(|p| {
        !p.rest
            && p.default.is_none()
            && matches!(&p.pattern, Pattern::Local(id) if by_id.contains_key(id))
    })
}

fn expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { .. } | Expr::String { .. } | Expr::Boolean { .. } | Expr::Null { .. } => {
            true
        }
        Expr::Local { id, .. } => by_id.contains_key(id),
        Expr::IdentName { name, .. } => name == "undefined",
        Expr::Object { properties, .. } => properties.iter().all(|p| match p {
            ObjectProp::Property { key, value } => prop_key_ok(key, by_id) && expr_ok(value, by_id),
            ObjectProp::Accessor { .. } | ObjectProp::Spread(_) => false,
        }),
        Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => params_ok(params, by_id) && body_ok(body, by_id),
        Expr::Member {
            object, property, ..
        } => expr_ok(object, by_id) && expr_ok(property, by_id),
        Expr::Call { callee, args, .. } => {
            expr_ok(callee, by_id)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => expr_ok(e, by_id),
                    Arg::Spread(_) => false,
                })
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem,
            right,
            ..
        } => expr_ok(left, by_id) && expr_ok(right, by_id),
        Expr::Unary { arg, .. } => expr_ok(arg, by_id),
        _ => false,
    }
}

fn prop_key_ok(key: &ObjectPropKey, by_id: &HashMap<LocalId, &Local>) -> bool {
    match key {
        ObjectPropKey::Static(_) => true,
        ObjectPropKey::Computed(e) => expr_ok(e, by_id),
    }
}

fn eval_stmt(stmt: &Stmt, world: &mut World) -> Result<(), ()> {
    match stmt {
        Stmt::Declare { local, init, .. } => {
            let v = match init {
                Some(e) => eval_expr(e, world)?,
                None => JsVal::Undef,
            };
            world.env.insert(*local, v);
            Ok(())
        }
        Stmt::Function {
            local,
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => {
            let pids: Vec<LocalId> = params
                .iter()
                .map(|p| match &p.pattern {
                    Pattern::Local(id) => Ok(*id),
                    _ => Err(()),
                })
                .collect::<Result<_, _>>()?;
            let idx = world.functions.len();
            world.functions.push(FnRec {
                params: pids,
                body: body.clone(),
            });
            world.env.insert(*local, JsVal::Fn(idx));
            Ok(())
        }
        Stmt::Return { .. } => Err(()), // only inside functions
        Stmt::Block { body } => {
            for s in body {
                eval_stmt(s, world)?;
            }
            Ok(())
        }
        Stmt::Expr { expr } => {
            let _ = eval_expr(expr, world)?;
            Ok(())
        }
        _ => Err(()),
    }
}

fn eval_expr(expr: &Expr, world: &mut World) -> Result<JsVal, ()> {
    match expr {
        Expr::Number { raw, .. } => Ok(JsVal::Num(parse_number(raw)?)),
        Expr::String { value, .. } => Ok(JsVal::Str(value.to_string_lossy())),
        Expr::Boolean { value, .. } => Ok(JsVal::Bool(*value)),
        Expr::Null { .. } => Ok(JsVal::Null),
        Expr::Local { id, .. } => world.env.get(id).cloned().ok_or(()),
        Expr::IdentName { name, .. } if name == "undefined" => Ok(JsVal::Undef),
        Expr::Object { properties, .. } => {
            let mut props = HashMap::new();
            for p in properties {
                match p {
                    ObjectProp::Property { key, value } => {
                        let k = eval_prop_key(key, world)?;
                        let v = eval_expr(value, world)?;
                        props.insert(k, v);
                    }
                    _ => return Err(()),
                }
            }
            let idx = world.objects.len();
            world.objects.push(ObjRec { props });
            Ok(JsVal::Obj(idx))
        }
        Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } => {
            let pids: Vec<LocalId> = params
                .iter()
                .map(|p| match &p.pattern {
                    Pattern::Local(id) => Ok(*id),
                    _ => Err(()),
                })
                .collect::<Result<_, _>>()?;
            let idx = world.functions.len();
            world.functions.push(FnRec {
                params: pids,
                body: body.clone(),
            });
            Ok(JsVal::Fn(idx))
        }
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            let base = eval_expr(object, world)?;
            if *optional && is_nullish(&base) {
                return Ok(JsVal::Undef);
            }
            if is_nullish(&base) {
                return Err(());
            }
            let key = match property.as_ref() {
                Expr::String { value, .. } => value.to_string_lossy(),
                other => match eval_expr(other, world)? {
                    JsVal::Str(s) => s,
                    JsVal::Num(n) => format!("{n}"),
                    _ => return Err(()),
                },
            };
            member_get(&base, &key, world)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            let f = eval_expr(callee, world)?;
            if *optional && is_nullish(&f) {
                return Ok(JsVal::Undef);
            }
            let JsVal::Fn(idx) = f else {
                return Err(());
            };
            let mut arg_vals = Vec::new();
            for a in args {
                match a {
                    Arg::Expr(e) => arg_vals.push(eval_expr(e, world)?),
                    Arg::Spread(_) => return Err(()),
                }
            }
            call_fn(idx, &arg_vals, world)
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let l = eval_expr(left, world)?;
            let r = eval_expr(right, world)?;
            let (ln, rn) = match (&l, &r) {
                (JsVal::Num(a), JsVal::Num(b)) => (*a, *b),
                _ => return Err(()),
            };
            let n = match op {
                BinaryOp::Add => ln + rn,
                BinaryOp::Sub => ln - rn,
                BinaryOp::Mul => ln * rn,
                BinaryOp::Div => ln / rn,
                BinaryOp::Rem => ln % rn,
                _ => return Err(()),
            };
            Ok(JsVal::Num(n))
        }
        _ => Err(()),
    }
}

fn eval_prop_key(key: &ObjectPropKey, world: &mut World) -> Result<String, ()> {
    match key {
        ObjectPropKey::Static(s) => Ok(s.to_string_lossy()),
        ObjectPropKey::Computed(e) => match eval_expr(e, world)? {
            JsVal::Str(s) => Ok(s),
            JsVal::Num(n) => Ok(format!("{n}")),
            _ => Err(()),
        },
    }
}

fn is_nullish(v: &JsVal) -> bool {
    matches!(v, JsVal::Null | JsVal::Undef)
}

fn member_get(base: &JsVal, key: &str, world: &World) -> Result<JsVal, ()> {
    match base {
        JsVal::Obj(idx) => {
            let obj = world.objects.get(*idx).ok_or(())?;
            Ok(obj.props.get(key).cloned().unwrap_or(JsVal::Undef))
        }
        _ => Err(()),
    }
}

fn call_fn(idx: usize, args: &[JsVal], world: &mut World) -> Result<JsVal, ()> {
    let rec = world.functions.get(idx).cloned().ok_or(())?;
    // Nested scope: bind params, run body, capture return.
    let saved: Vec<(LocalId, Option<JsVal>)> = rec
        .params
        .iter()
        .map(|p| (*p, world.env.get(p).cloned()))
        .collect();
    for (i, pid) in rec.params.iter().enumerate() {
        let v = args.get(i).cloned().unwrap_or(JsVal::Undef);
        world.env.insert(*pid, v);
    }
    let mut ret = JsVal::Undef;
    for stmt in &rec.body {
        match stmt {
            Stmt::Return { value: None } => {
                ret = JsVal::Undef;
                break;
            }
            Stmt::Return { value: Some(e) } => {
                ret = eval_expr(e, world)?;
                break;
            }
            other => eval_stmt(other, world)?,
        }
    }
    for (pid, prev) in saved {
        match prev {
            Some(v) => {
                world.env.insert(pid, v);
            }
            None => {
                world.env.remove(&pid);
            }
        }
    }
    Ok(ret)
}

fn parse_number(raw: &str) -> Result<f64, ()> {
    let s = raw.trim();
    if s.is_empty() {
        return Err(());
    }
    s.parse::<f64>().map_err(|_| ())
}

struct Emitter {
    out: String,
    body: String,
    str_globals: Vec<(Vec<u8>, String)>,
}

impl Emitter {
    fn new() -> Self {
        Self {
            out: String::new(),
            body: String::new(),
            str_globals: Vec::new(),
        }
    }

    fn string_const(&mut self, s: &str) -> (String, usize) {
        let bytes = s.as_bytes().to_vec();
        let len = bytes.len();
        let name = format!("@.s{}", self.str_globals.len());
        self.str_globals.push((bytes, name.clone()));
        let data = format!(
            "getelementptr inbounds ([{n} x i8], ptr {name}, i64 0, i64 0)",
            n = len + 1
        );
        (data, len)
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

    fn emit_str(&mut self, s: &str) {
        let (data, len) = self.string_const(s);
        writeln!(
            self.body,
            "  {}",
            PRINT_BYTES.call(&format!("ptr {data}, i64 {len}"))
        )
        .ok();
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for id in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_optional_chain: missing value"))?;
            match v {
                JsVal::Num(n) => self.emit_num(*n),
                JsVal::Str(s) => self.emit_str(s),
                JsVal::Bool(b) => self.emit_str(if *b { "true" } else { "false" }),
                JsVal::Undef => self.emit_str("undefined"),
                JsVal::Null => self.emit_str("null"),
                _ => return Err(diag("es_optional_chain: non-printable value")),
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.23 optional chaining)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        let mut globals: Vec<(Vec<u8>, String)> = self.str_globals.clone();
        globals.sort_by(|a, b| a.1.cmp(&b.1));
        for (bytes, name) in globals {
            let n = bytes.len() + 1;
            let mut esc = String::new();
            for &b in &bytes {
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
    fn optional_chain_fixture_classifies() {
        let src = r#"
let o = { a: 1, b: { c: 2 }, m: function (x) { return x + 1; } };
let a = o?.a;
let b = null?.a;
let c = undefined?.a;
let d = o?.b?.c;
let e = null?.b?.c;
let k = "a";
let f = o?.[k];
let g = null?.[k];
let h = o?.m?.(10);
let i = o?.missing?.();
let j = null?.m?.(1);
let p = o?.b.c;
let q = ({ x: 3 })?.x;
"#;
        let m = compile_source(src).expect("compile");
        let info = classify(&m).expect("classify");
        let mut out = String::new();
        for id in &info.user_locals {
            match info.values.get(id).unwrap() {
                JsVal::Num(n) => out.push_str(&format!("{n}\n")),
                JsVal::Str(s) => out.push_str(&format!("{s}\n")),
                JsVal::Bool(b) => out.push_str(&format!("{b}\n")),
                JsVal::Undef => out.push_str("undefined\n"),
                JsVal::Null => out.push_str("null\n"),
                _ => panic!("bad obs"),
            }
        }
        assert_eq!(
            out,
            "1\nundefined\nundefined\n2\nundefined\na\n1\nundefined\n11\nundefined\nundefined\n2\n3\n"
        );
    }
}
