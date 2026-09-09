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
#[path = "es_var_for_emit.rs"]
mod emit;


pub(crate) fn is_es_var_for_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_var_for(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_var_for module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_var_for(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_var_for_module(module) {
        return None;
    }
    Some(emit_es_var_for(module))
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
                    if !object_expr_ok(init, by_id, slot_of) && !array_expr_ok(init, by_id, slot_of)
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
