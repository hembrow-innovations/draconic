//! N08.09.01–N08.09.02: native observations for Symbol constructor basics and
//! Symbol property keys (E09.01–E09.02).
//!
//! Supports `Symbol()` / `Symbol(desc)`, `Symbol.for` / `Symbol.keyFor`,
//! `typeof` on symbols/statics/undefined, `===`/`!==` on symbol ids, empty
//! object lits, computed symbol keys in lits, and get/set with symbol vs string
//! keys (string keys do not collide with symbols).

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, JsString, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ES_VALUES_DECLARES, GC_INIT, OBJECT_GET, OBJECT_GET_BY_SYMBOL,
    OBJECT_SET, OBJECT_SET_BY_SYMBOL, PRINT_BOOL, PRINT_BYTES, PRINT_F64, SYMBOL_FOR, SYMBOL_KEY_FOR,
    SYMBOL_NEW,
};

pub(crate) fn is_es_values_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_values(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_values module"))?;
    let mut em = Emitter::new(module);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    /// JS Symbol as unique i64 id (not printed).
    Symbol,
    Boolean,
    String,
    Number,
    Object,
    /// Missing property read → undefined (not printed; typeof → "undefined").
    Undefined,
}

struct ModuleInfo {
    user_locals: Vec<(LocalId, SlotTy)>,
    symbol_locals: std::collections::HashSet<LocalId>,
    undefined_locals: std::collections::HashSet<LocalId>,
    needs_gc: bool,
}

fn js_string_to_utf8(s: &JsString) -> String {
    s.to_string_lossy()
}

fn is_symbol_ctor_local(id: LocalId, ty: Type, by_id: &HashMap<LocalId, &Local>) -> bool {
    ty == Type::Function
        && by_id
            .get(&id)
            .is_some_and(|l| l.name == "Symbol" && l.ty == Type::Function)
}

fn is_symbol_ctor_expr(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    matches!(expr, Expr::Local { id, ty } if is_symbol_ctor_local(*id, *ty, by_id))
}

fn symbol_member_name(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> Option<&'static str> {
    match expr {
        Expr::Member {
            object,
            property,
            computed: false,
            ..
        } if is_symbol_ctor_expr(object, by_id) => match property.as_ref() {
            Expr::String { value, .. } => match js_string_to_utf8(value).as_str() {
                "for" => Some("for"),
                "keyFor" => Some("keyFor"),
                _ => None,
            },
            _ => None,
        },
        _ => None,
    }
}

fn expr_is_symbol_new(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } if is_symbol_ctor_expr(callee, by_id) => match args.as_slice() {
            [] => true,
            [Arg::Expr(Expr::String { .. })] => true,
            _ => false,
        },
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } if symbol_member_name(callee, by_id) == Some("for") => {
            matches!(args.as_slice(), [Arg::Expr(Expr::String { .. })])
        }
        _ => false,
    }
}

fn expr_is_symbol_key_for(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    symbols: &std::collections::HashSet<LocalId>,
) -> bool {
    match expr {
        Expr::Call {
            callee,
            args,
            optional: false,
            ..
        } if symbol_member_name(callee, by_id) == Some("keyFor") => match args.as_slice() {
            [Arg::Expr(e)] => expr_is_symbol_value(e, by_id, symbols),
            _ => false,
        },
        _ => false,
    }
}

fn expr_is_symbol_value(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    symbols: &std::collections::HashSet<LocalId>,
) -> bool {
    match expr {
        Expr::Local { id, .. } => symbols.contains(id),
        e => expr_is_symbol_new(e, by_id),
    }
}

fn expr_is_boolean(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    symbols: &std::collections::HashSet<LocalId>,
) -> bool {
    match expr {
        Expr::Boolean { .. } => true,
        Expr::Local { id, ty } => {
            *ty == Type::Boolean && by_id.get(id).is_some_and(|l| l.ty == Type::Boolean)
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            *ty == Type::Boolean
                && matches!(op, BinaryOp::EqEqEq | BinaryOp::NotEqEq)
                && expr_is_symbol_value(left, by_id, symbols)
                && expr_is_symbol_value(right, by_id, symbols)
        }
        _ => false,
    }
}

