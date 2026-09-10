//! N08.06.01–N08.06.06: native observations for ES array literals, index
//! access, `.length`, element assignment, spread in array literals,
//! `for-of` over arrays, and array destructuring (`es/arrays/array_lit_access`,
//! `array_element_assign`, `array_spread`, `array_for_of`, `array_destructure`).
//!
//! Arrays are Runtime GC heap values (`draconic_rt_array_*`). Number elements
//! are stored as `inttoptr` of integer bit-patterns; nested arrays store GC
//! ptrs; strings are cstr ptrs; booleans are `inttoptr` 0/1; `null`/`undefined`
//! are null. Empty objects use Runtime `alloc_object` for member destructure
//! targets. Number locals print via `print_f64`; string index/accumulator
//! results via `print_str`. `for-of` walks length + index get with break/continue.
//! Destructuring binds via index get + rest copy; defaults fire on null/hole.

use std::collections::HashMap;

use draconic_ast::{AssignOp, BinaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    ArrayElement, ArrayPatternEl, AssignTarget, Expr, IrType as Type, Local, LocalId, Module,
    Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, ARRAY_GET, ARRAY_LEN, ARRAY_NEW, ARRAY_SET, ARRAY_SPREAD_ARRAY,
    ARRAY_SPREAD_CSTR, CSTR_CONCAT, GC_INIT, OBJECT_GET, OBJECT_SET, PRINT_F64, PRINT_STR,
};

mod classify;
mod emit;
mod emit_expr;
mod ok;

use classify::classify;

pub(crate) fn is_es_arrays_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_arrays(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_arrays module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
    Ok(em.finish())
}

pub(crate) fn walk_es_arrays(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_es_arrays_module(module) {
        return None;
    }
    Some(emit_es_arrays(module))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LocalSlot {
    Number,
    Array,
    String,
    Bool,
    Null,
    Object,
}

/// Homogeneous element kind for spread/index type inference (N08.06.03).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ElemKind {
    Number,
    String,
    Array,
    Unknown,
}

struct ModuleInfo {
    slots: Vec<(LocalId, LocalSlot)>,
    /// Observation prints: numbers via `print_f64`, strings via `print_str`.
    print_locals: Vec<(LocalId, LocalSlot)>,
}

struct ClassifyCtx<'a> {
    by_id: &'a HashMap<LocalId, &'a Local>,
    slots: Vec<(LocalId, LocalSlot)>,
    print_locals: Vec<(LocalId, LocalSlot)>,
    has_array: bool,
    arr_inits: HashMap<LocalId, Expr>,
    arr_elem: HashMap<LocalId, ElemKind>,
    slot_of: HashMap<LocalId, LocalSlot>,
}

struct CtrlFrame {
    break_label: String,
    continue_label: Option<String>,
}

struct Emitter<'a> {
    module: &'a Module,
    out: String,
    body: String,
    allocas: HashMap<LocalId, String>,
    slot_of: HashMap<LocalId, LocalSlot>,
    str_globals: Vec<(String, String)>,
    tmp: usize,
    str_n: usize,
    ctrls: Vec<CtrlFrame>,
}

fn number_global_name(id: LocalId) -> String {
    format!("es_arr_n{}", id.0)
}

fn ptr_global_name(id: LocalId, kind: LocalSlot) -> String {
    let tag = match kind {
        LocalSlot::Array => "a",
        LocalSlot::String => "s",
        LocalSlot::Bool => "b",
        LocalSlot::Null => "z",
        LocalSlot::Object => "o",
        LocalSlot::Number => "n",
    };
    format!("es_arr_{tag}{}", id.0)
}

fn format_number_const(raw: &str) -> Result<String, Diagnostic> {
    let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
    let f: f64 = cleaned
        .parse()
        .map_err(|_| diag(format!("invalid number literal {raw}")))?;
    Ok(format!("{f:.17e}"))
}

fn diag(message: impl Into<String>) -> Diagnostic {
    Diagnostic::new(message, Span::dummy())
}
