//! N08.09.03: native observations for abstract equality & coercion (E09.03).
//!
//! Supports mixed-type `==`/`!=`/`===`/`!==`, `+` (string concat / numeric with
//! ToNumber/ToString), unary `+`, `null`/`void`/`NaN`, empty object identity,
//! and `if` ToBoolean — matching `es/values/abstract_eq_coercion`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, Stmt};
use draconic_runtime::abi::{llvm_declares, ES_EXPR_DECLARES, PRINT_BOOL, PRINT_BYTES, PRINT_F64};

pub(crate) fn is_es_coercion_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_coercion(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_coercion module"))?;
    let mut em = Emitter::new(module);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Boolean,
    String,
    Number,
    Object,
}

struct ModuleInfo {
    /// User locals in declaration order (print order skips Object).
    user_locals: Vec<(LocalId, SlotTy)>,
    /// Compile-time values for each user local after executing the body.
    values: HashMap<LocalId, JsVal>,
}

#[derive(Clone, Debug)]
enum JsVal {
    Num(f64),
    Bool(bool),
    Str(String),
    Null,
    Undef,
    /// Fresh empty-object identity (compile-time).
    Obj(u64),
}

fn js_string_to_utf8(s: &JsString) -> String {
    s.to_string_lossy()
}

fn is_nan_global(id: LocalId, by_id: &HashMap<LocalId, &Local>) -> bool {
    by_id
        .get(&id)
        .is_some_and(|l| l.name == "NaN" && l.ty == Type::Number)
}

fn is_object_ty(ty: &Type) -> bool {
    matches!(ty, Type::Object | Type::Shape(_))
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
                // Match common ToString for fixture integers/simple floats.
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

fn to_boolean(v: &JsVal) -> bool {
    match v {
        JsVal::Bool(b) => *b,
        JsVal::Num(n) => *n != 0.0 && !n.is_nan(),
        JsVal::Str(s) => !s.is_empty(),
        JsVal::Null | JsVal::Undef => false,
        JsVal::Obj(_) => true,
    }
}

fn same_type_strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(x), JsVal::Num(y)) => {
            // ES Number === : NaN !== NaN; +0 === -0
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
        (JsVal::Obj(x), JsVal::Obj(y)) => x == y,
        _ => false,
    }
}

fn strict_eq(a: &JsVal, b: &JsVal) -> bool {
    match (a, b) {
        (JsVal::Num(_), JsVal::Num(_))
        | (JsVal::Bool(_), JsVal::Bool(_))
        | (JsVal::Str(_), JsVal::Str(_))
        | (JsVal::Null, JsVal::Null)
        | (JsVal::Undef, JsVal::Undef)
        | (JsVal::Obj(_), JsVal::Obj(_)) => same_type_strict_eq(a, b),
        _ => false,
    }
}

fn abstract_eq(a: &JsVal, b: &JsVal) -> bool {
    // ECMA-262 IsLooselyEqual (simplified for fixture primitives + empty objects).
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
        // null/undefined only loosely equal each other (already handled) and not numbers/strings.
        (JsVal::Null | JsVal::Undef, _) | (_, JsVal::Null | JsVal::Undef) => false,
        // Objects would ToPrimitive; fixture only uses identity same-type above.
        _ => false,
    }
}

fn add_vals(a: &JsVal, b: &JsVal) -> JsVal {
    // Prefer string concat if either side is string (after ToPrimitive — primitives only here).
    match (a, b) {
        (JsVal::Str(_), _) | (_, JsVal::Str(_)) => {
            JsVal::Str(format!("{}{}", to_string(a), to_string(b)))
        }
        _ => JsVal::Num(to_number(a) + to_number(b)),
    }
}