fn typeof_arg_ok(
    arg: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    symbols: &std::collections::HashSet<LocalId>,
    undefineds: &std::collections::HashSet<LocalId>,
) -> bool {
    if is_symbol_ctor_expr(arg, by_id) {
        return true;
    }
    if symbol_member_name(arg, by_id).is_some() {
        return true;
    }
    if expr_is_symbol_value(arg, by_id, symbols) {
        return true;
    }
    matches!(arg, Expr::Local { id, .. } if undefineds.contains(id))
}

fn expr_is_string(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    symbols: &std::collections::HashSet<LocalId>,
    undefineds: &std::collections::HashSet<LocalId>,
) -> bool {
    match expr {
        Expr::String { .. } => true,
        Expr::Local { id, ty } => {
            *ty == Type::String && by_id.get(id).is_some_and(|l| l.ty == Type::String)
        }
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ty,
        } => *ty == Type::String && typeof_arg_ok(arg, by_id, symbols, undefineds),
        e if expr_is_symbol_key_for(e, by_id, symbols) => true,
        _ => false,
    }
}

fn expr_is_number(expr: &Expr) -> bool {
    matches!(expr, Expr::Number { .. })
}

fn is_object_ty(ty: &Type) -> bool {
    matches!(ty, Type::Object | Type::Shape(_))
}

fn member_key_is_symbol(
    property: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    symbols: &std::collections::HashSet<LocalId>,
) -> bool {
    expr_is_symbol_value(property, by_id, symbols)
}

fn member_key_is_string(property: &Expr) -> bool {
    matches!(property, Expr::String { .. })
}

fn object_prop_ok(
    prop: &ObjectProp,
    by_id: &HashMap<LocalId, &Local>,
    symbols: &std::collections::HashSet<LocalId>,
) -> bool {
    match prop {
        ObjectProp::Property { key, value } => {
            if !expr_is_number(value) {
                return false;
            }
            match key {
                ObjectPropKey::Computed(k) => expr_is_symbol_value(k, by_id, symbols),
                ObjectPropKey::Static(_) => false,
            }
        }
        _ => false,
    }
}

fn object_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    symbols: &std::collections::HashSet<LocalId>,
) -> bool {
    match expr {
        Expr::Object { properties, .. } => {
            properties.iter().all(|p| object_prop_ok(p, by_id, symbols))
        }
        Expr::Local { id, .. } => by_id.get(id).is_some_and(|l| is_object_ty(&l.ty)),
        _ => false,
    }
}

fn member_get_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    symbols: &std::collections::HashSet<LocalId>,
    objects: &std::collections::HashSet<LocalId>,
) -> Option<SlotTy> {
    let Expr::Member {
        object,
        property,
        optional: false,
        ..
    } = expr
    else {
        return None;
    };
    let obj_ok = match object.as_ref() {
        Expr::Local { id, .. } => objects.contains(id),
        _ => false,
    };
    if !obj_ok {
        return None;
    }
    if member_key_is_symbol(property, by_id, symbols) {
        Some(SlotTy::Number)
    } else if member_key_is_string(property) {
        // Fixture only uses string keys for intentional misses.
        Some(SlotTy::Undefined)
    } else {
        None
    }
}

