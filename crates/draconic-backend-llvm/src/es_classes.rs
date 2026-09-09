//! N08.05.01–N08.05.04 + N08.16.26 + N08.16.33 + N08.16.36: native observations for ES
//! class declarations (E05.01 / `class_basic`), class expressions (E18.33 /
//! `class_expr`), heritage (E05.02), static methods (E05.03), `super` property access
//! (E05.04), public fields (E18.26 / `class_fields`), and private instance fields
//! (E18.35 / `private_fields`).
//!
//! Classes lower to builder IIFEs (`const C = (function(){ … return ctor })()`).
//! This adapter recognizes that shape for base and derived classes (public fields via
//! `__fi` defineProperty rewrite; private fields via WeakMap desugar), extracts the
//! constructor + prototype methods + static methods/fields + optional `extends`
//! parent, and emits the Runtime GC/object ABI.
//! Number locals print via `print_f64`; typeof/undefined/string locals via `print_str`.

use std::collections::HashMap;

use draconic_ast::UnaryOp;
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, Expr, Local, LocalId, Module, ObjectProp, Stmt};

#[path = "es_classes_private.rs"]
mod private;
#[path = "es_classes_ctor.rs"]
mod ctor;
#[path = "es_classes_ok.rs"]
mod ok;
#[path = "es_classes_extract.rs"]
mod extract;
#[path = "es_classes_emit.rs"]
mod emit;
#[path = "es_classes_emit_expr.rs"]
mod emit_expr;

use extract::{try_extract_class, try_fold_new_class_iife_member};
use ok::{
    is_object_slot, number_expr_ok, object_expr_ok, side_effect_ok, typeof_string_expr_ok,
};

const MAX_METHOD_ARGS: usize = 4;
/// qNaN payload marking JS `undefined` (matches es_functions).
const UNDEF_BITS: u64 = 0x7FF8_0000_0000_0001;

fn undef_double_const() -> String {
    format!("bitcast (i64 {UNDEF_BITS} to double)")
}

pub(crate) fn walk_es_classes(module: &Module) -> Option<Result<String, Diagnostic>> {
    let info = classify(module)?;
    Some(emit_classified(module, &info))
}

pub(crate) fn walk_es_classes_applies(module: &Module) -> bool {
    classify(module).is_some()
}

fn emit_classified(module: &Module, info: &ModuleInfo) -> Result<String, Diagnostic> {
    let mut em = Emitter::new(module, info);
    em.emit_module(info)?;
    Ok(em.finish())
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum SlotTy {
    Number,
    Object,
    /// Top-level typeof observation (`"undefined"` / `"function"` / …).
    String,
    /// Missing/undefined field observation (print `undefined`).
    Undefined,
}

/// Public field initializer value (instance or static).
#[derive(Clone)]
enum FieldVal {
    Number(Expr),
    String(String),
    Undef,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MethodRet {
    Number,
    String,
}

#[derive(Clone)]
struct FnInfo {
    idx: usize,
    params: Vec<LocalId>,
    body: Vec<Stmt>,
    /// When set, this function is a derived constructor; `super(...)` calls this parent ctor.
    parent_ctor_fn_idx: Option<usize>,
    /// Parent class index for `super.m(…)` resolution in derived methods.
    super_class_idx: Option<usize>,
    ret: MethodRet,
}

#[derive(Clone)]
struct ClassInfo {
    ctor_fn_idx: usize,
    /// Prototype method name → function index.
    methods: Vec<(String, usize)>,
    /// Static method name → function index (own props on the constructor).
    static_methods: Vec<(String, usize)>,
    /// Public static fields (`static x = expr`).
    static_fields: Vec<(String, FieldVal)>,
    /// Public instance fields (`x = expr`).
    instance_fields: Vec<(String, FieldVal)>,
    /// Parent class index in `ModuleInfo::classes` when `extends` is present.
    parent: Option<usize>,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    /// Observation print order (numbers/strings/undefined interleaved by declare order).
    observe_locals: Vec<LocalId>,
    functions: Vec<FnInfo>,
    classes: Vec<ClassInfo>,
    /// Class binding → index in `classes`.
    class_of: HashMap<LocalId, usize>,
    /// Instance local → class index (`let p = new C()`).
    instance_of: HashMap<LocalId, usize>,
    /// Folded number observations (`new (class {…})().x` → constant).
    const_number: HashMap<LocalId, String>,
}


fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut class_of = HashMap::new();
    let mut instance_of = HashMap::new();
    let mut slots = Vec::new();
    let mut observe_locals = Vec::new();
    let mut const_number = HashMap::new();
    let mut saw_class = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Declare { init: None, .. } => {
                // Private compound-assign temps (`__drac_pobj_*` / `__drac_pval_*`).
            }
            Stmt::Declare {
                local,
                init: Some(init),
                ..
            } => {
                if let Some(cls) =
                    try_extract_class(init, *local, &by_id, &mut functions, &class_of, &classes)
                {
                    saw_class = true;
                    let idx = classes.len();
                    class_of.insert(*local, idx);
                    classes.push(cls);
                    slots.push((*local, SlotTy::Object));
                } else if let Some(ci) = new_class_idx(init, &class_of) {
                    if !object_expr_ok(init, &class_of, &by_id, &functions, &classes) {
                        return None;
                    }
                    instance_of.insert(*local, ci);
                    slots.push((*local, SlotTy::Object));
                } else if is_object_slot(init, &class_of, &by_id) {
                    if !object_expr_ok(init, &class_of, &by_id, &functions, &classes) {
                        return None;
                    }
                    slots.push((*local, SlotTy::Object));
                } else if let Some((st, raw)) = try_fold_new_class_iife_member(
                    init,
                    &by_id,
                    &mut functions,
                    &class_of,
                    &classes,
                ) {
                    // `let i = new (class { constructor(){ this.x = 42 } })().x`
                    saw_class = true;
                    slots.push((*local, st));
                    if let Some(raw) = raw {
                        const_number.insert(*local, raw);
                    }
                    if matches!(st, SlotTy::Number | SlotTy::String | SlotTy::Undefined) {
                        observe_locals.push(*local);
                    }
                } else if let Some(st) =
                    classify_value_init(init, &class_of, &instance_of, &classes, &by_id, &functions)
                {
                    slots.push((*local, st));
                    if matches!(st, SlotTy::Number | SlotTy::String | SlotTy::Undefined) {
                        observe_locals.push(*local);
                    }
                } else {
                    return None;
                }
            }
            Stmt::Expr { expr } => {
                if !side_effect_ok(expr, &class_of, &by_id, &functions, &classes) {
                    return None;
                }
            }
            _ => return None,
        }
    }

    if !saw_class || observe_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        slots,
        observe_locals,
        functions,
        classes,
        class_of,
        instance_of,
        const_number,
    })
}