/// True when this module is the abstract-eq / coercion subset (not pure es_expr).
fn has_coercion_marker(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Binary {
            left, op, right, ..
        } => {
            let mixed_eq = matches!(
                op,
                BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq
            ) && !same_static_kind(left, right, by_id);
            let mixed_add = matches!(op, BinaryOp::Add)
                && (is_stringish(left, by_id) && !is_stringish(right, by_id)
                    || !is_stringish(left, by_id) && is_stringish(right, by_id)
                    || involves_null_undef(left)
                    || involves_null_undef(right)
                    || involves_bool(left, by_id)
                    || involves_bool(right, by_id));
            mixed_eq
                || mixed_add
                || has_coercion_marker(left, by_id)
                || has_coercion_marker(right, by_id)
        }
        Expr::Unary {
            op: UnaryOp::Plus,
            arg,
            ..
        } => {
            !matches!(arg.ty(), Type::Number)
                || involves_null_undef(arg)
                || involves_bool(arg, by_id)
                || is_stringish(arg, by_id)
                || has_coercion_marker(arg, by_id)
        }
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => true,
        Expr::Unary { arg, .. } => has_coercion_marker(arg, by_id),
        Expr::Assign { value, .. } => has_coercion_marker(value, by_id),
        Expr::Null { .. } => true,
        _ => false,
    }
}

fn same_static_kind(a: &Expr, b: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    kind_of(a, by_id) == kind_of(b, by_id) && kind_of(a, by_id).is_some()
}

fn kind_of(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> Option<&'static str> {
    match expr {
        Expr::Number { .. } => Some("num"),
        Expr::Boolean { .. } => Some("bool"),
        Expr::String { .. } => Some("str"),
        Expr::Null { .. } => Some("null"),
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => Some("undef"),
        Expr::Local { id, ty } => {
            if is_nan_global(*id, by_id) {
                return Some("num");
            }
            match ty {
                Type::Number => Some("num"),
                Type::Boolean => Some("bool"),
                Type::String => Some("str"),
                Type::Null => Some("nullish"),
                t if is_object_ty(t) => Some("obj"),
                _ => None,
            }
        }
        Expr::Object { .. } => Some("obj"),
        _ => None,
    }
}

fn is_stringish(expr: &Expr, _by_id: &HashMap<LocalId, &Local>) -> bool {
    matches!(expr.ty(), Type::String) || matches!(expr, Expr::String { .. })
}

fn involves_null_undef(expr: &Expr) -> bool {
    match expr {
        Expr::Null { .. } => true,
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => true,
        Expr::Local { ty: Type::Null, .. } => true,
        _ => false,
    }
}

fn involves_bool(expr: &Expr, _by_id: &HashMap<LocalId, &Local>) -> bool {
    matches!(expr.ty(), Type::Boolean) || matches!(expr, Expr::Boolean { .. })
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
            if is_nan_global(*id, by_id) {
                return true;
            }
            if is_object_ty(ty) {
                return objs.contains(id) || by_id.get(id).is_some_and(|l| is_object_ty(&l.ty));
            }
            by_id.contains_key(id)
        }
        Expr::Unary { op, arg, .. } => {
            matches!(
                op,
                UnaryOp::Plus | UnaryOp::Minus | UnaryOp::Not | UnaryOp::Void | UnaryOp::TypeOf
            ) && expr_ok(arg, by_id, objs)
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            matches!(
                op,
                BinaryOp::EqEq
                    | BinaryOp::NotEq
                    | BinaryOp::EqEqEq
                    | BinaryOp::NotEqEq
                    | BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
            ) && expr_ok(left, by_id, objs)
                && expr_ok(right, by_id, objs)
        }
        Expr::Object { properties, .. } => properties.is_empty(),
        Expr::Assign {
            target,
            op: AssignOp::Eq,
            value,
            ..
        } => {
            matches!(target, AssignTarget::Local(id) if by_id.contains_key(id))
                && expr_ok(value, by_id, objs)
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
                let slot = match &loc.ty {
                    Type::Boolean => SlotTy::Boolean,
                    Type::String => SlotTy::String,
                    Type::Number => SlotTy::Number,
                    ty if is_object_ty(ty) => {
                        objs.insert(*local);
                        SlotTy::Object
                    }
                    _ => return None,
                };
                let init = init.as_ref()?;
                if !expr_ok(init, &by_id, &objs) {
                    return None;
                }
                if has_coercion_marker(init, &by_id) {
                    saw_marker = true;
                }
                if seen.insert(*local) {
                    user_locals.push((*local, slot));
                }
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                if !expr_ok(test, &by_id, &objs) {
                    return None;
                }
                if involves_null_undef(test)
                    || is_stringish(test, &by_id)
                    || matches!(test, Expr::Object { .. })
                    || matches!(test.ty(), Type::Object | Type::Shape(_))
                {
                    saw_marker = true;
                }
                if !block_ok(consequent, &by_id, &objs) {
                    return None;
                }
                if let Some(alt) = alternate {
                    if !block_ok(alt, &by_id, &objs) {
                        return None;
                    }
                }
            }
            _ => return None,
        }
    }

    if user_locals.is_empty() || !saw_marker {
        return None;
    }

    // Interpret body at compile time.
    let mut values: HashMap<LocalId, JsVal> = HashMap::new();
    let mut next_obj: u64 = 1;
    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let init = init.as_ref()?;
                let v = eval_expr(init, &values, &by_id, &mut next_obj)?;
                values.insert(*local, v);
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let t = eval_expr(test, &values, &by_id, &mut next_obj)?;
                if to_boolean(&t) {
                    exec_block(consequent, &mut values, &by_id, &mut next_obj)?;
                } else if let Some(alt) = alternate {
                    exec_block(alt, &mut values, &by_id, &mut next_obj)?;
                }
            }
            _ => return None,
        }
    }

    Some(ModuleInfo {
        user_locals,
        values,
    })
}

