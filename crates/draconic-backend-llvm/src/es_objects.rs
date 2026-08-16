//! N08.04.01: native observations for ES object literals + property access
//! (E04.01 / `es/objects/object_lit_access`).
//!
//! Object values are Runtime GC heap ptrs; number props are stored as
//! `inttoptr` of integer bit-patterns (fixture uses small integers). Nested
//! objects store GC ptrs. Number locals from member reads are printed via
//! `print_f64`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, GC_INIT, OBJECT_GET, OBJECT_SET, PRINT_F64,
};

pub(crate) fn is_es_objects_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_objects(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_objects module"))?;
    let mut em = Emitter::new(module);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    Object,
}

struct ModuleInfo {
    /// All allocated locals (object + number) with slot kinds.
    slots: Vec<(LocalId, SlotTy)>,
    /// Number user locals in declare order (printed).
    number_locals: Vec<LocalId>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut slots = Vec::new();
    let mut number_locals = Vec::new();
    let mut has_object = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let loc = by_id.get(local)?;
                let init = init.as_ref()?;
                if is_object_slot_ty(&loc.ty) || expr_is_object_init(init) {
                    if !object_expr_ok(init, &by_id) {
                        return None;
                    }
                    has_object = true;
                    slots.push((*local, SlotTy::Object));
                } else if is_number_slot_ty(&loc.ty) || expr_is_number_init(init) {
                    if !number_expr_ok(init, &by_id) {
                        return None;
                    }
                    slots.push((*local, SlotTy::Number));
                    number_locals.push(*local);
                } else {
                    return None;
                }
            }
            _ => return None,
        }
    }

    if !has_object || number_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        slots,
        number_locals,
    })
}

fn is_object_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Object | Type::Shape(_))
}

fn is_number_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Number | Type::Any)
}

fn expr_is_object_init(expr: &Expr) -> bool {
    match expr {
        Expr::Object { .. } => true,
        Expr::Local { ty, .. } => is_object_slot_ty(ty),
        Expr::Member { ty, .. } => is_object_slot_ty(ty),
        _ => false,
    }
}

fn expr_is_number_init(expr: &Expr) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { ty, .. } => matches!(ty, Type::Number),
        // Property reads: Number or untyped Any (computed string keys).
        Expr::Member {
            ty: Type::Number | Type::Any,
            ..
        } => true,
        _ => false,
    }
}

fn object_expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { key, value } => {
                        if !static_key_ok(key) {
                            return false;
                        }
                        // Value may be number or nested object.
                        if object_expr_ok(value, by_id) {
                            continue;
                        }
                        if number_expr_ok(value, by_id) {
                            continue;
                        }
                        return false;
                    }
                    ObjectProp::Accessor { .. } | ObjectProp::Spread(_) => return false,
                }
            }
            true
        }
        Expr::Local { id, ty } => {
            is_object_slot_ty(ty)
                || by_id
                    .get(id)
                    .is_some_and(|l| is_object_slot_ty(&l.ty) || matches!(l.ty, Type::Any))
        }
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => !*optional && object_expr_ok(object, by_id) && member_key_ok(property),
        _ => false,
    }
}

fn number_expr_ok(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::Number | Type::Any)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => !*optional && object_expr_ok(object, by_id) && member_key_ok(property),
        _ => false,
    }
}

fn static_key_ok(key: &ObjectPropKey) -> bool {
    matches!(key, ObjectPropKey::Static(_))
}

fn member_key_ok(property: &Expr) -> bool {
    matches!(property, Expr::String { .. })
}

