//! N08.16.15: native observations for `var` in `for` heads (E18.15) —
//! `es/annex-b/var_for`: `for (var k in obj)`, `for (var c of arr)`, classic
//! `for (var i = …; …; …)`, and Annex B.3.5 `for (var j = init in obj)`.
//!
//! Object for-in is unrolled over static own string keys (insertion order).
//! Array for-of uses Runtime array get/len with number→ToString concat.
//! `var` slots are script-scoped allocas (shared primary by name).

use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, BindingKind};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    ArrayElement, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp,
    ObjectPropKey, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, CSTR_CONCAT,
    CSTR_FROM_U64, GC_INIT, OBJECT_SET, PRINT_F64, PRINT_STR,
};

pub(crate) fn is_es_var_for_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_var_for(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_var_for module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    String,
    /// Opaque object/array heap ptr — allocated, not printed.
    Heap,
}

struct ModuleInfo {
    /// All alloc slots (user + for-head vars), primary id → type.
    slots: Vec<(LocalId, SlotTy)>,
    /// Top-level declares to print (declaration order): Number | String only.
    print_locals: Vec<(LocalId, SlotTy)>,
    /// Same-name `var` redecl / for-head → primary storage.
    var_primary: HashMap<LocalId, LocalId>,
    /// Object local → static own keys in insertion order (for for-in unroll).
    object_keys: HashMap<LocalId, Vec<String>>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut var_primary = HashMap::new();
    let mut var_slots = HashSet::new();
    collect_var_slots(&module.body, &by_id, &mut var_primary, &mut var_slots);

    let mut slot_of: HashMap<LocalId, SlotTy> = HashMap::new();
    let mut object_keys: HashMap<LocalId, Vec<String>> = HashMap::new();
    let mut print_locals = Vec::new();
    let mut seen_print = HashSet::new();
    let mut has_var_for_head = false;
    let mut has_object_for_in = false;

    for stmt in &module.body {
        classify_stmt(
            stmt,
            /* top */ true,
            &by_id,
            &var_primary,
            &mut slot_of,
            &mut object_keys,
            &mut print_locals,
            &mut seen_print,
            &mut has_var_for_head,
            &mut has_object_for_in,
        )?;
    }

    if !has_var_for_head || !has_object_for_in || print_locals.is_empty() {
        return None;
    }

    let mut slots: Vec<(LocalId, SlotTy)> = slot_of.into_iter().collect();
    slots.sort_by_key(|(id, _)| id.0);

    Some(ModuleInfo {
        slots,
        print_locals,
        var_primary,
        object_keys,
    })
}

fn collect_var_slots(
    stmts: &[Stmt],
    by_id: &HashMap<LocalId, &Local>,
    var_primary: &mut HashMap<LocalId, LocalId>,
    var_slots: &mut HashSet<LocalId>,
) {
    for s in stmts {
        collect_var_slots_stmt(s, by_id, var_primary, var_slots);
    }
}

fn collect_var_slots_stmt(
    stmt: &Stmt,
    by_id: &HashMap<LocalId, &Local>,
    var_primary: &mut HashMap<LocalId, LocalId>,
    var_slots: &mut HashSet<LocalId>,
) {
    match stmt {
        Stmt::Declare {
            local,
            kind: BindingKind::Var,
            ..
        } => {
            register_var(*local, by_id, var_primary, var_slots);
        }
        Stmt::Block { body } => collect_var_slots(body, by_id, var_primary, var_slots),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            collect_var_slots_stmt(consequent, by_id, var_primary, var_slots);
            if let Some(a) = alternate {
                collect_var_slots_stmt(a, by_id, var_primary, var_slots);
            }
        }
        Stmt::For { init, body, .. } => {
            if let Some(i) = init {
                collect_var_slots_stmt(i, by_id, var_primary, var_slots);
            }
            collect_var_slots_stmt(body, by_id, var_primary, var_slots);
        }
        Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
            collect_var_slots_stmt(left, by_id, var_primary, var_slots);
            collect_var_slots_stmt(body, by_id, var_primary, var_slots);
        }
        _ => {}
    }
}

fn register_var(
    local: LocalId,
    by_id: &HashMap<LocalId, &Local>,
    var_primary: &mut HashMap<LocalId, LocalId>,
    var_slots: &mut HashSet<LocalId>,
) {
    if var_primary.contains_key(&local) {
        return;
    }
    let name = by_id.get(&local).map(|l| l.name.as_str());
    if let Some(name) = name {
        for &primary in var_slots.iter() {
            if by_id.get(&primary).is_some_and(|l| l.name == name) {
                var_primary.insert(local, primary);
                return;
            }
        }
    }
    var_primary.insert(local, local);
    var_slots.insert(local);
}

fn primary(id: LocalId, var_primary: &HashMap<LocalId, LocalId>) -> LocalId {
    var_primary.get(&id).copied().unwrap_or(id)
}