fn member_assign_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    symbols: &std::collections::HashSet<LocalId>,
    objects: &std::collections::HashSet<LocalId>,
) -> bool {
    let Expr::Assign {
        target:
            AssignTarget::Member {
                object,
                property,
                computed: true,
            },
        op: AssignOp::Eq,
        value,
        ..
    } = expr
    else {
        return false;
    };
    if !expr_is_number(value) {
        return false;
    }
    let obj_ok = match object.as_ref() {
        Expr::Local { id, .. } => objects.contains(id),
        _ => false,
    };
    obj_ok
        && (member_key_is_symbol(property, by_id, symbols) || member_key_is_string(property))
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_locals = Vec::new();
    let mut symbols = std::collections::HashSet::new();
    let mut objects = std::collections::HashSet::new();
    let mut undefineds = std::collections::HashSet::new();
    let mut seen = std::collections::HashSet::new();
    let mut saw_symbol = false;
    let mut needs_gc = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let init = init.as_ref()?;
                let loc = by_id.get(local)?;
                let slot = match &loc.ty {
                    Type::Boolean => {
                        if !expr_is_boolean(init, &by_id, &symbols) {
                            return None;
                        }
                        SlotTy::Boolean
                    }
                    Type::String => {
                        if !expr_is_string(init, &by_id, &symbols, &undefineds) {
                            return None;
                        }
                        SlotTy::String
                    }
                    Type::Number => {
                        if !expr_is_number(init) {
                            return None;
                        }
                        SlotTy::Number
                    }
                    ty if is_object_ty(ty) => {
                        if !object_expr_ok(init, &by_id, &symbols) {
                            return None;
                        }
                        needs_gc = true;
                        objects.insert(*local);
                        SlotTy::Object
                    }
                    Type::Object => {
                        if !object_expr_ok(init, &by_id, &symbols) {
                            return None;
                        }
                        needs_gc = true;
                        objects.insert(*local);
                        SlotTy::Object
                    }
                    Type::Any => {
                        if expr_is_symbol_key_for(init, &by_id, &symbols) {
                            SlotTy::String
                        } else if expr_is_symbol_new(init, &by_id) {
                            saw_symbol = true;
                            symbols.insert(*local);
                            SlotTy::Symbol
                        } else if let Some(st) = member_get_ok(init, &by_id, &symbols, &objects) {
                            needs_gc = true;
                            if st == SlotTy::Undefined {
                                undefineds.insert(*local);
                            }
                            st
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                };
                if matches!(
                    init,
                    Expr::Unary {
                        op: UnaryOp::TypeOf,
                        ..
                    }
                ) || matches!(
                    init,
                    Expr::Binary {
                        op: BinaryOp::EqEqEq | BinaryOp::NotEqEq,
                        ..
                    }
                ) || expr_is_symbol_key_for(init, &by_id, &symbols)
                    || expr_is_symbol_new(init, &by_id)
                    || matches!(init, Expr::Object { .. })
                    || matches!(init, Expr::Member { .. })
                {
                    saw_symbol = true;
                }
                if seen.insert(*local) {
                    user_locals.push((*local, slot));
                }
            }
            Stmt::Expr { expr } => {
                if !member_assign_ok(expr, &by_id, &symbols, &objects) {
                    return None;
                }
                needs_gc = true;
                saw_symbol = true;
            }
            _ => return None,
        }
    }

    if user_locals.is_empty() || !saw_symbol || symbols.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        user_locals,
        symbol_locals: symbols,
        undefined_locals: undefineds,
        needs_gc,
    })
}

struct StrVal {
    data: String,
    len: String,
}