struct Emitter<'a> {
    module: &'a Module,
    slot_of: HashMap<LocalId, SlotTy>,
    allocas: HashMap<LocalId, String>,
    str_globals: Vec<(String, String)>,
    out: String,
    body: String,
    tmp: usize,
    str_n: usize,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        Self {
            module,
            slot_of: HashMap::new(),
            allocas: HashMap::new(),
            str_globals: Vec::new(),
            out: String::new(),
            body: String::new(),
            tmp: 0,
            str_n: 0,
        }
    }

    fn finish(self) -> String {
        self.out
    }

    fn fresh(&mut self) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("%t{t}")
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        for (id, ty) in &info.slots {
            self.slot_of.insert(*id, *ty);
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.04.01 ES object lit + property access via Runtime ABI)"
        )
        .ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[GC_INIT, ALLOC_OBJECT, OBJECT_SET, OBJECT_GET, PRINT_F64])
        )
        .ok();
        writeln!(self.out).ok();

        // Emit body first so string globals are collected.
        for (id, kind) in &info.slots {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            match kind {
                SlotTy::Number => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                    writeln!(self.body, "  store double 0.00000000000000000e+00, ptr {ptr}").ok();
                }
                SlotTy::Object => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for id in &info.number_locals {
            let ptr = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("internal: print missing alloca"))?;
            let v = self.fresh();
            writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
            writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
        }

        for (content, gname) in self.str_globals.clone() {
            let n = content.len() + 1;
            let esc = escape_llvm_string(&content);
            writeln!(
                self.out,
                "@{gname} = private unnamed_addr constant [{n} x i8] c\"{esc}\\00\", align 1"
            )
            .ok();
        }
        if !self.str_globals.is_empty() {
            writeln!(self.out).ok();
        }

        writeln!(self.out, "define i32 @main() {{").ok();
        writeln!(self.out, "entry:").ok();
        writeln!(self.out, "  {}", GC_INIT.call("")).ok();
        self.out.push_str(&self.body);
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                let kind = *self
                    .slot_of
                    .get(local)
                    .ok_or_else(|| diag("es_objects: declare unknown slot"))?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        let ptr = self.allocas.get(local).cloned().unwrap();
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Object => {
                        let v = self.emit_object_expr(init)?;
                        let ptr = self.allocas.get(local).cloned().unwrap();
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            _ => Err(diag("es_objects: unsupported stmt")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => format_number_const(raw),
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_objects: number local unknown"))?;
                if kind != SlotTy::Number {
                    return Err(diag("es_objects: expected number local"));
                }
                let ptr = self.allocas.get(id).cloned().unwrap();
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_objects: optional member not supported"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                let raw = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_GET.call_to(&raw, &format!("ptr {obj}, ptr {key}"))
                )
                .ok();
                let i = self.fresh();
                writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
                let d = self.fresh();
                writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                Ok(d)
            }
            _ => Err(diag("es_objects: unsupported number expr")),
        }
    }

    fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Object { properties, .. } => {
                let obj = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
                for p in properties {
                    match p {
                        ObjectProp::Property { key, value } => {
                            let key_s = match key {
                                ObjectPropKey::Static(s) => s.to_string_lossy(),
                                ObjectPropKey::Computed(_) => {
                                    return Err(diag("es_objects: computed keys not supported"));
                                }
                            };
                            let key_ptr = self.string_const(&key_s)?;
                            let val_ptr = if object_value_is_object(value) {
                                self.emit_object_expr(value)?
                            } else {
                                let n = self.emit_number_expr(value)?;
                                let i = self.fresh();
                                writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                                let p = self.fresh();
                                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                                p
                            };
                            writeln!(
                                self.body,
                                "  {}",
                                OBJECT_SET.call(&format!(
                                    "ptr {obj}, ptr {key_ptr}, ptr {val_ptr}"
                                ))
                            )
                            .ok();
                        }
                        _ => return Err(diag("es_objects: only plain properties")),
                    }
                }
                Ok(obj)
            }
            Expr::Local { id, .. } => {
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_objects: object local unknown"))?;
                if kind != SlotTy::Object {
                    return Err(diag("es_objects: expected object local"));
                }
                let ptr = self.allocas.get(id).cloned().unwrap();
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_objects: optional member not supported"));
                }
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_GET.call_to(&t, &format!("ptr {obj}, ptr {key}"))
                )
                .ok();
                Ok(t)
            }
            _ => Err(diag("es_objects: unsupported object expr")),
        }
    }

    fn member_key_cstr(&mut self, property: &Expr) -> Result<String, Diagnostic> {
        match property {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            _ => Err(diag("es_objects: member key must be string")),
        }
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_obj_str.{}", self.str_n);
            self.str_n += 1;
            self.str_globals.push((s.to_string(), g.clone()));
            g
        };
        let t = self.fresh();
        let n = s.len() + 1;
        writeln!(
            self.body,
            "  {t} = getelementptr inbounds [{n} x i8], ptr @{gname}, i64 0, i64 0"
        )
        .ok();
        Ok(t)
    }
}

fn object_value_is_object(expr: &Expr) -> bool {
    match expr {
        Expr::Object { .. } => true,
        Expr::Member { ty, .. } | Expr::Local { ty, .. } => is_object_slot_ty(ty),
        _ => false,
    }
}

fn format_number_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(format!("invalid number literal {raw}")))?;
    Ok(format!("{f:.17e}"))
}

fn escape_llvm_string(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            c if (0x20..0x7f).contains(&c) => out.push(c as char),
            c => out.push_str(&format!("\\{c:02X}")),
        }
    }
    out
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