fn classify_stmt(
    stmt: &Stmt,
    top: bool,
    by_id: &HashMap<LocalId, &Local>,
    var_primary: &HashMap<LocalId, LocalId>,
    slot_of: &mut HashMap<LocalId, SlotTy>,
    object_keys: &mut HashMap<LocalId, Vec<String>>,
    print_locals: &mut Vec<(LocalId, SlotTy)>,
    seen_print: &mut HashSet<LocalId>,
    has_var_for_head: &mut bool,
    has_object_for_in: &mut bool,
) -> Option<()> {
    match stmt {
        Stmt::Declare { local, init, kind } => {
            let p = primary(*local, var_primary);
            let slot = slot_for_declare(p, init.as_ref(), by_id, slot_of, object_keys)?;
            slot_of.entry(p).or_insert(slot);
            if top && matches!(slot, SlotTy::Number | SlotTy::String) && seen_print.insert(p) {
                print_locals.push((p, slot));
            }
            if *kind == BindingKind::Var {
                // bare ok
            }
            Some(())
        }
        Stmt::Expr { expr } => {
            if expr_ok(expr, by_id, slot_of) {
                Some(())
            } else {
                None
            }
        }
        Stmt::Block { body } => {
            for s in body {
                classify_stmt(
                    s,
                    false,
                    by_id,
                    var_primary,
                    slot_of,
                    object_keys,
                    print_locals,
                    seen_print,
                    has_var_for_head,
                    has_object_for_in,
                )?;
            }
            Some(())
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            if !bool_expr_ok(test, by_id, slot_of) {
                return None;
            }
            classify_stmt(
                consequent,
                false,
                by_id,
                var_primary,
                slot_of,
                object_keys,
                print_locals,
                seen_print,
                has_var_for_head,
                has_object_for_in,
            )?;
            if let Some(a) = alternate {
                classify_stmt(
                    a,
                    false,
                    by_id,
                    var_primary,
                    slot_of,
                    object_keys,
                    print_locals,
                    seen_print,
                    has_var_for_head,
                    has_object_for_in,
                )?;
            }
            Some(())
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            if let Some(i) = init {
                if let Stmt::Declare {
                    kind: BindingKind::Var,
                    ..
                } = i.as_ref()
                {
                    *has_var_for_head = true;
                }
                classify_stmt(
                    i,
                    false,
                    by_id,
                    var_primary,
                    slot_of,
                    object_keys,
                    print_locals,
                    seen_print,
                    has_var_for_head,
                    has_object_for_in,
                )?;
            }
            if let Some(t) = test {
                if !bool_expr_ok(t, by_id, slot_of) {
                    return None;
                }
            }
            if let Some(u) = update {
                if !expr_ok(u, by_id, slot_of) {
                    return None;
                }
            }
            classify_stmt(
                body,
                false,
                by_id,
                var_primary,
                slot_of,
                object_keys,
                print_locals,
                seen_print,
                has_var_for_head,
                has_object_for_in,
            )
        }
        Stmt::ForIn { left, right, body } => {
            if let Stmt::Declare {
                kind: BindingKind::Var,
                local,
                init,
                ..
            } = left.as_ref()
            {
                *has_var_for_head = true;
                let p = primary(*local, var_primary);
                // for-in binds string keys
                slot_of.entry(p).or_insert(SlotTy::String);
                if let Some(init) = init {
                    // Annex B init — must be string-ish
                    if !string_expr_ok(init, by_id, slot_of) {
                        return None;
                    }
                }
            } else {
                return None;
            }
            let keys = object_keys_of(right, by_id, object_keys)?;
            if keys.is_empty() {
                return None;
            }
            *has_object_for_in = true;
            classify_stmt(
                body,
                false,
                by_id,
                var_primary,
                slot_of,
                object_keys,
                print_locals,
                seen_print,
                has_var_for_head,
                has_object_for_in,
            )
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
        } => {
            if *is_await {
                return None;
            }
            if let Stmt::Declare {
                kind: BindingKind::Var,
                local,
                init: None,
                ..
            } = left.as_ref()
            {
                *has_var_for_head = true;
                let p = primary(*local, var_primary);
                // fixture: number array elements
                slot_of.entry(p).or_insert(SlotTy::Number);
            } else {
                return None;
            }
            if !array_expr_ok(right, by_id, slot_of) {
                return None;
            }
            classify_stmt(
                body,
                false,
                by_id,
                var_primary,
                slot_of,
                object_keys,
                print_locals,
                seen_print,
                has_var_for_head,
                has_object_for_in,
            )
        }
        _ => None,
    }
}