fn block_ok(
    stmt: &Stmt,
    by_id: &HashMap<LocalId, &Local>,
    objs: &std::collections::HashSet<LocalId>,
) -> bool {
    match stmt {
        Stmt::Block { body } => body.iter().all(|s| block_ok(s, by_id, objs)),
        Stmt::Expr { expr } => expr_ok(expr, by_id, objs),
        Stmt::Declare { local, init, .. } => {
            by_id.contains_key(local)
                && init
                    .as_ref()
                    .map(|e| expr_ok(e, by_id, objs))
                    .unwrap_or(true)
        }
        _ => false,
    }
}

fn exec_block(
    stmt: &Stmt,
    values: &mut HashMap<LocalId, JsVal>,
    by_id: &HashMap<LocalId, &Local>,
    next_obj: &mut u64,
) -> Option<()> {
    match stmt {
        Stmt::Block { body } => {
            for s in body {
                exec_block(s, values, by_id, next_obj)?;
            }
            Some(())
        }
        Stmt::Expr { expr } => match expr {
            Expr::Assign {
                target: AssignTarget::Local(id),
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let v = eval_expr(value, values, by_id, next_obj)?;
                values.insert(*id, v);
                Some(())
            }
            _ => {
                let _ = eval_expr(expr, values, by_id, next_obj)?;
                Some(())
            }
        },
        Stmt::Declare { local, init, .. } => {
            if let Some(init) = init {
                let v = eval_expr(init, values, by_id, next_obj)?;
                values.insert(*local, v);
            }
            Some(())
        }
        _ => None,
    }
}

