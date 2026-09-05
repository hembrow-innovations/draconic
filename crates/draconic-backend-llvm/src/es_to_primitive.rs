//! N08.09.04: native observations for ToPrimitive valueOf/toString (E09.04).
//!
//! Supports object literals with `valueOf` / `toString` function properties
//! (no-arg, return number/string/empty-object), then `+` and `==`/`!=` that
//! apply OrdinaryToPrimitive (number default hint) — matching
//! `es/values/to_primitive`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{BinaryOp, JsString};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BOOL, PRINT_BYTES, PRINT_F64};

pub(crate) fn is_es_to_primitive_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_to_primitive(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_to_primitive module"))?;
    let mut em = Emitter::new();
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Boolean,
    String,
    Number,
    Object,
    /// `Any` result of `+` — print by runtime value kind.
    Any,
}

struct ModuleInfo {
    user_locals: Vec<(LocalId, SlotTy)>,
    values: HashMap<LocalId, JsVal>,
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Null,
    Undef,
    /// Object with optional valueOf / toString hooks (compile-time).
    Obj(ObjRec),
}

#[derive(Clone, Debug)]
struct ObjRec {
    id: u64,
    /// Primitive result of calling valueOf, if present and returns a primitive.
    /// `Some(Obj(_))` means valueOf returned an object (fall through).
    value_of: Option<Box<JsVal>>,
    to_string: Option<Box<JsVal>>,
    /// True when valueOf exists (even if it returns object).
    has_value_of: bool,
    has_to_string: bool,
}

fn js_string_to_utf8(s: &JsString) -> String {
    s.to_string_lossy()
}

fn is_object_ty(ty: &Type) -> bool {
    matches!(ty, Type::Object | Type::Shape(_) | Type::Any)
}

fn to_number(v: &JsVal) -> f64 {
    match v {
        JsVal::Num(n) => *n,
        JsVal::Bool(true) => 1.0,
        JsVal::Bool(false) => 0.0,
        JsVal::Null => 0.0,
        JsVal::Undef => f64::NAN,
        JsVal::Str(s) => {
            let t = s.trim();
            if t.is_empty() {
                0.0
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        JsVal::Obj(_) => f64::NAN,
    }
}

fn to_string(v: &JsVal) -> String {
    match v {
        JsVal::Num(n) => {
            if n.is_nan() {
                "NaN".into()
            } else if *n == 0.0 {
                "0".into()
            } else if n.is_infinite() {
                if n.is_sign_negative() {
                    "-Infinity".into()
                } else {
                    "Infinity".into()
                }
            } else if *n == (*n as i64 as f64) && n.abs() < 1e21 {
                format!("{}", *n as i64)
            } else {
                let s = format!("{n}");
                if s == "-0" {
                    "0".into()
                } else {
                    s
                }
            }
        }
        JsVal::Bool(true) => "true".into(),
        JsVal::Bool(false) => "false".into(),
        JsVal::Null => "null".into(),
        JsVal::Undef => "undefined".into(),
        JsVal::Str(s) => s.clone(),
        JsVal::Obj(_) => "[object Object]".into(),
    }
}

fn is_object(v: &JsVal) -> bool {
    matches!(v, JsVal::Obj(_))
}

/// OrdinaryToPrimitive with default number hint (`valueOf` then `toString`).
fn to_primitive(v: &JsVal) -> JsVal {
    match v {
        JsVal::Obj(o) => {
            // number hint: valueOf, then toString
            if o.has_value_of {
                if let Some(ref r) = o.value_of {
                    if !is_object(r) {
                        return (**r).clone();
                    }
                }
            }
            if o.has_to_string {
                if let Some(ref r) = o.to_string {
                    if !is_object(r) {
                        return (**r).clone();
                    }
                }
            }
            // Default Object.prototype.toString
            JsVal::Str("[object Object]".into())
        }
        other => other.clone(),
    }
}

fn same_type_strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => {
            if x.is_nan() || y.is_nan() {
                false
            } else {
                *x == *y
            }
        }
        (JsVal::Bool(x), JsVal::Bool(y)) => x == y,
        (JsVal::Str(x), JsVal::Str(y)) => x == y,
        (JsVal::Null, JsVal::Null) => true,
        (JsVal::Undef, JsVal::Undef) => true,
        (JsVal::Obj(x), JsVal::Obj(y)) => x.id == y.id,
        _ => false,
    }
}