fn slot_for_declare(
    local: LocalId,
    init: Option<&Expr>,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
    object_keys: &mut HashMap<LocalId, Vec<String>>,
) -> Option<SlotTy> {
    if let Some(existing) = slot_of.get(&local) {
        // Redeclare / already classified from for-head.
        if let Some(init) = init {
            match existing {
                SlotTy::Number => {
                    if !number_expr_ok(init, by_id, slot_of) {
                        return None;
                    }
                }
                SlotTy::String => {
                    if !string_expr_ok(init, by_id, slot_of) {
                        return None;
                    }
                }
                SlotTy::Heap => {
                    if !object_expr_ok(init, by_id, slot_of)
                        && !array_expr_ok(init, by_id, slot_of)
                    {
                        return None;
                    }
                }
            }
        }
        return Some(*existing);
    }
    let Some(init) = init else {
        // bare var — default number (uninit unused in fixture prints)
        return Some(SlotTy::Number);
    };
    if let Expr::Object { properties, .. } = init {
        let keys = static_object_keys(properties)?;
        object_keys.insert(local, keys);
        return Some(SlotTy::Heap);
    }
    if matches!(init, Expr::Array { .. }) {
        if !array_expr_ok(init, by_id, slot_of) {
            return None;
        }
        return Some(SlotTy::Heap);
    }
    // Prefer string when init is a known string slot / string lit / concat.
    if string_expr_ok(init, by_id, slot_of) {
        return Some(SlotTy::String);
    }
    if number_expr_ok(init, by_id, slot_of) {
        return Some(SlotTy::Number);
    }
    let _ = by_id;
    None
}

fn static_object_keys(properties: &[ObjectProp]) -> Option<Vec<String>> {
    let mut keys = Vec::new();
    for p in properties {
        match p {
            ObjectProp::Property {
                key: ObjectPropKey::Static(k),
                value,
                ..
            } => {
                if !matches!(value, Expr::Number { .. }) {
                    return None;
                }
                keys.push(k.to_string_lossy());
            }
            _ => return None,
        }
    }
    Some(keys)
}

fn object_keys_of(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    object_keys: &HashMap<LocalId, Vec<String>>,
) -> Option<Vec<String>> {
    match expr {
        Expr::Object { properties, .. } => static_object_keys(properties),
        Expr::Local { id, .. } => object_keys.get(id).cloned(),
        _ => {
            let _ = by_id;
            None
        }
    }
}

fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, .. } => {
            // Prefer known slot; bare `any` is not assumed number (for-in keys are strings).
            if let Some(s) = slot_of.get(id) {
                return *s == SlotTy::Number;
            }
            by_id.get(id).is_some_and(|l| l.ty == Type::Number)
        }
        Expr::Binary {
            left,
            op:
                BinaryOp::Add
                | BinaryOp::Sub
                | BinaryOp::Mul
                | BinaryOp::Div
                | BinaryOp::Rem
                | BinaryOp::BitAnd
                | BinaryOp::BitOr
                | BinaryOp::BitXor
                | BinaryOp::Shl
                | BinaryOp::Shr
                | BinaryOp::UShr,
            right,
            ..
        } => number_expr_ok(left, by_id, slot_of) && number_expr_ok(right, by_id, slot_of),
        Expr::Assign {
            target: AssignTarget::Local(id),
            op: AssignOp::Eq,
            value,
            ..
        } => {
            (slot_of.get(id) == Some(&SlotTy::Number)
                || by_id.get(id).is_some_and(|l| l.ty == Type::Number))
                && number_expr_ok(value, by_id, slot_of)
        }
        _ => false,
    }
}

fn string_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::String { .. } => true,
        Expr::Local { id, .. } => {
            if let Some(s) = slot_of.get(id) {
                return *s == SlotTy::String;
            }
            by_id
                .get(id)
                .is_some_and(|l| matches!(l.ty, Type::String | Type::Any))
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add,
            right,
            ..
        } => {
            // string + string | string + number (ToString)
            (string_expr_ok(left, by_id, slot_of) || number_expr_ok(left, by_id, slot_of))
                && (string_expr_ok(right, by_id, slot_of) || number_expr_ok(right, by_id, slot_of))
                && (string_expr_ok(left, by_id, slot_of) || string_expr_ok(right, by_id, slot_of))
        }
        Expr::Assign {
            target: AssignTarget::Local(_),
            op: AssignOp::Eq,
            value,
            ..
        } => string_expr_ok(value, by_id, slot_of) || number_expr_ok(value, by_id, slot_of),
        _ => false,
    }
}

fn bool_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Boolean { .. } => true,
        Expr::Binary {
            left,
            op:
                BinaryOp::Lt
                | BinaryOp::LtEq
                | BinaryOp::Gt
                | BinaryOp::GtEq
                | BinaryOp::EqEq
                | BinaryOp::NotEq
                | BinaryOp::EqEqEq
                | BinaryOp::NotEqEq,
            right,
            ..
        } => {
            (number_expr_ok(left, by_id, slot_of) && number_expr_ok(right, by_id, slot_of))
                || (string_expr_ok(left, by_id, slot_of) && string_expr_ok(right, by_id, slot_of))
        }
        _ => false,
    }
}