fn new_class_idx(init: &Expr, class_of: &HashMap<LocalId, usize>) -> Option<usize> {
    let Expr::New { callee, .. } = init else {
        return None;
    };
    let Expr::Local { id, .. } = callee.as_ref() else {
        return None;
    };
    class_of.get(id).copied()
}

fn classify_value_init(
    init: &Expr,
    class_of: &HashMap<LocalId, usize>,
    instance_of: &HashMap<LocalId, usize>,
    classes: &[ClassInfo],
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
) -> Option<SlotTy> {
    if let Expr::Unary {
        op: UnaryOp::TypeOf,
        arg,
        ..
    } = init
    {
        if member_field_val(arg, class_of, instance_of, classes).is_some()
            || object_expr_ok(arg, class_of, by_id, functions, classes)
            || matches!(arg.as_ref(), Expr::Local { .. } | Expr::Member { .. })
        {
            return Some(SlotTy::String);
        }
        return None;
    }
    if let Some(fv) = member_field_val(init, class_of, instance_of, classes) {
        return Some(match fv {
            FieldVal::Number(_) => SlotTy::Number,
            FieldVal::String(_) => SlotTy::String,
            FieldVal::Undef => SlotTy::Undefined,
        });
    }
    if typeof_string_expr_ok(init, class_of, by_id, functions, classes) {
        return Some(SlotTy::String);
    }
    if number_expr_ok(init, class_of, by_id, functions, classes) {
        return Some(SlotTy::Number);
    }
    None
}

fn member_field_val<'a>(
    expr: &Expr,
    class_of: &HashMap<LocalId, usize>,
    instance_of: &HashMap<LocalId, usize>,
    classes: &'a [ClassInfo],
) -> Option<&'a FieldVal> {
    let Expr::Member {
        object,
        property,
        optional,
        ..
    } = expr
    else {
        return None;
    };
    if *optional {
        return None;
    }
    let key = match property.as_ref() {
        Expr::String { value, .. } => value.to_string_lossy(),
        _ => return None,
    };
    match object.as_ref() {
        Expr::Local { id, .. } => {
            if let Some(&ci) = class_of.get(id) {
                return lookup_static_field(classes, ci, &key);
            }
            if let Some(&ci) = instance_of.get(id) {
                return lookup_instance_field(classes, ci, &key);
            }
            None
        }
        Expr::New { callee, .. } => {
            let Expr::Local { id, .. } = callee.as_ref() else {
                return None;
            };
            let ci = *class_of.get(id)?;
            lookup_instance_field(classes, ci, &key)
        }
        _ => None,
    }
}

fn lookup_instance_field<'a>(
    classes: &'a [ClassInfo],
    mut idx: usize,
    name: &str,
) -> Option<&'a FieldVal> {
    loop {
        let cls = classes.get(idx)?;
        if let Some((_, v)) = cls.instance_fields.iter().find(|(n, _)| n == name) {
            return Some(v);
        }
        idx = cls.parent?;
    }
}