fn abstract_eq(a: &JsVal, b: &JsVal) -> bool {
    if std::mem::discriminant(a) == std::mem::discriminant(b)
        || matches!((a, b), (JsVal::Num(_), JsVal::Num(_)))
    {
        return match (a, b) {
            (JsVal::Num(_), JsVal::Num(_))
            | (JsVal::Bool(_), JsVal::Bool(_))
            | (JsVal::Str(_), JsVal::Str(_))
            | (JsVal::Obj(_), JsVal::Obj(_)) => same_type_strict_eq(a, b),
            (JsVal::Null, JsVal::Null) | (JsVal::Undef, JsVal::Undef) => true,
            _ => false,
        };
    }
    match (a, b) {
        (JsVal::Null, JsVal::Undef) | (JsVal::Undef, JsVal::Null) => true,
        (JsVal::Num(_), JsVal::Str(_)) => abstract_eq(a, &JsVal::Num(to_number(b))),
        (JsVal::Str(_), JsVal::Num(_)) => abstract_eq(&JsVal::Num(to_number(a)), b),
        (JsVal::Bool(_), _) => abstract_eq(&JsVal::Num(to_number(a)), b),
        (_, JsVal::Bool(_)) => abstract_eq(a, &JsVal::Num(to_number(b))),
        (JsVal::Null | JsVal::Undef, _) | (_, JsVal::Null | JsVal::Undef) => false,
        (JsVal::Obj(_), _) => abstract_eq(&to_primitive(a), b),
        (_, JsVal::Obj(_)) => abstract_eq(a, &to_primitive(b)),
        _ => false,
    }
}

fn add_vals(a: &JsVal, b: &JsVal) -> JsVal {
    let l = to_primitive(a);
    let r = to_primitive(b);
    match (&l, &r) {
        (JsVal::Str(_), _) | (_, JsVal::Str(_)) => {
            JsVal::Str(format!("{}{}", to_string(&l), to_string(&r)))
        }
        _ => JsVal::Num(to_number(&l) + to_number(&r)),
    }
}

fn static_key_name(key: &ObjectPropKey) -> Option<String> {
    match key {
        ObjectPropKey::Static(s) => Some(js_string_to_utf8(s)),
        _ => None,
    }
}

/// Evaluate a no-arg function body used as valueOf/toString (return literal only).
fn eval_hook_body(body: &[Stmt], next_obj: &mut u64) -> Option<JsVal> {
    if body.len() != 1 {
        return None;
    }
    match &body[0] {
        Stmt::Return {
            value: Some(expr), ..
        } => eval_literal_expr(expr, next_obj),
        _ => None,
    }
}

fn eval_literal_expr(expr: &Expr, next_obj: &mut u64) -> Option<JsVal> {
    match expr {
        Expr::Number { raw, .. } => Some(JsVal::Num(raw.parse().ok()?)),
        Expr::Boolean { value, .. } => Some(JsVal::Bool(*value)),
        Expr::String { value, .. } => Some(JsVal::Str(js_string_to_utf8(value))),
        Expr::Null { .. } => Some(JsVal::Null),
        Expr::Object { properties, .. } if properties.is_empty() => {
            let id = *next_obj;
            *next_obj += 1;
            Some(JsVal::Obj(ObjRec {
                id,
                value_of: None,
                to_string: None,
                has_value_of: false,
                has_to_string: false,
            }))
        }
        _ => None,
    }
}

fn eval_object_lit(properties: &[ObjectProp], next_obj: &mut u64) -> Option<JsVal> {
    let id = *next_obj;
    *next_obj += 1;
    let mut value_of = None;
    let mut to_string = None;
    let mut has_value_of = false;
    let mut has_to_string = false;

    for p in properties {
        let ObjectProp::Property { key, value } = p else {
            return None;
        };
        let name = static_key_name(key)?;
        let Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } = value
        else {
            return None;
        };
        if !params.is_empty() {
            return None;
        }
        let result = eval_hook_body(body, next_obj)?;
        match name.as_str() {
            "valueOf" => {
                has_value_of = true;
                value_of = Some(Box::new(result));
            }
            "toString" => {
                has_to_string = true;
                to_string = Some(Box::new(result));
            }
            _ => return None,
        }
    }

    if !has_value_of && !has_to_string {
        return None;
    }

    Some(JsVal::Obj(ObjRec {
        id,
        value_of,
        to_string,
        has_value_of,
        has_to_string,
    }))
}