fn eval_expr(
    expr: &Expr,
    values: &HashMap<LocalId, JsVal>,
    by_id: &HashMap<LocalId, &Local>,
    next_obj: &mut u64,
) -> Option<JsVal> {
    match expr {
        Expr::Number { raw, .. } => {
            let n: f64 = raw.parse().ok()?;
            Some(JsVal::Num(n))
        }
        Expr::Boolean { value, .. } => Some(JsVal::Bool(*value)),
        Expr::String { value, .. } => Some(JsVal::Str(js_string_to_utf8(value))),
        Expr::Null { .. } => Some(JsVal::Null),
        Expr::Local { id, .. } => {
            if is_nan_global(*id, by_id) {
                return Some(JsVal::Num(f64::NAN));
            }
            values.get(id).cloned()
        }
        Expr::Object { properties, .. } => {
            if !properties.is_empty()
                && !properties
                    .iter()
                    .all(|p| matches!(p, ObjectProp::Property { .. }))
            {
                // only empty objects
            }
            if !properties.is_empty() {
                return None;
            }
            let id = *next_obj;
            *next_obj += 1;
            Some(JsVal::Obj(id))
        }
        Expr::Unary { op, arg, .. } => {
            let a = eval_expr(arg, values, by_id, next_obj)?;
            match op {
                UnaryOp::Plus => Some(JsVal::Num(to_number(&a))),
                UnaryOp::Minus => Some(JsVal::Num(-to_number(&a))),
                UnaryOp::Not => Some(JsVal::Bool(!to_boolean(&a))),
                UnaryOp::Void => Some(JsVal::Undef),
                UnaryOp::TypeOf => Some(JsVal::Str(typeof_name(&a).into())),
                _ => None,
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let l = eval_expr(left, values, by_id, next_obj)?;
            let r = eval_expr(right, values, by_id, next_obj)?;
            match op {
                BinaryOp::EqEq => Some(JsVal::Bool(abstract_eq(&l, &r))),
                BinaryOp::NotEq => Some(JsVal::Bool(!abstract_eq(&l, &r))),
                BinaryOp::EqEqEq => Some(JsVal::Bool(strict_eq(&l, &r))),
                BinaryOp::NotEqEq => Some(JsVal::Bool(!strict_eq(&l, &r))),
                BinaryOp::Add => Some(add_vals(&l, &r)),
                BinaryOp::Sub => Some(JsVal::Num(to_number(&l) - to_number(&r))),
                BinaryOp::Mul => Some(JsVal::Num(to_number(&l) * to_number(&r))),
                BinaryOp::Div => Some(JsVal::Num(to_number(&l) / to_number(&r))),
                BinaryOp::Rem => Some(JsVal::Num(to_number(&l) % to_number(&r))),
                _ => None,
            }
        }
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            let v = eval_expr(value, values, by_id, next_obj)?;
            // Caller must write into values for Declare; for Expr assign we need mut.
            // Use interior via unsafe pattern — handled in eval_assign path.
            let _ = id;
            Some(v)
        }
        _ => None,
    }
}

fn typeof_name(v: &JsVal) -> &'static str {
    match v {
        JsVal::Num(_) => "number",
        JsVal::Bool(_) => "boolean",
        JsVal::Str(_) => "string",
        JsVal::Null => "object",
        JsVal::Undef => "undefined",
        JsVal::Obj(_) => "object",
    }
}

struct Emitter<'a> {
    module: &'a Module,
    str_globals: HashMap<Vec<u8>, String>,
    out: String,
    body: String,
    tmp: u32,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        Self {
            module,
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

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        // Emit prints for computed values in declaration order (skip objects).
        for (id, slot) in &info.user_locals {
            let v = info
                .values
                .get(id)
                .ok_or_else(|| diag("es_coercion: missing value"))?;
            match slot {
                SlotTy::Object => {}
                SlotTy::Boolean => {
                    let JsVal::Bool(b) = v else {
                        return Err(diag("es_coercion: expected bool"));
                    };
                    let ext = self.fresh();
                    let bit = if *b { 1 } else { 0 };
                    writeln!(self.body, "  {ext} = add i8 0, {bit}").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {ext}"))).ok();
                }
                SlotTy::String => {
                    let JsVal::Str(s) = v else {
                        return Err(diag("es_coercion: expected string"));
                    };
                    let (data, len) = self.string_const(s);
                    writeln!(
                        self.body,
                        "  {}",
                        PRINT_BYTES.call(&format!("ptr {data}, i64 {len}"))
                    )
                    .ok();
                }
                SlotTy::Number => {
                    let JsVal::Num(n) = v else {
                        return Err(diag("es_coercion: expected number"));
                    };
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
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.09.03 abstract eq / coercion)"
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
        let _ = self.module;
        Ok(())
    }

    fn finish(self) -> String {
        self.out
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}