fn lookup_static_field<'a>(
    classes: &'a [ClassInfo],
    idx: usize,
    name: &str,
) -> Option<&'a FieldVal> {
    let cls = classes.get(idx)?;
    cls.static_fields
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v)
}


pub(super) fn is_object_set_prototype_of(callee: &Expr) -> bool {
    let Expr::Member {
        object, property, ..
    } = callee
    else {
        return false;
    };
    matches!(
        (object.as_ref(), property.as_ref()),
        (
            Expr::IdentName { name, .. },
            Expr::String { value, .. }
        ) if name == "Object" && value.to_string_lossy() == "setPrototypeOf"
    )
}

pub(super) fn is_object_define_property(callee: &Expr) -> bool {
    let Expr::Member {
        object, property, ..
    } = callee
    else {
        return false;
    };
    matches!(
        (object.as_ref(), property.as_ref()),
        (
            Expr::IdentName { name, .. },
            Expr::String { value, .. }
        ) if name == "Object" && value.to_string_lossy() == "defineProperty"
    )
}

pub(super) fn is_define_on_ctor(args: &[Arg], ctor: LocalId) -> bool {
    matches!(
        &args[0],
        Arg::Expr(Expr::Local { id, .. }) if *id == ctor
    )
}

pub(super) fn is_define_on_proto(args: &[Arg], ctor: LocalId) -> bool {
    let Arg::Expr(Expr::Member {
        object, property, ..
    }) = &args[0]
    else {
        return false;
    };
    matches!(
        (object.as_ref(), property.as_ref()),
        (
            Expr::Local { id, .. },
            Expr::String { value, .. }
        ) if *id == ctor && value.to_string_lossy() == "prototype"
    )
}

pub(super) fn string_arg(arg: &Arg) -> Option<String> {
    match arg {
        Arg::Expr(Expr::String { value, .. }) => Some(value.to_string_lossy()),
        Arg::Expr(Expr::Local { .. }) => None,
        _ => None,
    }
}

pub(super) fn find_method_function(expr: &Expr) -> Option<&Expr> {
    match expr {
        Expr::Function {
            is_method: true, ..
        } => Some(expr),
        Expr::Function { body, .. } => {
            for s in body {
                if let Some(f) = find_method_function_in_stmt(s) {
                    return Some(f);
                }
            }
            None
        }
        Expr::Call { callee, args, .. } => {
            if let Some(f) = find_method_function(callee) {
                return Some(f);
            }
            for a in args {
                if let Arg::Expr(e) = a {
                    if let Some(f) = find_method_function(e) {
                        return Some(f);
                    }
                }
            }
            None
        }
        Expr::Object { properties, .. } => {
            for p in properties {
                if let ObjectProp::Property { value, .. } = p {
                    if let Some(f) = find_method_function(value) {
                        return Some(f);
                    }
                }
            }
            None
        }
        Expr::Member {
            object, property, ..
        } => find_method_function(object).or_else(|| find_method_function(property)),
        Expr::Binary { left, right, .. } => {
            find_method_function(left).or_else(|| find_method_function(right))
        }
        Expr::Assign { value, .. } => find_method_function(value),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => find_method_function(test)
            .or_else(|| find_method_function(consequent))
            .or_else(|| find_method_function(alternate)),
        Expr::Unary { arg, .. } => find_method_function(arg),
        _ => None,
    }
}

fn find_method_function_in_stmt(stmt: &Stmt) -> Option<&Expr> {
    match stmt {
        Stmt::Expr { expr } | Stmt::Return { value: Some(expr) } => find_method_function(expr),
        Stmt::Declare {
            init: Some(expr), ..
        } => find_method_function(expr),
        Stmt::Block { body } => {
            for s in body {
                if let Some(f) = find_method_function_in_stmt(s) {
                    return Some(f);
                }
            }
            None
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => find_method_function(test)
            .or_else(|| find_method_function_in_stmt(consequent))
            .or_else(|| {
                alternate
                    .as_ref()
                    .and_then(|a| find_method_function_in_stmt(a))
            }),
        _ => None,
    }
}


struct Emitter<'a> {
    module: &'a Module,
    info: &'a ModuleInfo,
    out: String,
    body: String,
    allocas: HashMap<LocalId, String>,
    slot_of: HashMap<LocalId, SlotTy>,
    param_allocas: HashMap<LocalId, String>,
    this_ssa: Option<String>,
    active_parent_ctor: Option<usize>,
    /// Parent class index while emitting a derived method (`super.m` base).
    active_super_class: Option<usize>,
    active_method_ret: MethodRet,
    /// Class binding local id for each class index (for parent ctor object load).
    class_binding: HashMap<usize, LocalId>,
    str_globals: Vec<(String, String)>,
    tmp: usize,
    str_n: usize,
}

fn number_global_name(id: LocalId) -> String {
    format!("es_cls_n_{}", id.0)
}

fn string_global_name(id: LocalId) -> String {
    format!("es_cls_s_{}", id.0)
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