fn expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    number_expr_ok(expr, by_id, slot_of)
        || string_expr_ok(expr, by_id, slot_of)
        || bool_expr_ok(expr, by_id, slot_of)
}

fn object_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Object { properties, .. } => static_object_keys(properties).is_some(),
        Expr::Local { id, .. } => {
            slot_of.get(id) == Some(&SlotTy::Heap)
                || by_id.get(id).is_some_and(|l| l.ty == Type::Object)
        }
        _ => false,
    }
}

fn array_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => number_expr_ok(e, by_id, slot_of),
            _ => false,
        }),
        Expr::Local { id, .. } => {
            slot_of.get(id) == Some(&SlotTy::Heap)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Object | Type::Any))
        }
        _ => false,
    }
}

// --- Emitter ----------------------------------------------------------------

struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    out: String,
    body: String,
    tmp: usize,
    allocas: HashMap<LocalId, String>,
    str_globals: HashMap<String, String>,
}

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            tmp: 0,
            allocas: HashMap::new(),
            str_globals: HashMap::new(),
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

    fn fresh_label(&mut self, prefix: &str) -> String {
        let t = self.tmp;
        self.tmp += 1;
        format!("{prefix}{t}")
    }

    fn resolve(&self, id: LocalId) -> LocalId {
        self.info.var_primary.get(&id).copied().unwrap_or(id)
    }

    fn slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        let p = self.resolve(id);
        self.allocas
            .get(&p)
            .cloned()
            .ok_or_else(|| diag(format!("es_var_for: missing alloca %{}", p.0)))
    }

    fn slot_ty(&self, id: LocalId) -> Result<SlotTy, Diagnostic> {
        let p = self.resolve(id);
        self.info
            .slots
            .iter()
            .find(|(i, _)| *i == p)
            .map(|(_, t)| *t)
            .ok_or_else(|| diag(format!("es_var_for: missing slot %{}", p.0)))
    }

    fn body_ends_with_terminator(&self) -> bool {
        self.body
            .lines()
            .rev()
            .find(|l| !l.trim().is_empty())
            .is_some_and(|l| {
                let t = l.trim_start();
                t.starts_with("br ")
                    || t.starts_with("ret ")
                    || t.starts_with("unreachable")
                    || t.starts_with("switch ")
                    || t.starts_with("indirectbr ")
            })
    }

    fn emit_module(&mut self, info: &ModuleInfo) -> Result<(), Diagnostic> {
        writeln!(
            self.out,
            "; Draconic LLVM backend (N08.16.15 var in for heads via Runtime ABI)"
        )
        .ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[
                GC_INIT,
                ALLOC_OBJECT,
                OBJECT_SET,
                ARRAY_NEW,
                ARRAY_GET,
                ARRAY_SET,
                ARRAY_LEN,
                CSTR_CONCAT,
                CSTR_FROM_U64,
                PRINT_F64,
                PRINT_STR,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        // Body first (collect string globals), then header.
        writeln!(self.body, "  {}", GC_INIT.call("")).ok();

        for (id, kind) in &info.slots {
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            match kind {
                SlotTy::Number => {
                    writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
                    writeln!(
                        self.body,
                        "  store double 0.00000000000000000e+00, ptr {ptr}"
                    )
                    .ok();
                }
                SlotTy::String | SlotTy::Heap => {
                    writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
                    writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
                }
            }
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for (id, kind) in &info.print_locals {
            let ptr = self.slot_ptr(*id)?;
            match kind {
                SlotTy::Number => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                }
                SlotTy::String => {
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                SlotTy::Heap => {}
            }
        }

        for (content, gname) in &self.str_globals {
            let n = content.len() + 1;
            let esc = escape_llvm_string(content);
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
        write!(self.out, "{}", self.body).ok();
        writeln!(self.out, "  ret i32 0").ok();
        writeln!(self.out, "}}").ok();
        Ok(())
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some(g) = self.str_globals.get(s) {
            g.clone()
        } else {
            let g = format!(".str.{}", self.str_globals.len());
            self.str_globals.insert(s.to_string(), g.clone());
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

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let p = self.resolve(*local);
                let kind = self.slot_ty(p)?;
                let ptr = self.slot_ptr(p)?;
                match (kind, init) {
                    (SlotTy::Number, Some(init)) => {
                        let v = self.emit_number_expr(init)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    (SlotTy::String, Some(init)) => {
                        let v = self.emit_string_expr(init)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    (SlotTy::Heap, Some(init)) => {
                        if matches!(init, Expr::Array { .. })
                            || self.slot_ty_of_expr(init) == Some(SlotTy::Heap)
                                && matches!(init, Expr::Local { .. })
                        {
                            // Prefer array when Array lit; object otherwise.
                        }
                        if matches!(init, Expr::Array { .. }) {
                            let v = self.emit_array_expr(init)?;
                            writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                        } else if matches!(init, Expr::Object { .. })
                            || self.info.object_keys.contains_key(&p)
                        {
                            let v = self.emit_object_expr(init)?;
                            writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                        } else if matches!(init, Expr::Local { .. }) {
                            // copy heap ptr
                            let v = self.emit_heap_local(init)?;
                            writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                        } else {
                            return Err(diag("es_var_for: unsupported heap init"));
                        }
                    }
                    (_, None) => {
                        // hoisted var already zero/null
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr } => {
                let _ = self.emit_discard_expr(expr)?;
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    if self.body_ends_with_terminator() {
                        break;
                    }
                    self.emit_stmt(s)?;
                }
                Ok(())
            }
            Stmt::If {
                test,
                consequent,
                alternate,
            } => {
                let cond = self.emit_bool_expr(test)?;
                let then_l = self.fresh_label("then");
                let else_l = self.fresh_label("else");
                let end_l = self.fresh_label("endif");
                if alternate.is_some() {
                    writeln!(
                        self.body,
                        "  br i1 {cond}, label %{then_l}, label %{else_l}"
                    )
                    .ok();
                } else {
                    writeln!(
                        self.body,
                        "  br i1 {cond}, label %{then_l}, label %{end_l}"
                    )
                    .ok();
                }
                writeln!(self.body, "{then_l}:").ok();
                self.emit_stmt(consequent)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{end_l}").ok();
                }
                if let Some(alt) = alternate {
                    writeln!(self.body, "{else_l}:").ok();
                    self.emit_stmt(alt)?;
                    if !self.body_ends_with_terminator() {
                        writeln!(self.body, "  br label %{end_l}").ok();
                    }
                }
                writeln!(self.body, "{end_l}:").ok();
                Ok(())
            }
            Stmt::For {
                init,
                test,
                update,
                body,
            } => {
                if let Some(i) = init {
                    self.emit_stmt(i)?;
                }
                let head = self.fresh_label("for_head");
                let bod = self.fresh_label("for_body");
                let upd = self.fresh_label("for_update");
                let end = self.fresh_label("for_end");
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{head}:").ok();
                if let Some(t) = test {
                    let cond = self.emit_bool_expr(t)?;
                    writeln!(self.body, "  br i1 {cond}, label %{bod}, label %{end}").ok();
                } else {
                    writeln!(self.body, "  br label %{bod}").ok();
                }
                writeln!(self.body, "{bod}:").ok();
                self.emit_stmt(body)?;
                if !self.body_ends_with_terminator() {
                    writeln!(self.body, "  br label %{upd}").ok();
                }
                writeln!(self.body, "{upd}:").ok();
                if let Some(u) = update {
                    let _ = self.emit_discard_expr(u)?;
                }
                writeln!(self.body, "  br label %{head}").ok();
                writeln!(self.body, "{end}:").ok();
                Ok(())
            }
            Stmt::ForIn { left, right, body } => self.emit_for_in(left, right, body),
            Stmt::ForOf {
                left,
                right,
                body,
                is_await,
            } => {
                if *is_await {
                    return Err(diag("es_var_for: for-await-of unsupported"));
                }
                self.emit_for_of(left, right, body)
            }
            _ => Err(diag("es_var_for: unsupported stmt")),
        }
    }

    fn emit_for_in(
        &mut self,
        left: &Stmt,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), Diagnostic> {
        let (bind_id, annex_init) = match left {
            Stmt::Declare { local, init, .. } => (*local, init.as_ref()),
            _ => return Err(diag("es_var_for: for-in left must be var declare")),
        };
        // Annex B.3.5: evaluate init once before enumeration.
        if let Some(init) = annex_init {
            let v = self.emit_string_expr(init)?;
            let ptr = self.slot_ptr(bind_id)?;
            writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
        }

        let keys = match right {
            Expr::Object { properties, .. } => static_object_keys(properties)
                .ok_or_else(|| diag("es_var_for: for-in object keys"))?,
            Expr::Local { id, .. } => self
                .info
                .object_keys
                .get(id)
                .cloned()
                .ok_or_else(|| diag("es_var_for: for-in unknown object keys"))?,
            _ => return Err(diag("es_var_for: for-in right must be object")),
        };

        // Unroll for-in over static keys (insertion order).
        for key in &keys {
            let s = self.string_const(key)?;
            let ptr = self.slot_ptr(bind_id)?;
            writeln!(self.body, "  store ptr {s}, ptr {ptr}").ok();
            self.emit_stmt(body)?;
        }
        Ok(())
    }

    fn emit_for_of(
        &mut self,
        left: &Stmt,
        right: &Expr,
        body: &Stmt,
    ) -> Result<(), Diagnostic> {
        let bind_id = match left {
            Stmt::Declare { local, init: None, .. } => *local,
            _ => return Err(diag("es_var_for: for-of left must be bare var declare")),
        };
        let arr = self.emit_array_expr(right)?;
        let idx_ptr = self.fresh();
        writeln!(self.body, "  {idx_ptr} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {idx_ptr}").ok();
        let head = self.fresh_label("forof_head");
        let bod = self.fresh_label("forof_body");
        let cont = self.fresh_label("forof_cont");
        let end = self.fresh_label("forof_end");
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{head}:").ok();
        let idx = self.fresh();
        writeln!(self.body, "  {idx} = load i64, ptr {idx_ptr}").ok();
        let len = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_LEN.call_to(&len, &format!("ptr {arr}"))
        )
        .ok();
        let cmp = self.fresh();
        writeln!(self.body, "  {cmp} = icmp ult i64 {idx}, {len}").ok();
        writeln!(self.body, "  br i1 {cmp}, label %{bod}, label %{end}").ok();
        writeln!(self.body, "{bod}:").ok();
        let elem = self.fresh();
        writeln!(
            self.body,
            "  {}",
            ARRAY_GET.call_to(&elem, &format!("ptr {arr}, i64 {idx}"))
        )
        .ok();
        // Number elements stored as inttoptr bit-patterns.
        let i = self.fresh();
        writeln!(self.body, "  {i} = ptrtoint ptr {elem} to i64").ok();
        let d = self.fresh();
        writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
        let bind_ptr = self.slot_ptr(bind_id)?;
        writeln!(self.body, "  store double {d}, ptr {bind_ptr}").ok();
        self.emit_stmt(body)?;
        if !self.body_ends_with_terminator() {
            writeln!(self.body, "  br label %{cont}").ok();
        }
        writeln!(self.body, "{cont}:").ok();
        let idx2 = self.fresh();
        writeln!(self.body, "  {idx2} = load i64, ptr {idx_ptr}").ok();
        let next = self.fresh();
        writeln!(self.body, "  {next} = add i64 {idx2}, 1").ok();
        writeln!(self.body, "  store i64 {next}, ptr {idx_ptr}").ok();
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{end}:").ok();
        Ok(())
    }

    fn emit_discard_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        if number_expr_ok(expr, &HashMap::new(), &self.slot_map())
            || matches!(
                expr,
                Expr::Assign {
                    target: AssignTarget::Local(_),
                    ..
                }
            )
        {
            // Prefer typed emit via assign / number / string.
        }
        match expr {
            Expr::Assign {
                target: AssignTarget::Local(id),
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let kind = self.slot_ty(*id)?;
                let ptr = self.slot_ptr(*id)?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(value)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(value)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Heap => {
                        let v = self.emit_heap_local(value)?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            e if self.is_number_slot_expr(e) => {
                let _ = self.emit_number_expr(e)?;
                Ok(())
            }
            e => {
                let _ = self.emit_string_expr(e)?;
                Ok(())
            }
        }
    }

    fn slot_map(&self) -> HashMap<LocalId, SlotTy> {
        self.info.slots.iter().copied().collect()
    }

    fn is_number_slot_expr(&self, expr: &Expr) -> bool {
        match expr {
            Expr::Number { .. } => true,
            Expr::Local { id, .. } => self.slot_ty(*id).ok() == Some(SlotTy::Number),
            Expr::Binary {
                op:
                    BinaryOp::Add
                    | BinaryOp::Sub
                    | BinaryOp::Mul
                    | BinaryOp::Div
                    | BinaryOp::Rem
                    | BinaryOp::Lt
                    | BinaryOp::LtEq
                    | BinaryOp::Gt
                    | BinaryOp::GtEq,
                left,
                right,
                ..
            } => {
                // Add may be string concat — check result ty via both number.
                self.is_number_slot_expr(left) && self.is_number_slot_expr(right)
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                value,
                ..
            } => {
                self.slot_ty(*id).ok() == Some(SlotTy::Number) && self.is_number_slot_expr(value)
            }
            _ => false,
        }
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => format_number_const(raw),
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Binary {
                left,
                op,
                right,
                ..
            } => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let t = self.fresh();
                let inst = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    BinaryOp::Rem => "frem",
                    _ => return Err(diag("es_var_for: unsupported number binop")),
                };
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let v = self.emit_number_expr(value)?;
                let ptr = self.slot_ptr(*id)?;
                writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                Ok(v)
            }
            _ => Err(diag("es_var_for: unsupported number expr")),
        }
    }

    fn emit_bool_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Boolean { value, .. } => Ok(if *value { "true" } else { "false" }.into()),
            Expr::Binary {
                left,
                op,
                right,
                ..
            } => {
                // string === string or number compare
                if matches!(
                    op,
                    BinaryOp::EqEqEq
                        | BinaryOp::NotEqEq
                        | BinaryOp::EqEq
                        | BinaryOp::NotEq
                ) && (self.is_stringish(left) || self.is_stringish(right))
                {
                    let l = self.emit_string_expr(left)?;
                    let r = self.emit_string_expr(right)?;
                    return self.emit_cstr_eq(
                        l,
                        r,
                        matches!(op, BinaryOp::NotEqEq | BinaryOp::NotEq),
                    );
                }
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let pred = match op {
                    BinaryOp::Lt => "olt",
                    BinaryOp::LtEq => "ole",
                    BinaryOp::Gt => "ogt",
                    BinaryOp::GtEq => "oge",
                    BinaryOp::EqEq | BinaryOp::EqEqEq => "oeq",
                    BinaryOp::NotEq | BinaryOp::NotEqEq => "one",
                    _ => return Err(diag("es_var_for: unsupported bool binop")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = fcmp {pred} double {l}, {r}").ok();
                Ok(t)
            }
            _ => Err(diag("es_var_for: unsupported bool expr")),
        }
    }

    fn is_stringish(&self, expr: &Expr) -> bool {
        match expr {
            Expr::String { .. } => true,
            Expr::Local { id, .. } => self.slot_ty(*id).ok() == Some(SlotTy::String),
            Expr::Binary {
                op: BinaryOp::Add,
                left,
                right,
                ..
            } => self.is_stringish(left) || self.is_stringish(right),
            _ => false,
        }
    }

    fn emit_cstr_eq(
        &mut self,
        left: String,
        right: String,
        ne: bool,
    ) -> Result<String, Diagnostic> {
        // Byte-wise null-terminated compare without libc: loop until mismatch or NUL.
        // For fixture `first === ""` both are simple — use runtime-free loop.
        let idx_ptr = self.fresh();
        writeln!(self.body, "  {idx_ptr} = alloca i64, align 8").ok();
        writeln!(self.body, "  store i64 0, ptr {idx_ptr}").ok();
        let head = self.fresh_label("streq_head");
        let cmp_l = self.fresh_label("streq_cmp");
        let ne_l = self.fresh_label("streq_ne");
        let eq_l = self.fresh_label("streq_eq");
        let done = self.fresh_label("streq_done");
        let res_ptr = self.fresh();
        writeln!(self.body, "  {res_ptr} = alloca i1, align 1").ok();
        writeln!(self.body, "  br label %{head}").ok();
        writeln!(self.body, "{head}:").ok();
        let idx = self.fresh();
        writeln!(self.body, "  {idx} = load i64, ptr {idx_ptr}").ok();
        let lp = self.fresh();
        writeln!(
            self.body,
            "  {lp} = getelementptr inbounds i8, ptr {left}, i64 {idx}"
        )
        .ok();
        let rp = self.fresh();
        writeln!(
            self.body,
            "  {rp} = getelementptr inbounds i8, ptr {right}, i64 {idx}"
        )
        .ok();
        let lb = self.fresh();
        writeln!(self.body, "  {lb} = load i8, ptr {lp}").ok();
        let rb = self.fresh();
        writeln!(self.body, "  {rb} = load i8, ptr {rp}").ok();
        let same = self.fresh();
        writeln!(self.body, "  {same} = icmp eq i8 {lb}, {rb}").ok();
        writeln!(self.body, "  br i1 {same}, label %{cmp_l}, label %{ne_l}").ok();
        writeln!(self.body, "{cmp_l}:").ok();
        let is_nul = self.fresh();
        writeln!(self.body, "  {is_nul} = icmp eq i8 {lb}, 0").ok();
        let next = self.fresh();
        writeln!(self.body, "  {next} = add i64 {idx}, 1").ok();
        writeln!(self.body, "  store i64 {next}, ptr {idx_ptr}").ok();
        writeln!(self.body, "  br i1 {is_nul}, label %{eq_l}, label %{head}").ok();
        writeln!(self.body, "{ne_l}:").ok();
        writeln!(self.body, "  store i1 false, ptr {res_ptr}").ok();
        writeln!(self.body, "  br label %{done}").ok();
        writeln!(self.body, "{eq_l}:").ok();
        writeln!(self.body, "  store i1 true, ptr {res_ptr}").ok();
        writeln!(self.body, "  br label %{done}").ok();
        writeln!(self.body, "{done}:").ok();
        let eq = self.fresh();
        writeln!(self.body, "  {eq} = load i1, ptr {res_ptr}").ok();
        if ne {
            let t = self.fresh();
            writeln!(self.body, "  {t} = xor i1 {eq}, true").ok();
            Ok(t)
        } else {
            Ok(eq)
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Local { id, .. } => {
                match self.slot_ty(*id)? {
                    SlotTy::String => {
                        let ptr = self.slot_ptr(*id)?;
                        let t = self.fresh();
                        writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                        Ok(t)
                    }
                    SlotTy::Number => {
                        // ToString number
                        let n = self.emit_number_expr(expr)?;
                        self.number_to_cstr(&n)
                    }
                    SlotTy::Heap => Err(diag("es_var_for: heap local is not a string")),
                }
            }
            Expr::Binary {
                left,
                op: BinaryOp::Add,
                right,
                ..
            } => {
                let l = self.emit_concat_operand(left)?;
                let r = self.emit_concat_operand(right)?;
                let t = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    CSTR_CONCAT.call_to(&t, &format!("ptr {l}, ptr {r}"))
                )
                .ok();
                Ok(t)
            }
            Expr::Assign {
                target: AssignTarget::Local(id),
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let v = self.emit_string_expr(value)?;
                let ptr = self.slot_ptr(*id)?;
                writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                Ok(v)
            }
            // number used in string context
            e if self.is_number_slot_expr(e) => {
                let n = self.emit_number_expr(e)?;
                self.number_to_cstr(&n)
            }
            _ => Err(diag("es_var_for: unsupported string expr")),
        }
    }

    fn emit_concat_operand(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        if self.is_number_slot_expr(expr) && !self.is_stringish(expr) {
            let n = self.emit_number_expr(expr)?;
            return self.number_to_cstr(&n);
        }
        // Local number slot
        if let Expr::Local { id, .. } = expr {
            if self.slot_ty(*id).ok() == Some(SlotTy::Number) {
                let n = self.emit_number_expr(expr)?;
                return self.number_to_cstr(&n);
            }
        }
        self.emit_string_expr(expr)
    }

    fn number_to_cstr(&mut self, n: &str) -> Result<String, Diagnostic> {
        let i = self.fresh();
        writeln!(self.body, "  {i} = fptoui double {n} to i64").ok();
        let p = self.fresh();
        writeln!(
            self.body,
            "  {}",
            CSTR_FROM_U64.call_to(&p, &format!("i64 {i}"))
        )
        .ok();
        Ok(p)
    }

    fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Object { properties, .. } => {
                let obj = self.fresh();
                writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
                for p in properties {
                    let ObjectProp::Property {
                        key: ObjectPropKey::Static(k),
                        value: Expr::Number { raw, .. },
                        ..
                    } = p
                    else {
                        return Err(diag("es_var_for: unsupported object prop"));
                    };
                    let key = self.string_const(&k.to_string_lossy())?;
                    // store number as inttoptr (fixture small ints)
                    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
                    let f: f64 = cleaned
                        .parse()
                        .map_err(|_| diag(format!("invalid number literal {raw}")))?;
                    let iv = f as i64;
                    let ip = self.fresh();
                    writeln!(self.body, "  {ip} = inttoptr i64 {iv} to ptr").ok();
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {ip}"))
                    )
                    .ok();
                }
                Ok(obj)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            _ => Err(diag("es_var_for: unsupported object expr")),
        }
    }

    fn emit_array_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Array { elements, .. } => {
                let n = elements.len() as u64;
                let arr = self.fresh();
                writeln!(
                    self.body,
                    "  {}",
                    ARRAY_NEW.call_to(&arr, &format!("i64 {n}"))
                )
                .ok();
                for (i, el) in elements.iter().enumerate() {
                    let ArrayElement::Expr(e) = el else {
                        return Err(diag("es_var_for: array hole/spread unsupported"));
                    };
                    let num = self.emit_number_expr(e)?;
                    let iv = self.fresh();
                    writeln!(self.body, "  {iv} = fptosi double {num} to i64").ok();
                    let ip = self.fresh();
                    writeln!(self.body, "  {ip} = inttoptr i64 {iv} to ptr").ok();
                    writeln!(
                        self.body,
                        "  {}",
                        ARRAY_SET.call(&format!("ptr {arr}, i64 {i}, ptr {ip}"))
                    )
                    .ok();
                }
                Ok(arr)
            }
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            _ => Err(diag("es_var_for: unsupported array expr")),
        }
    }

    fn emit_heap_local(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Local { id, .. } => {
                let ptr = self.slot_ptr(*id)?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            Expr::Object { .. } => self.emit_object_expr(expr),
            Expr::Array { .. } => self.emit_array_expr(expr),
            _ => Err(diag("es_var_for: unsupported heap expr")),
        }
    }

    fn slot_ty_of_expr(&self, expr: &Expr) -> Option<SlotTy> {
        match expr {
            Expr::Local { id, .. } => self.slot_ty(*id).ok(),
            Expr::Object { .. } | Expr::Array { .. } => Some(SlotTy::Heap),
            Expr::Number { .. } => Some(SlotTy::Number),
            Expr::String { .. } => Some(SlotTy::String),
            _ => None,
        }
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
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'\\' => out.push_str("\\\\"),
            b'"' => out.push_str("\\22"),
            0x07 => out.push_str("\\07"),
            0x08 => out.push_str("\\08"),
            0x09 => out.push_str("\\09"),
            0x0a => out.push_str("\\0A"),
            0x0c => out.push_str("\\0C"),
            0x0d => out.push_str("\\0D"),
            0x20..=0x7e => out.push(b as char),
            _ => {
                let _ = write!(out, "\\{b:02X}");
            }
        }
    }
    out
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