fn object_lit_ok(properties: &[ObjectProp]) -> bool {
    if properties.is_empty() {
        return false;
    }
    let mut saw_hook = false;
    for p in properties {
        let ObjectProp::Property { key, value } = p else {
            return false;
        };
        let Some(name) = static_key_name(key) else {
            return false;
        };
        if name != "valueOf" && name != "toString" {
            return false;
        }
        let Expr::Function {
            params,
            body,
            is_async: false,
            is_generator: false,
            ..
        } = value
        else {
            return false;
        };
        if !params.is_empty() {
            return false;
        }
        if body.len() != 1 {
            return false;
        }
        match &body[0] {
            Stmt::Return {
                value: Some(expr), ..
            } => match expr {
                Expr::Number { .. }
                | Expr::String { .. }
                | Expr::Boolean { .. }
                | Expr::Null { .. } => {}
                Expr::Object { properties, .. } if properties.is_empty() => {}
                _ => return false,
            },
            _ => return false,
        }
        saw_hook = true;
    }
    saw_hook
}

fn has_to_primitive_marker(expr: &Expr) -> bool {
    match expr {
        Expr::Object { properties, .. } => object_lit_ok(properties),
        Expr::Binary {
            left, op, right, ..
        } => {
            let op_ok = matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::EqEq
                    | BinaryOp::NotEq
                    | BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
            );
            op_ok
                && (involves_object(left)
                    || involves_object(right)
                    || has_to_primitive_marker(left)
                    || has_to_primitive_marker(right))
        }
        _ => false,
    }
}

fn involves_object(expr: &Expr) -> bool {
    match expr {
        Expr::Object { .. } => true,
        Expr::Local { ty, .. } => is_object_ty(ty) && !matches!(ty, Type::Any),
        _ => false,
    }
}

fn expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    objs: &std::collections::HashSet<LocalId>,
) -> bool {
    match expr {
        Expr::Number { .. } | Expr::Boolean { .. } | Expr::String { .. } | Expr::Null { .. } => {
            true
        }
        Expr::Local { id, ty } => {
            if is_object_ty(ty) && !matches!(ty, Type::Any) {
                return objs.contains(id);
            }
            by_id.contains_key(id)
        }
        Expr::Object { properties, .. } => object_lit_ok(properties),
        Expr::Binary {
            left, op, right, ..
        } => {
            matches!(
                op,
                BinaryOp::Add
                    | BinaryOp::EqEq
                    | BinaryOp::NotEq
                    | BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
            ) && expr_ok(left, by_id, objs)
                && expr_ok(right, by_id, objs)
        }
        _ => false,
    }
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_locals = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut objs = std::collections::HashSet::new();
    let mut saw_marker = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                let init = init.as_ref()?;
                if !expr_ok(init, &by_id, &objs) {
                    return None;
                }
                if has_to_primitive_marker(init) {
                    saw_marker = true;
                }
                let slot = match &loc.ty {
                    Type::Boolean => SlotTy::Boolean,
                    Type::String => SlotTy::String,
                    Type::Number => SlotTy::Number,
                    Type::Any => {
                        if matches!(init, Expr::Object { .. }) {
                            objs.insert(*local);
                            SlotTy::Object
                        } else {
                            SlotTy::Any
                        }
                    }
                    ty if matches!(ty, Type::Object | Type::Shape(_)) => {
                        objs.insert(*local);
                        SlotTy::Object
                    }
                    _ => return None,
                };
                if matches!(slot, SlotTy::Object) {
                    objs.insert(*local);
                }
                if seen.insert(*local) {
                    user_locals.push((*local, slot));
                }
            }
            _ => return None,
        }
    }

    if user_locals.is_empty() || !saw_marker {
        return None;
    }
    // Must include at least one ToPrimitive object hook lit.
    if !user_locals.iter().any(|(_, s)| *s == SlotTy::Object) {
        return None;
    }

    let mut values: HashMap<LocalId, JsVal> = HashMap::new();
    let mut next_obj: u64 = 1;
    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let init = init.as_ref()?;
                let v = eval_expr(init, &values, &mut next_obj)?;
                values.insert(*local, v);
            }
            _ => return None,
        }
    }

    Some(ModuleInfo {
        user_locals,
        values,
    })
}