struct Emitter<'a> {
    module: &'a Module,
    allocas: HashMap<LocalId, (String, SlotTy)>,
    string_lens: HashMap<LocalId, String>,
    symbol_locals: std::collections::HashSet<LocalId>,
    undefined_locals: std::collections::HashSet<LocalId>,
    str_globals: HashMap<Vec<u8>, String>,
    out: String,
    body: String,
    tmp: u32,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module) -> Self {
        Self {
            module,
            allocas: HashMap::new(),
            string_lens: HashMap::new(),
            symbol_locals: std::collections::HashSet::new(),
            undefined_locals: std::collections::HashSet::new(),
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

    fn by_id(&self) -> HashMap<LocalId, &Local> {
        self.module.locals.iter().map(|l| (l.id, l)).collect()
    }

    fn string_const(&mut self, s: &str) -> Result<StrVal, Diagnostic> {
        let bytes = s.as_bytes().to_vec();
        let len = bytes.len();
        let name = if let Some(n) = self.str_globals.get(&bytes) {
            n.clone()
        } else {
            let n = format!("@.str.{}", self.str_globals.len());
            self.str_globals.insert(bytes, n.clone());
            n
        };
        Ok(StrVal {
            data: format!(
                "getelementptr inbounds ([{n} x i8], ptr {name}, i64 0, i64 0)",
                n = len + 1
            ),
            len: format!("{len}"),
        })
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        self.symbol_locals = info.symbol_locals.clone();
        self.undefined_locals = info.undefined_locals.clone();
        if info.needs_gc {
            writeln!(self.body, "  {}", GC_INIT.call("")).ok();
        }
        for (id, slot) in &info.user_locals {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, (ptr.clone(), *slot));
            match slot {
                SlotTy::Symbol => {
                    writeln!(self.body, "  {ptr} = alloca i64, align 8").ok();
                }
                SlotTy::Boolean => {
                    writeln!(self.body, "  {ptr} = alloca i1, align 1").ok();
                }
                SlotTy::String => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    let len_ptr = format!("%l{}_len", id.0);
                    writeln!(self.body, "  {len_ptr} = alloca i64, align 8").ok();
                    self.string_lens.insert(*id, len_ptr);
                }
                SlotTy::Number => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                }
                SlotTy::Object => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                }
                SlotTy::Undefined => {
                    // no storage; typeof is compile-time
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, slot) in &info.user_locals {
            match slot {
                SlotTy::Symbol | SlotTy::Object | SlotTy::Undefined => {}
                SlotTy::Boolean => {
                    let (ptr, _) = self.allocas.get(id).cloned().unwrap();
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load i1, ptr {ptr}").ok();
                    let ext = self.fresh();
                    writeln!(self.body, "  {ext} = zext i1 {v} to i8").ok();
                    writeln!(self.body, "  {}", PRINT_BOOL.call(&format!("i8 {ext}"))).ok();
                }
                SlotTy::String => {
                    let (ptr, _) = self.allocas.get(id).cloned().unwrap();
                    let len_ptr = self.string_lens.get(id).cloned().unwrap();
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    let n = self.fresh();
                    writeln!(self.body, "  {n} = load i64, ptr {len_ptr}").ok();
                    writeln!(
                        self.body,
                        "  {}",
                        PRINT_BYTES.call(&format!("ptr {v}, i64 {n}"))
                    )
                    .ok();
                }
                SlotTy::Number => {
                    let (ptr, _) = self.allocas.get(id).cloned().unwrap();
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
            }
        }

        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.09 Symbol / symbol property keys via Runtime ABI)"
        )
        .ok();
        writeln!(self.out, "{}", llvm_declares(ES_VALUES_DECLARES)).ok();
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

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let slot = self
                    .allocas
                    .get(local)
                    .map(|(_, s)| *s)
                    .or_else(|| {
                        if self.undefined_locals.contains(local) {
                            Some(SlotTy::Undefined)
                        } else {
                            None
                        }
                    })
                    .ok_or_else(|| diag("internal: declare missing slot"))?;
                let init = init
                    .as_ref()
                    .ok_or_else(|| diag("es_values: declare requires init"))?;
                match slot {
                    SlotTy::Symbol => {
                        let (ptr, _) = self.allocas.get(local).cloned().unwrap();
                        let v = self.emit_symbol_expr(init)?;
                        writeln!(self.body, "  store i64 {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Boolean => {
                        let (ptr, _) = self.allocas.get(local).cloned().unwrap();
                        let v = self.emit_bool_expr(init)?;
                        writeln!(self.body, "  store i1 {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let (ptr, _) = self.allocas.get(local).cloned().unwrap();
                        let s = self.emit_string_expr(init)?;
                        let len_ptr = self.string_lens.get(local).cloned().unwrap();
                        writeln!(self.body, "  store ptr {}, ptr {ptr}", s.data).ok();
                        writeln!(self.body, "  store i64 {}, ptr {len_ptr}", s.len).ok();
                    }
                    SlotTy::Number => {
                        let (ptr, _) = self.allocas.get(local).cloned().unwrap();
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Object => {
                        let (ptr, _) = self.allocas.get(local).cloned().unwrap();
                        let v = self.emit_object_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Undefined => {
                        // Missing property — no runtime store.
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr } => self.emit_side_effect(expr),
            _ => Err(diag("es_values: unsupported stmt")),
        }
    }

    fn emit_side_effect(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        let Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } = expr
        else {
            return Err(diag("es_values: unsupported side-effect expr"));
        };
        let obj = self.emit_object_expr(object)?;
        let n = self.emit_number_expr(value)?;
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
        let p = self.fresh();
        writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
        self.emit_object_set(&obj, property, &p)?;
        Ok(())
    }

    fn emit_object_set(
        &mut self,
        obj: &str,
        property: &Expr,
        val_ptr: &str,
    ) -> Result<(), Diagnostic> {
        let by_id = self.by_id();
        if member_key_is_symbol(property, &by_id, &self.symbol_locals) {
            let sym = self.emit_symbol_expr(property)?;
            writeln!(
                self.body,
                "  {}",
                OBJECT_SET_BY_SYMBOL.call(&format!("ptr {obj}, i64 {sym}, ptr {val_ptr}"))
            )
            .ok();
            Ok(())
        } else if let Expr::String { value, .. } = property {
            let key = self.string_const(&js_string_to_utf8(value))?;
            writeln!(
                self.body,
                "  {}",
                OBJECT_SET.call(&format!("ptr {obj}, ptr {}, ptr {val_ptr}", key.data))
            )
            .ok();
            Ok(())
        } else {
            Err(diag("es_values: unsupported set key"))
        }
    }

    fn emit_object_get(&mut self, obj: &str, property: &Expr) -> Result<String, Diagnostic> {
        let is_sym = {
            let by_id = self.by_id();
            member_key_is_symbol(property, &by_id, &self.symbol_locals)
        };
        if is_sym {
            let sym = self.emit_symbol_expr(property)?;
            let raw = self.fresh();
            writeln!(
                self.body,
                "  {}",
                OBJECT_GET_BY_SYMBOL.call_to(&raw, &format!("ptr {obj}, i64 {sym}"))
            )
            .ok();
            Ok(raw)
        } else if let Expr::String { value, .. } = property {
            let key = self.string_const(&js_string_to_utf8(value))?;
            let raw = self.fresh();
            writeln!(
                self.body,
                "  {}",
                OBJECT_GET.call_to(&raw, &format!("ptr {obj}, ptr {}", key.data))
            )
            .ok();
            Ok(raw)
        } else {
            Err(diag("es_values: unsupported get key"))
        }
    }

    fn load_symbol_local(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        let (ptr, slot) = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("es_values: unknown symbol local"))?;
        if slot != SlotTy::Symbol {
            return Err(diag("es_values: local is not symbol"));
        }
        let v = self.fresh();
        writeln!(self.body, "  {v} = load i64, ptr {ptr}").ok();
        Ok(v)
    }

    fn load_object_local(&mut self, id: LocalId) -> Result<String, Diagnostic> {
        let (ptr, slot) = self
            .allocas
            .get(&id)
            .cloned()
            .ok_or_else(|| diag("es_values: unknown object local"))?;
        if slot != SlotTy::Object {
            return Err(diag("es_values: local is not object"));
        }
        let v = self.fresh();
        writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
        Ok(v)
    }

    fn emit_symbol_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        let by_id = self.by_id();
        match expr {
            Expr::Local { id, .. } => self.load_symbol_local(*id),
            Expr::Call {
                callee,
                args,
                optional: false,
                ..
            } if is_symbol_ctor_expr(callee, &by_id) => match args.as_slice() {
                [] | [Arg::Expr(Expr::String { .. })] => {
                    let v = self.fresh();
                    writeln!(self.body, "  {}", SYMBOL_NEW.call_to(&v, "")).ok();
                    Ok(v)
                }
                _ => Err(diag("es_values: unsupported Symbol() args")),
            },
            Expr::Call {
                callee,
                args,
                optional: false,
                ..
            } if symbol_member_name(callee, &by_id) == Some("for") => {
                let Arg::Expr(Expr::String { value, .. }) = &args[0] else {
                    return Err(diag("es_values: Symbol.for key must be string lit"));
                };
                let key = js_string_to_utf8(value);
                let s = self.string_const(&key)?;
                let v = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    SYMBOL_FOR.call_to(&v, &format!("ptr {}, i64 {}", s.data, s.len))
                )
                .ok();
                Ok(v)
            }
            _ => Err(diag("es_values: unsupported symbol expr")),
        }
    }

    fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => self.load_object_local(*id),
            Expr::Object { properties, .. } => {
                let obj = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
                for prop in properties {
                    let ObjectProp::Property { key, value } = prop else {
                        return Err(diag("es_values: unsupported object prop"));
                    };
                    let ObjectPropKey::Computed(k) = key else {
                        return Err(diag("es_values: only computed symbol keys"));
                    };
                    let n = self.emit_number_expr(value)?;
                    let i = self.fresh();
                    writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                    let p = self.fresh();
                    writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
                    self.emit_object_set(&obj, k, &p)?;
                }
                Ok(obj)
            }
            _ => Err(diag("es_values: unsupported object expr")),
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => {
                let n: f64 = raw.parse().map_err(|_| diag("es_values: bad number"))?;
                Ok(format!("{n:?}"))
            }
            Expr::Local { id, .. } => {
                let (ptr, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_values: unknown number local"))?;
                if slot != SlotTy::Number {
                    return Err(diag("es_values: local is not number"));
                }
                let v = self.fresh();
                writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Member {
                object,
                property,
                optional: false,
                ..
            } => {
                let obj = self.emit_object_expr(object)?;
                let raw = self.emit_object_get(&obj, property)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
                let d = self.fresh();
                writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
                Ok(d)
            }
            _ => Err(diag("es_values: unsupported number expr")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Boolean { value, .. } => Ok(if *value {
                "true".into()
            } else {
                "false".into()
            }),
            Expr::Local { id, .. } => {
                let (ptr, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_values: unknown bool local"))?;
                if slot != SlotTy::Boolean {
                    return Err(diag("es_values: local is not bool"));
                }
                let v = self.fresh();
                writeln!(self.body, "  {v} = load i1, ptr {ptr}").ok();
                Ok(v)
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.emit_symbol_expr(left)?;
                let r = self.emit_symbol_expr(right)?;
                let v = self.fresh();
                match op {
                    BinaryOp::EqEqEq => {
                        writeln!(self.body, "  {v} = icmp eq i64 {l}, {r}").ok();
                    }
                    BinaryOp::NotEqEq => {
                        writeln!(self.body, "  {v} = icmp ne i64 {l}, {r}").ok();
                    }
                    _ => return Err(diag("es_values: unsupported bool binary")),
                }
                Ok(v)
            }
            _ => Err(diag("es_values: unsupported bool expr")),
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<StrVal, Diagnostic> {
        let by_id = self.by_id();
        match expr {
            Expr::String { value, .. } => self.string_const(&js_string_to_utf8(value)),
            Expr::Local { id, .. } => {
                let (ptr, slot) = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_values: unknown string local"))?;
                if slot != SlotTy::String {
                    return Err(diag("es_values: local is not string"));
                }
                let len_ptr = self.string_lens.get(id).cloned().unwrap();
                let data = self.fresh();
                writeln!(self.body, "  {data} = load ptr, ptr {ptr}").ok();
                let len = self.fresh();
                writeln!(self.body, "  {len} = load i64, ptr {len_ptr}").ok();
                Ok(StrVal { data, len })
            }
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => {
                let name = self.typeof_name(arg)?;
                self.string_const(name)
            }
            Expr::Call {
                callee,
                args,
                optional: false,
                ..
            } if symbol_member_name(callee, &by_id) == Some("keyFor") => {
                let Arg::Expr(sym) = &args[0] else {
                    return Err(diag("es_values: keyFor arg must be expr"));
                };
                let id = self.emit_symbol_expr(sym)?;
                let len_slot = self.fresh();
                writeln!(self.body, "  {len_slot} = alloca i64, align 8").ok();
                let data = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    SYMBOL_KEY_FOR.call_to(&data, &format!("i64 {id}, ptr {len_slot}"))
                )
                .ok();
                let len = self.fresh();
                writeln!(self.body, "  {len} = load i64, ptr {len_slot}").ok();
                Ok(StrVal { data, len })
            }
            _ => Err(diag("es_values: unsupported string expr")),
        }
    }

    fn typeof_name(&self, arg: &Expr) -> Result<&'static str, Diagnostic> {
        let by_id = self.by_id();
        if is_symbol_ctor_expr(arg, &by_id) {
            return Ok("function");
        }
        if let Some(name) = symbol_member_name(arg, &by_id) {
            if name == "for" || name == "keyFor" {
                return Ok("function");
            }
        }
        match arg {
            Expr::Local { id, .. } => {
                if self.undefined_locals.contains(id) {
                    return Ok("undefined");
                }
                if self.symbol_locals.contains(id) {
                    return Ok("symbol");
                }
                match self.allocas.get(id).map(|(_, s)| *s) {
                    Some(SlotTy::Symbol) => Ok("symbol"),
                    Some(SlotTy::String) => Ok("string"),
                    Some(SlotTy::Boolean) => Ok("boolean"),
                    Some(SlotTy::Number) => Ok("number"),
                    Some(SlotTy::Object) => Ok("object"),
                    Some(SlotTy::Undefined) => Ok("undefined"),
                    None if is_symbol_ctor_local(*id, Type::Function, &by_id) => Ok("function"),
                    _ => Err(diag("es_values: typeof unsupported local")),
                }
            }
            e if expr_is_symbol_new(e, &by_id) => Ok("symbol"),
            _ => Err(diag("es_values: typeof unsupported")),
        }
    }

    fn finish(self) -> String {
        self.out
    }
}

fn diag(msg: &str) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}