fn eval_expr(expr: &Expr, values: &HashMap<LocalId, JsVal>, next_obj: &mut u64) -> Option<JsVal> {
    match expr {
        Expr::Number { raw, .. } => Some(JsVal::Num(raw.parse().ok()?)),
        Expr::Boolean { value, .. } => Some(JsVal::Bool(*value)),
        Expr::String { value, .. } => Some(JsVal::Str(js_string_to_utf8(value))),
        Expr::Null { .. } => Some(JsVal::Null),
        Expr::Local { id, .. } => values.get(id).cloned(),
        Expr::Object { properties, .. } => eval_object_lit(properties, next_obj),
        Expr::Binary {
            left, op, right, ..
        } => {
            let l = eval_expr(left, values, next_obj)?;
            let r = eval_expr(right, values, next_obj)?;
            match op {
                BinaryOp::EqEq => Some(JsVal::Bool(abstract_eq(&l, &r))),
                BinaryOp::NotEq => Some(JsVal::Bool(!abstract_eq(&l, &r))),
                BinaryOp::EqEqEq => Some(JsVal::Bool(same_type_strict_eq(&l, &r))),
                BinaryOp::NotEqEq => Some(JsVal::Bool(!same_type_strict_eq(&l, &r))),
                BinaryOp::Add => Some(add_vals(&l, &r)),
                _ => None,
            }
        }
        _ => None,
    }
}

struct Emitter {
    str_globals: HashMap<Vec<u8>, String>,
    out: String,
    body: String,
    tmp: u32,
}

impl Emitter {
    fn new() -> Self {
        Self {
            str_globals: HashMap::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
        }
    }

    fn fresh(&mut self) -> String {
        let n = self.tmp;
        self.tmp += 1;
        format!("%t{n}")
    }

    fn string_const(&mut self, s: &str) -> (String, usize) {
        let bytes = s.as_bytes().to_vec();
        let len = bytes.len();
        let name = if let Some(n) = self.str_globals.get(&bytes) {
            n.clone()
        } else {
            let n = format!("@.str.{}", self.str_globals.len());
            self.str_globals.insert(bytes, n.clone());
            n
        };
        let data = format!(
            "getelementptr inbounds ([{n} x i8], ptr {name}, i64 0, i64 0)",
            n = len + 1
        );
        (data, len)
    }

    fn emit_bool(&mut self, b: bool) {
        let ext = self.fresh();
        let bit = if b { 1 } else { 0 };
        writeln!(self.body, "  {ext} = add i8 0, {bit}").ok();
        writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {ext}"))).ok();
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

    fn emit_val(&mut self, v: &JsVal) -> Result<(), Diagnostic> {
        match v {
            JsVal::Bool(b) => self.emit_bool(*b),
            JsVal::Str(s) => self.emit_str(s),
            JsVal::Num(n) => self.emit_num(*n),
            JsVal::Obj(_) | JsVal::Null | JsVal::Undef => {
                return Err(diag("es_to_primitive: unexpected non-printable value"));
            }
        }
        Ok(())
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for (id, slot) in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_to_primitive: missing value"))?;
            match slot {
                SlotTy::Object => {}
                SlotTy::Boolean => {
                    let JsVal::Bool(b) = v else {
                        return Err(diag("es_to_primitive: expected bool"));
                    };
                    self.emit_bool(*b);
                }
                SlotTy::String => {
                    let JsVal::Str(s) = v else {
                        return Err(diag("es_to_primitive: expected string"));
                    };
                    self.emit_str(s);
                }
                SlotTy::Number => {
                    let JsVal::Num(n) = v else {
                        return Err(diag("es_to_primitive: expected number"));
                    };
                    self.emit_num(*n);
                }
                SlotTy::Any => self.emit_val(v)?,
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.09.04 ToPrimitive valueOf/toString)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_EXPR_DECLARES)).ok();
        let mut globals: Vec<(Vec<u8>, String)> = self
            .str_globals
            .iter()
            .map(|(b, n)| (b.clone(), n.clone()))
            .collect();
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
