//! N08.05.01–N08.05.04 + N08.16.26 + N08.16.36: native observations for ES class
//! declarations (E05.01 / `class_basic`), heritage (E05.02), static methods (E05.03),
//! `super` property access (E05.04), public fields (E18.26 / `class_fields`), and
//! private instance fields (E18.35 / `private_fields`).
//!
//! Classes lower to builder IIFEs (`const C = (function(){ … return ctor })()`).
//! This adapter recognizes that shape for base and derived classes (public fields via
//! `__fi` defineProperty rewrite; private fields via WeakMap desugar), extracts the
//! constructor + prototype methods + static methods/fields + optional `extends`
//! parent, and emits the Runtime GC/object ABI.
//! Number locals print via `print_f64`; typeof/undefined/string locals via `print_str`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, ObjectPropKey,
    Param, Pattern, Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, GC_INIT, OBJECT_GET, OBJECT_SET, OBJECT_SET_PROTO, PRINT_F64,
    PRINT_STR,
};

const MAX_METHOD_ARGS: usize = 4;
/// qNaN payload marking JS `undefined` (matches es_functions).
const UNDEF_BITS: u64 = 0x7FF8_0000_0000_0001;

fn undef_double_const() -> String {
    format!("bitcast (i64 {UNDEF_BITS} to double)")
}

pub(crate) fn is_es_classes_module(module: &Module) -> bool {
    classify(module).is_some()
}

pub(crate) fn emit_es_classes(module: &Module) -> Result<String, Diagnostic> {
    let info = classify(module).ok_or_else(|| diag("internal: not an es_classes module"))?;
    let mut em = Emitter::new(module, &info);
    em.emit_module(&info)?;
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
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut class_of = HashMap::new();
    let mut instance_of = HashMap::new();
    let mut slots = Vec::new();
    let mut observe_locals = Vec::new();
    let mut saw_class = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Declare { init: None, .. } => {
                // Private compound-assign temps (`__drac_pobj_*` / `__drac_pval_*`).
            }
            Stmt::Declare { local, init: Some(init), .. } => {
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
                } else if let Some(st) = classify_value_init(
                    init,
                    &class_of,
                    &instance_of,
                    &classes,
                    &by_id,
                    &functions,
                ) {
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

fn try_extract_class(
    init: &Expr,
    _binding: LocalId,
    by_id: &HashMap<LocalId, &Local>,
    functions: &mut Vec<FnInfo>,
    class_of: &HashMap<LocalId, usize>,
    classes: &[ClassInfo],
) -> Option<ClassInfo> {
    let Expr::Call {
        callee,
        args,
        optional,
        ..
    } = init
    else {
        return None;
    };
    if *optional || !args.is_empty() {
        return None;
    }
    let Expr::Function {
        params,
        body,
        is_async,
        is_generator,
        is_arrow,
        ..
    } = callee.as_ref()
    else {
        return None;
    };
    if *is_async || *is_generator || *is_arrow || !params.is_empty() {
        return None;
    }

    // Base: "use strict"; WeakMaps; const ctor = function…; defineProperty…; methods; return.
    // Derived: super binding + heritage checks + ctor with super() + setPrototypeOf.
    let mut ctor_local: Option<LocalId> = None;
    let mut ctor_fn_idx: Option<usize> = None;
    let mut methods: Vec<(String, usize)> = Vec::new();
    let mut static_methods: Vec<(String, usize)> = Vec::new();
    let mut static_fields: Vec<(String, FieldVal)> = Vec::new();
    let mut instance_fields: Vec<(String, FieldVal)> = Vec::new();
    let mut pending_key: Option<String> = None;
    let mut parent_idx: Option<usize> = None;
    let mut parent_ctor_fn_idx: Option<usize> = None;
    let mut wm_fields: HashMap<LocalId, String> = HashMap::new();

    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            // Private instance field storage: `let __drac_pf_N_name = new WeakMap()`
            Stmt::Declare {
                local,
                init:
                    Some(Expr::New {
                        callee,
                        args,
                        ..
                    }),
                ..
            } if args.is_empty() && is_ident_name(callee, "WeakMap") => {
                let lname = by_id.get(local)?.name.as_str();
                if let Some(fname) = private_field_name_from_wm(lname) {
                    wm_fields.insert(*local, fname);
                } else {
                    return None;
                }
            }
            // Derived: `let __drac_super_N = Parent`
            Stmt::Declare {
                init: Some(Expr::Local { id, .. }),
                ..
            } if ctor_local.is_none() && parent_idx.is_none() && !is_weakmap_local(*id, by_id) => {
                let pidx = *class_of.get(id)?;
                parent_idx = Some(pidx);
                parent_ctor_fn_idx = Some(classes[pidx].ctor_fn_idx);
            }
            // Derived: bare `let __drac_sproto_N` / private temps
            Stmt::Declare { init: None, .. } => {}
            // Derived: heritage IsConstructor / prototype checks
            Stmt::If { .. } if ctor_local.is_none() => {}
            Stmt::Declare {
                local,
                init: Some(Expr::Function {
                    params: cparams,
                    body: cbody,
                    is_async: ca,
                    is_generator: cg,
                    is_arrow: carrow,
                    ..
                }),
                ..
            } if ctor_local.is_none() => {
                if *ca || *cg || *carrow {
                    return None;
                }
                let param_ids = simple_param_ids(cparams, by_id)?;
                let (filtered, ifields) = if parent_idx.is_some() {
                    filter_derived_ctor_body(cbody)
                } else {
                    filter_ctor_body(cbody)
                };
                instance_fields = ifields;
                let rewritten = rewrite_private_stmts(&filtered, &wm_fields);
                if !method_body_ok(&rewritten, by_id) {
                    return None;
                }
                let idx = functions.len();
                functions.push(FnInfo {
                    idx,
                    params: param_ids,
                    body: rewritten,
                    parent_ctor_fn_idx,
                    super_class_idx: None,
                    ret: MethodRet::Number,
                });
                ctor_local = Some(*local);
                ctor_fn_idx = Some(idx);
            }
            Stmt::Declare {
                init: Some(Expr::String { value, .. }),
                ..
            } => {
                pending_key = Some(value.to_string_lossy());
            }
            Stmt::Expr {
                expr: Expr::Call {
                    callee: def_callee,
                    args: def_args,
                    ..
                },
            } if is_object_define_property(def_callee) && def_args.len() == 3 => {
                let ctor = ctor_local?;
                if is_define_on_ctor(def_args, ctor) {
                    let Arg::Expr(desc_expr) = &def_args[2] else {
                        continue;
                    };
                    let key = pending_key
                        .take()
                        .or_else(|| string_arg(&def_args[1]))
                        .unwrap_or_default();
                    if key.is_empty() || key == "name" || key == "prototype" {
                        continue;
                    }
                    // Static method: descriptor.value is directly a function (not field __fi.call).
                    if let Some(method_fn) = descriptor_direct_method_fn(desc_expr) {
                        let Expr::Function {
                            params: mparams,
                            body: mbody,
                            is_async: ma,
                            is_generator: mg,
                            ..
                        } = method_fn
                        else {
                            return None;
                        };
                        if *ma || *mg {
                            return None;
                        }
                        let param_ids = simple_param_ids(mparams, by_id)?;
                        let filtered = filter_method_body(mbody);
                        let rewritten = rewrite_private_stmts(&filtered, &wm_fields);
                        if !method_body_ok(&rewritten, by_id) {
                            return None;
                        }
                        let ret = method_ret_kind(&rewritten);
                        let idx = functions.len();
                        functions.push(FnInfo {
                            idx,
                            params: param_ids,
                            body: rewritten,
                            parent_ctor_fn_idx: None,
                            super_class_idx: parent_idx,
                            ret,
                        });
                        static_methods.push((key, idx));
                        continue;
                    }
                    // Public static field: defineProperty(ctor, key, { value: __fi.call(ctor), … })
                    if let Some(fv) = static_field_val_from_desc(desc_expr) {
                        static_fields.push((key, fv));
                        continue;
                    }
                    continue;
                }
                if !is_define_on_proto(def_args, ctor) {
                    return None;
                }
                let key = pending_key.take().or_else(|| string_arg(&def_args[1]))?;
                let Arg::Expr(desc_expr) = &def_args[2] else {
                    return None;
                };
                let method_fn = find_method_function(desc_expr)?;
                let Expr::Function {
                    params: mparams,
                    body: mbody,
                    is_async: ma,
                    is_generator: mg,
                    ..
                } = method_fn
                else {
                    return None;
                };
                if *ma || *mg {
                    return None;
                }
                let param_ids = simple_param_ids(mparams, by_id)?;
                let filtered = filter_method_body(mbody);
                let rewritten = rewrite_private_stmts(&filtered, &wm_fields);
                if !method_body_ok(&rewritten, by_id) {
                    return None;
                }
                let ret = method_ret_kind(&rewritten);
                let idx = functions.len();
                functions.push(FnInfo {
                    idx,
                    params: param_ids,
                    body: rewritten,
                    parent_ctor_fn_idx: None,
                    super_class_idx: parent_idx,
                    ret,
                });
                methods.push((key, idx));
            }
            // Derived: Object.setPrototypeOf(ctor.prototype, sproto) / setPrototypeOf(ctor, super)
            Stmt::Expr {
                expr: Expr::Call {
                    callee: sp_callee,
                    args: sp_args,
                    ..
                },
            } if is_object_set_prototype_of(sp_callee) && sp_args.len() == 2 => {}
            Stmt::Return {
                value: Some(Expr::Local { id, .. }),
            } if Some(*id) == ctor_local => {}
            _ => return None,
        }
    }

    Some(ClassInfo {
        ctor_fn_idx: ctor_fn_idx?,
        methods,
        static_methods,
        static_fields,
        instance_fields,
        parent: parent_idx,
    })
}



fn is_wm_method_call(
    callee: &Expr,
    args: &[Arg],
    method: &str,
    obj_param: LocalId,
    val_param: Option<LocalId>,
) -> bool {
    let Expr::Member {
        object,
        property,
        ..
    } = callee
    else {
        return false;
    };
    if !matches!(object.as_ref(), Expr::Local { .. }) {
        return false;
    }
    let Expr::String { value, .. } = property.as_ref() else {
        return false;
    };
    if value.to_string_lossy() != method {
        return false;
    }
    if args.is_empty() {
        return false;
    }
    if !matches!(&args[0], Arg::Expr(Expr::Local { id, .. }) if *id == obj_param) {
        return false;
    }
    if let Some(vp) = val_param {
        if args.len() < 2 {
            return false;
        }
        if !matches!(&args[1], Arg::Expr(Expr::Local { id, .. }) if *id == vp) {
            return false;
        }
    }
    true
}

fn wm_id_from_callee(callee: &Expr) -> Option<LocalId> {
    let Expr::Member { object, .. } = callee else {
        return None;
    };
    match object.as_ref() {
        Expr::Local { id, .. } => Some(*id),
        _ => None,
    }
}

fn is_ident_name(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::IdentName { name: n, .. } if n == name)
}

fn is_weakmap_local(id: LocalId, by_id: &HashMap<LocalId, &Local>) -> bool {
    by_id
        .get(&id)
        .is_some_and(|l| l.name.starts_with("__drac_pf_"))
}

fn private_field_name_from_wm(local_name: &str) -> Option<String> {
    // `__drac_pf_<id>_<field>`
    let rest = local_name.strip_prefix("__drac_pf_")?;
    let (_, field) = rest.split_once('_')?;
    if field.is_empty() {
        return None;
    }
    Some(format!("#{field}"))
}

fn private_key_string(field_key: &str) -> Expr {
    Expr::String {
        value: field_key.into(),
        ty: Type::String,
    }
}

fn rewrite_private_stmts(body: &[Stmt], wm_fields: &HashMap<LocalId, String>) -> Vec<Stmt> {
    body.iter()
        .filter_map(|s| rewrite_private_stmt(s, wm_fields))
        .collect()
}

fn rewrite_private_stmt(stmt: &Stmt, wm_fields: &HashMap<LocalId, String>) -> Option<Stmt> {
    match stmt {
        Stmt::Expr { expr } => {
            if let Some(e) = rewrite_private_expr(expr, wm_fields) {
                // Drop pure "use strict" strings
                if matches!(&e, Expr::String { value, .. } if value.to_string_lossy() == "use strict")
                {
                    return None;
                }
                Some(Stmt::Expr { expr: e })
            } else {
                None
            }
        }
        Stmt::Return { value: Some(e) } => Some(Stmt::Return {
            value: Some(rewrite_private_expr(e, wm_fields).unwrap_or_else(|| e.clone())),
        }),
        Stmt::Return { value: None } => Some(stmt.clone()),
        Stmt::Block { body } => {
            let b: Vec<_> = body
                .iter()
                .filter_map(|s| rewrite_private_stmt(s, wm_fields))
                .collect();
            Some(Stmt::Block { body: b })
        }
        other => Some(other.clone()),
    }
}

fn rewrite_private_expr(expr: &Expr, wm_fields: &HashMap<LocalId, String>) -> Option<Expr> {
    // Field init: ({__fi(){ wm.set(this, VAL) }}).__fi.call(this)
    if let Some((field, val)) = match_field_init(expr, wm_fields) {
        return Some(Expr::Assign {
            target: AssignTarget::Member {
                object: Box::new(Expr::This { ty: Type::Any }),
                property: Box::new(private_key_string(&field)),
                computed: false,
            },
            op: AssignOp::Eq,
            value: Box::new(val),
            ty: Type::Any,
        });
    }
    // Private get: (o => … wm.get(o) …)(obj)
    if let Some((obj, field)) = match_private_get(expr, wm_fields) {
        return Some(Expr::Member {
            object: Box::new(obj),
            property: Box::new(private_key_string(&field)),
            optional: false,
            computed: false,
            ty: Type::Any,
        });
    }
    // Private set sequence: pobj=this, pval=v, (o=>wm.set(o,pval))(pobj)
    if let Some(assign) = match_private_set_stmt_expr(expr, wm_fields) {
        return Some(assign);
    }
    match expr {
        Expr::Unary { op, arg, ty } => Some(Expr::Unary {
            op: *op,
            arg: Box::new(rewrite_private_expr(arg, wm_fields).unwrap_or_else(|| arg.as_ref().clone())),
            ty: ty.clone(),
        }),
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            // Flatten comma private-set sequences
            if *op == BinaryOp::Comma {
                if let Some(a) = match_private_set_stmt_expr(expr, wm_fields) {
                    return Some(a);
                }
            }
            Some(Expr::Binary {
                left: Box::new(
                    rewrite_private_expr(left, wm_fields).unwrap_or_else(|| left.as_ref().clone()),
                ),
                op: *op,
                right: Box::new(
                    rewrite_private_expr(right, wm_fields).unwrap_or_else(|| right.as_ref().clone()),
                ),
                ty: ty.clone(),
            })
        }
        Expr::Call {
            callee,
            args,
            optional,
            ty,
        } => {
            let new_args: Vec<Arg> = args
                .iter()
                .map(|a| match a {
                    Arg::Expr(e) => Arg::Expr(
                        rewrite_private_expr(e, wm_fields).unwrap_or_else(|| e.clone()),
                    ),
                    other => other.clone(),
                })
                .collect();
            Some(Expr::Call {
                callee: Box::new(
                    rewrite_private_expr(callee, wm_fields)
                        .unwrap_or_else(|| callee.as_ref().clone()),
                ),
                args: new_args,
                optional: *optional,
                ty: ty.clone(),
            })
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ty,
        } => Some(Expr::Member {
            object: Box::new(
                rewrite_private_expr(object, wm_fields).unwrap_or_else(|| object.as_ref().clone()),
            ),
            property: property.clone(),
            optional: *optional,
            computed: *computed,
            ty: ty.clone(),
        }),
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => Some(Expr::Assign {
            target: target.clone(),
            op: *op,
            value: Box::new(
                rewrite_private_expr(value, wm_fields).unwrap_or_else(|| value.as_ref().clone()),
            ),
            ty: ty.clone(),
        }),
        other => Some(other.clone()),
    }
}

fn match_private_get(expr: &Expr, wm_fields: &HashMap<LocalId, String>) -> Option<(Expr, String)> {
    let Expr::Call {
        callee,
        args,
        optional: false,
        ..
    } = expr
    else {
        return None;
    };
    if args.len() != 1 {
        return None;
    }
    let Arg::Expr(obj_expr) = &args[0] else {
        return None;
    };
    let Expr::Function {
        is_arrow: true,
        params,
        body,
        ..
    } = callee.as_ref()
    else {
        return None;
    };
    if params.len() != 1 {
        return None;
    }
    let Pattern::Local(param) = &params[0].pattern else {
        return None;
    };
    // body: return Cond(… wm.get(param) …)
    let Stmt::Return { value: Some(ret) } = body.last()? else {
        return None;
    };
    let wm = find_wm_get_in_expr(ret, *param)?;
    let field = wm_fields.get(&wm)?.clone();
    Some((obj_expr.clone(), field))
}

fn find_wm_get_in_expr(expr: &Expr, param: LocalId) -> Option<LocalId> {
    match expr {
        Expr::Conditional {
            consequent,
            alternate,
            ..
        } => find_wm_get_in_expr(consequent, param).or_else(|| find_wm_get_in_expr(alternate, param)),
        Expr::Call {
            callee,
            args,
            ..
        } if is_wm_method_call(callee, args, "get", param, None) => {
            wm_id_from_callee(callee)
        }
        Expr::Call { callee, args, .. } => {
            find_wm_get_in_expr(callee, param).or_else(|| {
                args.iter().find_map(|a| match a {
                    Arg::Expr(e) => find_wm_get_in_expr(e, param),
                    _ => None,
                })
            })
        }
        Expr::Binary { left, right, .. } => {
            find_wm_get_in_expr(left, param).or_else(|| find_wm_get_in_expr(right, param))
        }
        _ => None,
    }
}

fn match_private_set_stmt_expr(
    expr: &Expr,
    wm_fields: &HashMap<LocalId, String>,
) -> Option<Expr> {
    // Comma: (pobj = this, (pval = v, setcall(pobj)))
    let mut assigns: Vec<(&AssignTarget, &Expr)> = Vec::new();
    let mut tail = expr;
    while let Expr::Binary {
        left,
        op: BinaryOp::Comma,
        right,
        ..
    } = tail
    {
        if let Expr::Assign {
            target,
            op: AssignOp::Eq,
            value,
            ..
        } = left.as_ref()
        {
            assigns.push((target, value));
        }
        tail = right.as_ref();
    }
    // tail should be private set call
    let (obj, field, val) = match_private_set_call(tail, wm_fields)?;
    // Prefer value from pval assign if present
    let value = assigns
        .iter()
        .rev()
        .find_map(|(t, v)| match t {
            AssignTarget::Local(_) => Some((*v).clone()),
            _ => None,
        })
        .unwrap_or(val);
    // Rewrite private gets inside the RHS (e.g. `this.#n = this.#n + 1`).
    let value = rewrite_private_expr(&value, wm_fields).unwrap_or(value);
    let object = assigns
        .iter()
        .find_map(|(t, v)| match t {
            AssignTarget::Local(_) if matches!(v, Expr::This { .. }) => Some(Expr::This { ty: Type::Any }),
            _ => None,
        })
        .unwrap_or(obj);
    Some(Expr::Assign {
        target: AssignTarget::Member {
            object: Box::new(object),
            property: Box::new(private_key_string(&field)),
            computed: false,
        },
        op: AssignOp::Eq,
        value: Box::new(value),
        ty: Type::Any,
    })
}

fn match_private_set_call(
    expr: &Expr,
    wm_fields: &HashMap<LocalId, String>,
) -> Option<(Expr, String, Expr)> {
    let Expr::Call {
        callee,
        args,
        optional: false,
        ..
    } = expr
    else {
        return None;
    };
    if args.len() != 1 {
        return None;
    }
    let Arg::Expr(obj_expr) = &args[0] else {
        return None;
    };
    let Expr::Function {
        is_arrow: true,
        params,
        body,
        ..
    } = callee.as_ref()
    else {
        return None;
    };
    if params.len() != 1 {
        return None;
    }
    let Pattern::Local(param) = &params[0].pattern else {
        return None;
    };
    let Stmt::Return { value: Some(ret) } = body.last()? else {
        return None;
    };
    let (wm, val) = find_wm_set_in_expr(ret, *param)?;
    let field = wm_fields.get(&wm)?.clone();
    Some((obj_expr.clone(), field, val))
}

fn find_wm_set_in_expr(expr: &Expr, param: LocalId) -> Option<(LocalId, Expr)> {
    match expr {
        Expr::Conditional {
            consequent,
            alternate,
            ..
        } => find_wm_set_in_expr(consequent, param)
            .or_else(|| find_wm_set_in_expr(alternate, param)),
        Expr::Binary {
            op: BinaryOp::Comma,
            left,
            right,
            ..
        } => find_wm_set_in_expr(left, param).or_else(|| find_wm_set_in_expr(right, param)),
        Expr::Call {
            callee,
            args,
            ..
        } if is_wm_method_call(callee, args, "set", param, None) && args.len() == 2 => {
            let id = wm_id_from_callee(callee)?;
            let Arg::Expr(val) = &args[1] else {
                return None;
            };
            Some((id, val.clone()))
        }
        Expr::Call { callee, args, .. } => find_wm_set_in_expr(callee, param).or_else(|| {
            args.iter().find_map(|a| match a {
                Arg::Expr(e) => find_wm_set_in_expr(e, param),
                _ => None,
            })
        }),
        _ => None,
    }
}

fn match_field_init(expr: &Expr, wm_fields: &HashMap<LocalId, String>) -> Option<(String, Expr)> {
    // ({ __fi: function() { helper(this, VAL) } }).__fi.call(this)
    let Expr::Call {
        callee,
        args,
        optional: false,
        ..
    } = expr
    else {
        return None;
    };
    if args.len() != 1 {
        return None;
    }
    // Base ctors pass `this`; derived field inits pass the post-super this local.
    if !matches!(
        &args[0],
        Arg::Expr(Expr::This { .. }) | Arg::Expr(Expr::Local { .. })
    ) {
        return None;
    }
    // callee: Member(Member(Object, "__fi"), "call")
    let Expr::Member {
        object: mid,
        property: call_prop,
        ..
    } = callee.as_ref()
    else {
        return None;
    };
    let Expr::String { value: call_s, .. } = call_prop.as_ref() else {
        return None;
    };
    if call_s.to_string_lossy() != "call" {
        return None;
    }
    let Expr::Member {
        object: obj_lit,
        property: fi_prop,
        ..
    } = mid.as_ref()
    else {
        return None;
    };
    let Expr::String { value: fi_s, .. } = fi_prop.as_ref() else {
        return None;
    };
    if fi_s.to_string_lossy() != "__fi" {
        return None;
    }
    let Expr::Object { properties, .. } = obj_lit.as_ref() else {
        return None;
    };
    let mut fi_fn: Option<&Expr> = None;
    for p in properties {
        if let ObjectProp::Property {
            key: ObjectPropKey::Static(k),
            value,
        } = p
        {
            if js_string_eq(k, "__fi") {
                fi_fn = Some(value);
            }
        }
    }
    let fi_fn = fi_fn?;
    let Expr::Function { body, .. } = fi_fn else {
        return None;
    };
    // Find Call helper(this, VAL) inside body
    for s in body {
        if let Stmt::Expr { expr: e } = s {
            if let Some(pair) = find_field_init_set(e, wm_fields) {
                return Some(pair);
            }
        }
    }
    None
}

fn js_string_eq(k: &draconic_ast::JsString, s: &str) -> bool {
    k.to_string_lossy() == s
}

fn find_field_init_set(
    expr: &Expr,
    wm_fields: &HashMap<LocalId, String>,
) -> Option<(String, Expr)> {
    // Call(arrow(o,v)=>…wm.set(o,v)…, this, VAL)
    let Expr::Call {
        callee,
        args,
        ..
    } = expr
    else {
        // recurse
        return match expr {
            Expr::Call { callee, args, .. } => find_field_init_set(callee, wm_fields).or_else(|| {
                args.iter().find_map(|a| match a {
                    Arg::Expr(e) => find_field_init_set(e, wm_fields),
                    _ => None,
                })
            }),
            _ => None,
        };
    };
    if args.len() == 2 {
        if let (Arg::Expr(Expr::This { .. }), Arg::Expr(val)) = (&args[0], &args[1]) {
            if let Expr::Function {
                is_arrow: true,
                params,
                body,
                ..
            } = callee.as_ref()
            {
                if params.len() == 2 {
                    if let (Pattern::Local(p0), Pattern::Local(p1)) =
                        (&params[0].pattern, &params[1].pattern)
                    {
                        for s in body {
                            if let Stmt::Return { value: Some(ret) } = s {
                                if let Some(wm) = find_wm_set_param(ret, *p0, *p1) {
                                    let field = wm_fields.get(&wm)?.clone();
                                    return Some((field, val.clone()));
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    // recurse into nested
    find_field_init_set_nested(expr, wm_fields)
}

fn find_field_init_set_nested(
    expr: &Expr,
    wm_fields: &HashMap<LocalId, String>,
) -> Option<(String, Expr)> {
    match expr {
        Expr::Call { callee, args, .. } => {
            if let Some(p) = find_field_init_set(callee, wm_fields) {
                return Some(p);
            }
            for a in args {
                if let Arg::Expr(e) = a {
                    if let Some(p) = find_field_init_set(e, wm_fields) {
                        return Some(p);
                    }
                }
            }
            None
        }
        Expr::Function { body, .. } => {
            for s in body {
                if let Stmt::Expr { expr: e } = s {
                    if let Some(p) = find_field_init_set(e, wm_fields) {
                        return Some(p);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn find_wm_set_param(expr: &Expr, obj_param: LocalId, val_param: LocalId) -> Option<LocalId> {
    match expr {
        Expr::Conditional {
            consequent,
            alternate,
            ..
        } => find_wm_set_param(consequent, obj_param, val_param)
            .or_else(|| find_wm_set_param(alternate, obj_param, val_param)),
        Expr::Binary {
            left, right, ..
        } => find_wm_set_param(left, obj_param, val_param)
            .or_else(|| find_wm_set_param(right, obj_param, val_param)),
        Expr::Unary { arg, .. } => find_wm_set_param(arg, obj_param, val_param),
        Expr::Call {
            callee,
            args,
            ..
        } if is_wm_method_call(callee, args, "set", obj_param, Some(val_param)) => {
            wm_id_from_callee(callee)
        }
        Expr::Call { callee, args, .. } => find_wm_set_param(callee, obj_param, val_param).or_else(
            || {
                args.iter().find_map(|a| match a {
                    Arg::Expr(e) => find_wm_set_param(e, obj_param, val_param),
                    _ => None,
                })
            },
        ),
        _ => None,
    }
}

fn is_object_set_prototype_of(callee: &Expr) -> bool {
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

fn is_object_define_property(callee: &Expr) -> bool {
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

fn is_define_on_ctor(args: &[Arg], ctor: LocalId) -> bool {
    matches!(
        &args[0],
        Arg::Expr(Expr::Local { id, .. }) if *id == ctor
    )
}

fn is_define_on_proto(args: &[Arg], ctor: LocalId) -> bool {
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

fn string_arg(arg: &Arg) -> Option<String> {
    match arg {
        Arg::Expr(Expr::String { value, .. }) => Some(value.to_string_lossy()),
        Arg::Expr(Expr::Local { .. }) => None,
        _ => None,
    }
}

fn find_method_function(expr: &Expr) -> Option<&Expr> {
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
            .or_else(|| alternate.as_ref().and_then(|a| find_method_function_in_stmt(a))),
        _ => None,
    }
}

fn filter_ctor_body(body: &[Stmt]) -> (Vec<Stmt>, Vec<(String, FieldVal)>) {
    let mut out = Vec::new();
    let mut fields = Vec::new();
    for s in body {
        match s {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            Stmt::If { .. } => {} // new.target check
            Stmt::Expr { expr } => {
                if let Some((name, fv, assign)) = try_field_init_assign(expr) {
                    fields.push((name, fv));
                    out.push(assign);
                } else if matches!(
                    expr,
                    Expr::Assign {
                        target: AssignTarget::Member { .. },
                        op: AssignOp::Eq,
                        ..
                    }
                ) {
                    out.push(s.clone());
                } else if matches!(expr, Expr::Call { .. } | Expr::Binary { op: BinaryOp::Comma, .. }) {
                    // Private field inits / assigns kept for rewrite_private_stmts.
                    out.push(s.clone());
                }
            }
            Stmt::Return { .. } | Stmt::Block { .. } => out.push(s.clone()),
            _ => {}
        }
    }
    (out, fields)
}

/// Collapse derived-ctor IR (this-TDZ + Reflect.construct super IIFE) into:
/// `super(args…); this.prop = …;`
fn filter_derived_ctor_body(body: &[Stmt]) -> (Vec<Stmt>, Vec<(String, FieldVal)>) {
    let mut out = Vec::new();
    let mut fields = Vec::new();
    collect_derived_ctor_stmts(body, &mut out, &mut fields);
    (out, fields)
}

fn collect_derived_ctor_stmts(
    body: &[Stmt],
    out: &mut Vec<Stmt>,
    fields: &mut Vec<(String, FieldVal)>,
) {
    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            Stmt::If { .. } | Stmt::Declare { .. } | Stmt::Return { .. } => {}
            Stmt::Try {
                block,
                handler,
                finalizer,
                ..
            } => {
                collect_derived_ctor_stmts(block, out, fields);
                if let Some(h) = handler {
                    collect_derived_ctor_stmts(h, out, fields);
                }
                if let Some(f) = finalizer {
                    collect_derived_ctor_stmts(f, out, fields);
                }
            }
            Stmt::Labeled { body, .. } => collect_derived_ctor_stmts_one(body, out, fields),
            Stmt::Block { body } => collect_derived_ctor_stmts(body, out, fields),
            other => collect_derived_ctor_stmts_one(other, out, fields),
        }
    }
}

fn collect_derived_ctor_stmts_one(
    stmt: &Stmt,
    out: &mut Vec<Stmt>,
    fields: &mut Vec<(String, FieldVal)>,
) {
    match stmt {
        Stmt::Block { body } => collect_derived_ctor_stmts(body, out, fields),
        Stmt::Labeled { body, .. } => collect_derived_ctor_stmts_one(body, out, fields),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            collect_derived_ctor_stmts(block, out, fields);
            if let Some(h) = handler {
                collect_derived_ctor_stmts(h, out, fields);
            }
            if let Some(f) = finalizer {
                collect_derived_ctor_stmts(f, out, fields);
            }
        }
        Stmt::Expr {
            expr:
                Expr::Call {
                    callee,
                    args,
                    optional,
                    ty,
                },
        } if !*optional && is_super_call_iife(callee) => {
            let super_args: Vec<Arg> = args
                .iter()
                .filter_map(|a| match a {
                    Arg::Expr(e) => Some(Arg::Expr(e.clone())),
                    Arg::Spread(_) => None,
                })
                .collect();
            if super_args.len() != args.len() {
                return;
            }
            out.push(Stmt::Expr {
                expr: Expr::Call {
                    callee: Box::new(Expr::Super { ty: Type::Any }),
                    args: super_args,
                    optional: false,
                    ty: ty.clone(),
                },
            });
            // Instance field inits nested inside the super() IIFE after construct.
            if let Expr::Function { body, .. } = callee.as_ref() {
                collect_derived_ctor_stmts(body, out, fields);
            }
        }
        Stmt::Expr { expr } => {
            if let Some((name, fv, assign)) = try_field_init_assign(expr) {
                fields.push((name, fv));
                out.push(assign);
            } else if let Expr::Assign {
                target:
                    AssignTarget::Member {
                        object: _,
                        property,
                        computed,
                    },
                op: AssignOp::Eq,
                value,
                ty,
            } = expr
            {
                if matches!(property.as_ref(), Expr::String { .. }) {
                    out.push(Stmt::Expr {
                        expr: Expr::Assign {
                            target: AssignTarget::Member {
                                object: Box::new(Expr::This { ty: Type::Any }),
                                property: property.clone(),
                                computed: *computed,
                            },
                            op: AssignOp::Eq,
                            value: value.clone(),
                            ty: ty.clone(),
                        },
                    });
                }
            } else if matches!(expr, Expr::Call { .. } | Expr::Binary { op: BinaryOp::Comma, .. }) {
                out.push(stmt.clone());
            }
        }
        _ => {}
    }
}

/// Instance field init: `({__proto__, __fi(){ Object.defineProperty(this,k,{value}) }}).__fi.call(recv)`
/// → `(name, FieldVal, this.name = value)`.
fn try_field_init_assign(expr: &Expr) -> Option<(String, FieldVal, Stmt)> {
    let (key, value) = extract_instance_field_define(expr)?;
    let fv = field_val_from_expr(&value)?;
    let assign = Stmt::Expr {
        expr: Expr::Assign {
            target: AssignTarget::Member {
                object: Box::new(Expr::This { ty: Type::Any }),
                property: Box::new(Expr::String {
                    value: key.clone().into(),
                    ty: Type::String,
                }),
                computed: false,
            },
            op: AssignOp::Eq,
            value: Box::new(value),
            ty: Type::Any,
        },
    };
    Some((key, fv, assign))
}

fn extract_instance_field_define(expr: &Expr) -> Option<(String, Expr)> {
    let Expr::Call {
        callee,
        args,
        optional,
        ..
    } = expr
    else {
        return None;
    };
    if *optional || args.len() != 1 {
        return None;
    }
    match &args[0] {
        Arg::Expr(Expr::This { .. } | Expr::Local { .. }) => {}
        _ => return None,
    }
    let fi_fn = fi_method_from_call_callee(callee)?;
    let Expr::Function { body, .. } = fi_fn else {
        return None;
    };
    for s in body {
        if let Stmt::Expr {
            expr:
                Expr::Call {
                    callee: def_c,
                    args: def_a,
                    ..
                },
        } = s
        {
            if is_object_define_property(def_c) && def_a.len() == 3 {
                let key = string_arg(&def_a[1])?;
                let Arg::Expr(desc) = &def_a[2] else {
                    return None;
                };
                let val = object_prop_value(desc, "value")?;
                return Some((key, val));
            }
        }
    }
    None
}

fn fi_method_from_call_callee(callee: &Expr) -> Option<&Expr> {
    let Expr::Member {
        object,
        property,
        optional,
        ..
    } = callee
    else {
        return None;
    };
    if *optional
        || !matches!(property.as_ref(), Expr::String { value, .. } if value.to_string_lossy() == "call")
    {
        return None;
    }
    let Expr::Member {
        object: obj,
        property: fi_key,
        optional: opt2,
        ..
    } = object.as_ref()
    else {
        return None;
    };
    if *opt2
        || !matches!(fi_key.as_ref(), Expr::String { value, .. } if value.to_string_lossy() == "__fi")
    {
        return None;
    }
    let Expr::Object { properties, .. } = obj.as_ref() else {
        return None;
    };
    for p in properties {
        if let ObjectProp::Property { key, value, .. } = p {
            if let ObjectPropKey::Static(k) = key {
                if k.to_string_lossy() == "__fi" {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn static_field_val_from_desc(desc: &Expr) -> Option<FieldVal> {
    let val = object_prop_value(desc, "value")?;
    if let Some(ret) = fi_call_return_expr(&val) {
        return field_val_from_expr(ret);
    }
    field_val_from_expr(&val)
}

fn fi_call_return_expr(expr: &Expr) -> Option<&Expr> {
    let Expr::Call {
        callee,
        args,
        optional,
        ..
    } = expr
    else {
        return None;
    };
    if *optional || args.is_empty() {
        return None;
    }
    let fi_fn = fi_method_from_call_callee(callee)?;
    let Expr::Function { body, .. } = fi_fn else {
        return None;
    };
    for s in body {
        match s {
            Stmt::Return { value: Some(v) } => return Some(v),
            Stmt::Expr {
                expr: Expr::String { .. },
            } => {}
            _ => {}
        }
    }
    None
}

fn object_prop_value(obj: &Expr, name: &str) -> Option<Expr> {
    let Expr::Object { properties, .. } = obj else {
        return None;
    };
    for p in properties {
        if let ObjectProp::Property { key, value, .. } = p {
            if let ObjectPropKey::Static(s) = key {
                if s.to_string_lossy() == name {
                    return Some(value.clone());
                }
            }
        }
    }
    None
}

fn field_val_from_expr(expr: &Expr) -> Option<FieldVal> {
    match expr {
        Expr::Number { .. } => Some(FieldVal::Number(expr.clone())),
        Expr::String { value, .. } => Some(FieldVal::String(value.to_string_lossy())),
        Expr::IdentName { name, .. } if name == "undefined" => Some(FieldVal::Undef),
        Expr::Unary {
            op: UnaryOp::Void, ..
        } => Some(FieldVal::Undef),
        Expr::Binary {
            left,
            op,
            right,
            ..
        } if matches!(
            op,
            BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
        ) && field_val_from_expr(left).is_some()
            && field_val_from_expr(right).is_some() =>
        {
            Some(FieldVal::Number(expr.clone()))
        }
        _ => None,
    }
}

fn descriptor_direct_method_fn(desc: &Expr) -> Option<&Expr> {
    if let Expr::Object { properties, .. } = desc {
        for p in properties {
            if let ObjectProp::Property { key, value, .. } = p {
                if let ObjectPropKey::Static(k) = key {
                    if k.to_string_lossy() == "value" {
                        if matches!(value, Expr::Function { .. }) {
                            return Some(value);
                        }
                        return None;
                    }
                }
            }
        }
    }
    find_method_function(desc)
}



fn is_super_call_iife(callee: &Expr) -> bool {
    let Expr::Function {
        body,
        is_arrow: true,
        ..
    } = callee
    else {
        return false;
    };
    body.iter().any(stmt_has_reflect_construct)
}

fn stmt_has_reflect_construct(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Declare {
            init: Some(expr), ..
        }
        | Stmt::Expr { expr }
        | Stmt::Return { value: Some(expr) } => expr_has_reflect_construct(expr),
        Stmt::Block { body } => body.iter().any(stmt_has_reflect_construct),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            stmt_has_reflect_construct(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_has_reflect_construct(a))
        }
        _ => false,
    }
}

fn expr_has_reflect_construct(expr: &Expr) -> bool {
    match expr {
        Expr::Call { callee, .. } => {
            if is_reflect_construct(callee) {
                return true;
            }
            expr_has_reflect_construct(callee)
        }
        Expr::Member {
            object, property, ..
        } => expr_has_reflect_construct(object) || expr_has_reflect_construct(property),
        Expr::Assign { value, .. } => expr_has_reflect_construct(value),
        _ => false,
    }
}

fn is_reflect_construct(callee: &Expr) -> bool {
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
        ) if name == "Reflect" && value.to_string_lossy() == "construct"
    )
}

fn filter_method_body(body: &[Stmt]) -> Vec<Stmt> {
    body.iter()
        .filter(|s| {
            !matches!(
                s,
                Stmt::Expr {
                    expr: Expr::String { value, .. },
                } if value.to_string_lossy() == "use strict"
            )
        })
        .cloned()
        .collect()
}

fn simple_param_ids(params: &[Param], by_id: &HashMap<LocalId, &Local>) -> Option<Vec<LocalId>> {
    let mut ids = Vec::new();
    for p in params {
        if p.rest || p.default.is_some() {
            return None;
        }
        match &p.pattern {
            Pattern::Local(id) => {
                let _ = by_id.get(id)?;
                ids.push(*id);
            }
            _ => return None,
        }
    }
    if ids.len() > MAX_METHOD_ARGS {
        return None;
    }
    Some(ids)
}

fn method_body_ok(body: &[Stmt], by_id: &HashMap<LocalId, &Local>) -> bool {
    body.iter().all(|s| method_stmt_ok(s, by_id))
}

fn method_stmt_ok(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Return { value: None } => true,
        Stmt::Return { value: Some(e) } => {
            number_expr_ok_method(e, by_id) || string_expr_ok_method(e, by_id)
        }
        Stmt::Block { body } => body.iter().all(|s| method_stmt_ok(s, by_id)),
        Stmt::Expr {
            expr:
                Expr::Call {
                    callee: c,
                    args,
                    optional,
                    ..
                },
        } if matches!(c.as_ref(), Expr::Super { .. }) && !*optional => args.iter().all(|a| match a {
            Arg::Expr(e) => number_expr_ok_method(e, by_id),
            Arg::Spread(_) => false,
        }),
        Stmt::Expr {
            expr:
                Expr::Assign {
                    target:
                        AssignTarget::Member {
                            object,
                            property,
                            ..
                        },
                    op: AssignOp::Eq,
                    value,
                    ..
                },
        } => {
            matches!(object.as_ref(), Expr::This { .. })
                && matches!(property.as_ref(), Expr::String { .. })
                && (number_expr_ok_method(value, by_id) || is_undefined_expr(value))
        }
        _ => false,
    }
}

fn method_ret_kind(body: &[Stmt]) -> MethodRet {
    for s in body {
        if let Stmt::Return { value: Some(e) } = s {
            if string_expr_ok_method(e, &HashMap::new()) {
                return MethodRet::String;
            }
        }
    }
    MethodRet::Number
}

fn string_expr_ok_method(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => number_expr_ok_method(arg, by_id)
            || matches!(arg.as_ref(), Expr::Member { .. })
            || matches!(arg.as_ref(), Expr::Local { .. }),
        Expr::String { .. } => true,
        _ => false,
    }
}

fn is_undefined_expr(expr: &Expr) -> bool {
    match expr {
        Expr::IdentName { name, .. } if name == "undefined" => true,
        Expr::Unary {
            op: UnaryOp::Void,
            ..
        } => true,
        _ => false,
    }
}

fn number_expr_ok_method(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            matches!(ty, Type::Number | Type::Any)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::This { .. } => false,
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && matches!(object.as_ref(), Expr::This { .. })
                && matches!(property.as_ref(), Expr::String { .. })
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && (super_method_callee_ok(callee) || this_method_callee_ok(callee))
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => number_expr_ok_method(e, by_id),
                    Arg::Spread(_) => false,
                })
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
            ) && number_expr_ok_method(left, by_id)
                && number_expr_ok_method(right, by_id)
        }
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            matches!(object.as_ref(), Expr::This { .. })
                && matches!(property.as_ref(), Expr::String { .. })
                && number_expr_ok_method(value, by_id)
        }
        _ => false,
    }
}

/// `super.m` / `super["m"]` as call callee (IR keeps bare Super in methods).
fn super_method_callee_ok(callee: &Expr) -> bool {
    match callee {
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && matches!(object.as_ref(), Expr::Super { .. })
                && matches!(property.as_ref(), Expr::String { .. })
        }
        _ => false,
    }
}

/// `this.m(...)` call callee in methods (e.g. Child.total → this.base()).
fn this_method_callee_ok(callee: &Expr) -> bool {
    match callee {
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && matches!(object.as_ref(), Expr::This { .. })
                && matches!(property.as_ref(), Expr::String { .. })
        }
        _ => false,
    }
}

fn is_object_slot(
    init: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
) -> bool {
    match init {
        Expr::New { .. } => true,
        Expr::Local { id, ty } => {
            class_of.contains_key(id)
                || matches!(ty, Type::Object | Type::Function)
                || by_id.get(id).is_some_and(|l| {
                    class_of.contains_key(id)
                        || matches!(l.ty, Type::Object | Type::Function | Type::Shape(_))
                })
        }
        // Member reads of instance props are numbers in class_basic fixtures
        // (typed `any`); do not treat as object slots.
        _ => false,
    }
}

fn object_expr_ok(
    expr: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    classes: &[ClassInfo],
) -> bool {
    match expr {
        Expr::This { .. } => true,
        Expr::New {
            callee,
            args,
            ..
        } => {
            let Expr::Local { id, .. } = callee.as_ref() else {
                return false;
            };
            if !class_of.contains_key(id) {
                return false;
            }
            args.iter().all(|a| match a {
                Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions, classes),
                Arg::Spread(_) => false,
            })
        }
        Expr::Local { id, .. } => {
            class_of.contains_key(id)
                || by_id.get(id).is_some_and(|l| {
                    matches!(l.ty, Type::Object | Type::Function | Type::Any | Type::Shape(_))
                })
        }
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && object_expr_ok(object, class_of, by_id, functions, classes)
                && matches!(property.as_ref(), Expr::String { .. })
        }
        _ => false,
    }
}

fn number_expr_ok(
    expr: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    classes: &[ClassInfo],
) -> bool {
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
        } => {
            !*optional
                && object_expr_ok(object, class_of, by_id, functions, classes)
                && matches!(property.as_ref(), Expr::String { .. })
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && method_callee_ok(callee, class_of, by_id, functions, classes)
                && method_call_returns_number(callee, classes, functions)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions, classes),
                    Arg::Spread(_) => false,
                })
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
            ) && number_expr_ok(left, class_of, by_id, functions, classes)
                && number_expr_ok(right, class_of, by_id, functions, classes)
        }
        Expr::New { .. } => false,
        _ => false,
    }
}

fn typeof_string_expr_ok(
    expr: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    classes: &[ClassInfo],
) -> bool {
    match expr {
        Expr::Unary {
            op: UnaryOp::TypeOf,
            arg,
            ..
        } => {
            // `typeof obj.missing` / `typeof` of values.
            matches!(arg.as_ref(), Expr::Member { optional: false, .. })
                || object_expr_ok(arg, class_of, by_id, functions, classes)
                || number_expr_ok(arg, class_of, by_id, functions, classes)
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && method_callee_ok(callee, class_of, by_id, functions, classes)
                && method_call_returns_string(callee, classes, functions)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions, classes),
                    Arg::Spread(_) => false,
                })
        }
        Expr::String { .. } => true,
        _ => false,
    }
}

fn method_call_returns_number(
    callee: &Expr,
    classes: &[ClassInfo],
    functions: &[FnInfo],
) -> bool {
    match method_call_ret(callee, classes, functions) {
        Some(MethodRet::String) => false,
        _ => true,
    }
}

fn method_call_returns_string(
    callee: &Expr,
    classes: &[ClassInfo],
    functions: &[FnInfo],
) -> bool {
    method_call_ret(callee, classes, functions) == Some(MethodRet::String)
}

fn method_call_ret(
    callee: &Expr,
    classes: &[ClassInfo],
    functions: &[FnInfo],
) -> Option<MethodRet> {
    let Expr::Member { property, .. } = callee else {
        return None;
    };
    let Expr::String { value, .. } = property.as_ref() else {
        return None;
    };
    let name = value.to_string_lossy();
    let mut found: Option<MethodRet> = None;
    for c in classes {
        for (mname, idx) in c.methods.iter().chain(c.static_methods.iter()) {
            if *mname == name {
                let r = functions[*idx].ret;
                if let Some(prev) = found {
                    if prev != r {
                        return None;
                    }
                }
                found = Some(r);
            }
        }
    }
    found
}

fn method_callee_ok(
    callee: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    classes: &[ClassInfo],
) -> bool {
    match callee {
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && object_expr_ok(object, class_of, by_id, functions, classes)
                && matches!(property.as_ref(), Expr::String { .. })
        }
        _ => false,
    }
}

fn side_effect_ok(
    expr: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
    classes: &[ClassInfo],
) -> bool {
    match expr {
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && method_callee_ok(callee, class_of, by_id, functions, classes)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions, classes),
                    Arg::Spread(_) => false,
                })
        }
        _ => false,
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

impl<'a> Emitter<'a> {
    fn new(module: &'a Module, info: &'a ModuleInfo) -> Self {
        let mut class_binding = HashMap::new();
        for (id, idx) in &info.class_of {
            class_binding.insert(*idx, *id);
        }
        Self {
            module,
            info,
            out: String::new(),
            body: String::new(),
            allocas: HashMap::new(),
            slot_of: HashMap::new(),
            param_allocas: HashMap::new(),
            this_ssa: None,
            active_parent_ctor: None,
            active_super_class: None,
            active_method_ret: MethodRet::Number,
            class_binding,
            str_globals: Vec::new(),
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
            "; Draconic LLVM backend (N08.05/N08.16.26/N08.16.36 ES classes + public/private fields via Runtime ABI)"
        )
        .ok();
        writeln!(
            self.out,
            "{}",
            llvm_declares(&[
                GC_INIT,
                ALLOC_OBJECT,
                OBJECT_SET,
                OBJECT_GET,
                OBJECT_SET_PROTO,
                PRINT_F64,
                PRINT_STR,
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for (id, kind) in &info.slots {
            match kind {
                SlotTy::Number => {
                    let g = number_global_name(*id);
                    writeln!(
                        self.out,
                        "@{g} = internal global double 0.00000000000000000e+00, align 8"
                    )
                    .ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::String => {
                    let g = string_global_name(*id);
                    writeln!(self.out, "@{g} = internal global ptr null, align 8").ok();
                    self.allocas.insert(*id, format!("@{g}"));
                }
                SlotTy::Object | SlotTy::Undefined => {}
            }
        }
        if info
            .slots
            .iter()
            .any(|(_, k)| matches!(k, SlotTy::Number | SlotTy::String))
        {
            writeln!(self.out).ok();
        }

        for f in &info.functions.clone() {
            self.emit_method_fn(f)?;
        }

        for (id, kind) in &info.slots {
            if *kind != SlotTy::Object {
                continue;
            }
            let ptr = format!("%l{}", id.0);
            self.allocas.insert(*id, ptr.clone());
            writeln!(self.body, "  {ptr} = alloca ptr, align 8").ok();
            writeln!(self.body, "  store ptr null, ptr {ptr}").ok();
        }

        for stmt in &self.module.body {
            self.emit_stmt(stmt)?;
        }

        for id in &info.observe_locals {
            match self.slot_of.get(id).copied() {
                Some(SlotTy::Number) => {
                    let ptr = self.number_slot_ptr(*id)?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load double, ptr {ptr}").ok();
                    let bits = self.fresh();
                    writeln!(self.body, "  {bits} = bitcast double {v} to i64").ok();
                    let is_u = self.fresh();
                    writeln!(
                        self.body,
                        "  {is_u} = icmp eq i64 {bits}, {UNDEF_BITS}"
                    )
                    .ok();
                    let und_l = format!("print_und_{}", id.0);
                    let num_l = format!("print_num_{}", id.0);
                    let end_l = format!("print_end_{}", id.0);
                    writeln!(
                        self.body,
                        "  br i1 {is_u}, label %{und_l}, label %{num_l}"
                    )
                    .ok();
                    writeln!(self.body, "{und_l}:").ok();
                    self.emit_print_str_lit("undefined")?;
                    writeln!(self.body, "  br label %{end_l}").ok();
                    writeln!(self.body, "{num_l}:").ok();
                    writeln!(self.body, "  {}", PRINT_F64.call(&format!("double {v}"))).ok();
                    writeln!(self.body, "  br label %{end_l}").ok();
                    writeln!(self.body, "{end_l}:").ok();
                }
                Some(SlotTy::String) => {
                    let ptr = self
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("es_classes: string slot missing"))?;
                    let v = self.fresh();
                    writeln!(self.body, "  {v} = load ptr, ptr {ptr}").ok();
                    writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {v}"))).ok();
                }
                Some(SlotTy::Undefined) => {
                    self.emit_print_str_lit("undefined")?;
                }
                _ => return Err(diag("es_classes: bad observe slot")),
            }
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

    fn emit_method_fn(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let name = format!("m_fn_{}", f.idx);
        let mut params_s = String::from("ptr %this");
        for i in 0..MAX_METHOD_ARGS {
            write!(params_s, ", double %a{i}").ok();
        }
        let ret_ty = match f.ret {
            MethodRet::Number => "double",
            MethodRet::String => "ptr",
        };
        writeln!(self.out, "define {ret_ty} @{name}({params_s}) {{").ok();
        writeln!(self.out, "entry:").ok();

        let saved_body = std::mem::take(&mut self.body);
        let saved_this = self.this_ssa.take();
        let saved_params = std::mem::take(&mut self.param_allocas);
        let saved_allocas = std::mem::take(&mut self.allocas);
        let saved_parent = self.active_parent_ctor.take();
        let saved_super = self.active_super_class.take();
        let saved_ret = self.active_method_ret;
        self.active_method_ret = f.ret;

        self.this_ssa = Some("%this".to_string());
        self.active_parent_ctor = f.parent_ctor_fn_idx;
        self.active_super_class = f.super_class_idx;
        for (i, pid) in f.params.iter().enumerate() {
            let ptr = format!("%p{}", pid.0);
            writeln!(self.body, "  {ptr} = alloca double, align 8").ok();
            writeln!(self.body, "  store double %a{i}, ptr {ptr}").ok();
            self.param_allocas.insert(*pid, ptr);
        }

        let mut saw_return = false;
        for stmt in &f.body {
            if matches!(stmt, Stmt::Return { .. }) {
                saw_return = true;
            }
            self.emit_method_stmt(stmt)?;
        }
        if !saw_return {
            match f.ret {
                MethodRet::Number => {
                    writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
                }
                MethodRet::String => {
                    let s = self.string_const("undefined")?;
                    writeln!(self.body, "  ret ptr {s}").ok();
                }
            }
        }

        self.out.push_str(&self.body);
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.this_ssa = saved_this;
        self.param_allocas = saved_params;
        self.allocas = saved_allocas;
        self.active_parent_ctor = saved_parent;
        self.active_super_class = saved_super;
        self.active_method_ret = saved_ret;
        Ok(())
    }

    fn emit_method_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(e) } => {
                match self.active_method_ret {
                    MethodRet::Number => {
                        let v = self.emit_number_expr(e)?;
                        writeln!(self.body, "  ret double {v}").ok();
                    }
                    MethodRet::String => {
                        let v = self.emit_string_expr(e)?;
                        writeln!(self.body, "  ret ptr {v}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Return { value: None } => {
                match self.active_method_ret {
                    MethodRet::Number => {
                        writeln!(self.body, "  ret double 0.00000000000000000e+00").ok();
                    }
                    MethodRet::String => {
                        let s = self.string_const("undefined")?;
                        writeln!(self.body, "  ret ptr {s}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Block { body } => {
                for s in body {
                    self.emit_method_stmt(s)?;
                }
                Ok(())
            }
            Stmt::Expr {
                expr:
                    Expr::Call {
                        callee,
                        args,
                        optional,
                        ..
                    },
            } if matches!(callee.as_ref(), Expr::Super { .. }) => {
                if *optional {
                    return Err(diag("es_classes: optional super call"));
                }
                self.emit_super_call(args)?;
                Ok(())
            }
            Stmt::Expr { expr } => self.emit_side_effect_expr(expr),
            _ => Err(diag("es_classes: unsupported method stmt")),
        }
    }

    fn emit_super_call(&mut self, args: &[Arg]) -> Result<(), Diagnostic> {
        let parent = self
            .active_parent_ctor
            .ok_or_else(|| diag("es_classes: super() outside derived ctor"))?;
        let this = self
            .this_ssa
            .clone()
            .ok_or_else(|| diag("es_classes: super() without this"))?;
        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_classes: spread super args not supported"));
                }
            }
        }
        while arg_vals.len() < MAX_METHOD_ARGS {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }
        let mut call_args = format!("ptr {this}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        let mut ty_params = String::from("ptr");
        for _ in 0..MAX_METHOD_ARGS {
            ty_params.push_str(", double");
        }
        let ret = self.fresh();
        writeln!(
            self.body,
            "  {ret} = call double ({ty_params}) @m_fn_{parent}({call_args})"
        )
        .ok();
        let _ = ret;
        Ok(())
    }

    fn emit_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let Some(init) = init else {
                    return Ok(());
                };
                let Some(kind) = self.slot_of.get(local).copied() else {
                    return Ok(());
                };
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        let ptr = self.number_slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
                    SlotTy::String => {
                        let v = self.emit_string_expr(init)?;
                        let ptr = self
                            .allocas
                            .get(local)
                            .cloned()
                            .ok_or_else(|| diag("es_classes: string alloca missing"))?;
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                    SlotTy::Undefined => {}
                    SlotTy::Object => {
                        let v = if let Some(ci) = self.info.class_of.get(local) {
                            self.emit_class_ctor(*ci)?
                        } else {
                            self.emit_object_expr(init)?
                        };
                        let ptr = self.allocas.get(local).cloned().unwrap();
                        writeln!(self.body, "  store ptr {v}, ptr {ptr}").ok();
                    }
                }
                Ok(())
            }
            Stmt::Expr { expr } => self.emit_side_effect_expr(expr),
            _ => Err(diag("es_classes: unsupported stmt")),
        }
    }

    fn emit_class_ctor(&mut self, class_idx: usize) -> Result<String, Diagnostic> {
        let cls = self.info.classes[class_idx].clone();
        let ctor = self.fresh();
        writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&ctor, "")).ok();
        let proto = self.fresh();
        writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&proto, "")).ok();
        let key = self.string_const("prototype")?;
        writeln!(
            self.body,
            "  {}",
            OBJECT_SET.call(&format!("ptr {ctor}, ptr {key}, ptr {proto}"))
        )
        .ok();
        if let Some(parent_idx) = cls.parent {
            let parent_binding = *self
                .class_binding
                .get(&parent_idx)
                .ok_or_else(|| diag("es_classes: parent class binding missing"))?;
            let parent_ptr = self
                .allocas
                .get(&parent_binding)
                .cloned()
                .ok_or_else(|| diag("es_classes: parent class alloca missing"))?;
            let parent_ctor = self.fresh();
            writeln!(self.body, "  {parent_ctor} = load ptr, ptr {parent_ptr}").ok();
            let parent_proto = self.fresh();
            writeln!(
                self.body,
                "  {}",
                OBJECT_GET.call_to(&parent_proto, &format!("ptr {parent_ctor}, ptr {key}"))
            )
            .ok();
            writeln!(
                self.body,
                "  {}",
                OBJECT_SET_PROTO.call(&format!("ptr {proto}, ptr {parent_proto}"))
            )
            .ok();
        }
        for (name, fn_idx) in &cls.methods {
            let mkey = self.string_const(name)?;
            let fptr = format!("@m_fn_{fn_idx}");
            writeln!(
                self.body,
                "  {}",
                OBJECT_SET.call(&format!("ptr {proto}, ptr {mkey}, ptr {fptr}"))
            )
            .ok();
        }
        for (name, fn_idx) in &cls.static_methods {
            let mkey = self.string_const(name)?;
            let fptr = format!("@m_fn_{fn_idx}");
            writeln!(
                self.body,
                "  {}",
                OBJECT_SET.call(&format!("ptr {ctor}, ptr {mkey}, ptr {fptr}"))
            )
            .ok();
        }
        for (name, fv) in &cls.static_fields {
            self.emit_field_on_object(&ctor, name, fv)?;
        }
        Ok(ctor)
    }

    fn emit_field_on_object(
        &mut self,
        obj: &str,
        name: &str,
        fv: &FieldVal,
    ) -> Result<(), Diagnostic> {
        match fv {
            FieldVal::Undef => Ok(()),
            FieldVal::Number(e) => {
                let key = self.string_const(name)?;
                let n = self.emit_number_expr(e)?;
                let p = self.box_number(&n)?;
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(())
            }
            FieldVal::String(s) => {
                let key = self.string_const(name)?;
                let p = self.string_const(s)?;
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(())
            }
        }
    }

    fn emit_print_str_lit(&mut self, s: &str) -> Result<(), Diagnostic> {
        let p = self.string_const(s)?;
        writeln!(self.body, "  {}", PRINT_STR.call(&format!("ptr {p}"))).ok();
        Ok(())
    }

    /// Pack f64 bits into a non-null ptr so `0` is distinct from missing/undefined.
    fn box_number(&mut self, n: &str) -> Result<String, Diagnostic> {
        let bits = self.fresh();
        writeln!(self.body, "  {bits} = bitcast double {n} to i64").ok();
        let tagged = self.fresh();
        writeln!(self.body, "  {tagged} = or i64 {bits}, 1").ok();
        let p = self.fresh();
        writeln!(self.body, "  {p} = inttoptr i64 {tagged} to ptr").ok();
        Ok(p)
    }

    fn unbox_number(&mut self, raw: &str) -> Result<String, Diagnostic> {
        let i = self.fresh();
        writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
        let bits = self.fresh();
        writeln!(self.body, "  {bits} = and i64 {i}, -2").ok();
        let d = self.fresh();
        writeln!(self.body, "  {d} = bitcast i64 {bits} to double").ok();
        Ok(d)
    }

    fn emit_side_effect_expr(&mut self, expr: &Expr) -> Result<(), Diagnostic> {
        match expr {
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object,
                        property,
                        ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                if is_undefined_expr(value) {
                    // Leave property missing → undefined on get.
                    let _ = (obj, key);
                    return Ok(());
                }
                if let Expr::String { value: s, .. } = value.as_ref() {
                    let p = self.string_const(&s.to_string_lossy())?;
                    writeln!(
                        self.body,
                        "  {}",
                        OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                    )
                    .ok();
                    return Ok(());
                }
                let n = self.emit_number_expr(value)?;
                let p = self.box_number(&n)?;
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(())
            }
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional call not supported"));
                }
                let _ = self.emit_method_call(callee, args)?;
                Ok(())
            }
            _ => {
                let _ = self.emit_number_expr(expr)?;
                Ok(())
            }
        }
    }

    fn emit_string_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            Expr::Unary {
                op: UnaryOp::TypeOf,
                arg,
                ..
            } => self.emit_typeof(arg),
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional string member"));
                }
                // Known string static/instance fields (classify-time FieldVal::String).
                if let Some(FieldVal::String(s)) = member_field_val(
                    expr,
                    &self.info.class_of,
                    &self.info.instance_of,
                    &self.info.classes,
                ) {
                    return self.string_const(s);
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
                Ok(raw)
            }
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional call"));
                }
                self.emit_method_call_string(callee, args)
            }
            Expr::Local { id, .. } => {
                let ptr = self
                    .allocas
                    .get(id)
                    .cloned()
                    .ok_or_else(|| diag("es_classes: string local missing"))?;
                let t = self.fresh();
                writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                Ok(t)
            }
            _ => Err(diag("es_classes: unsupported string expr")),
        }
    }

    fn emit_typeof(&mut self, arg: &Expr) -> Result<String, Diagnostic> {
        match arg {
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional typeof member"));
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
                let cmp = self.fresh();
                writeln!(self.body, "  {cmp} = icmp eq ptr {raw}, null").ok();
                let s_undef = self.string_const("undefined")?;
                let s_num = self.string_const("number")?;
                let sel = self.fresh();
                writeln!(
                    self.body,
                    "  {sel} = select i1 {cmp}, ptr {s_undef}, ptr {s_num}"
                )
                .ok();
                Ok(sel)
            }
            _ => self.string_const("undefined"),
        }
    }

    fn emit_method_call_string(
        &mut self,
        callee: &Expr,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let Expr::Member {
            object,
            property,
            optional,
            ..
        } = callee
        else {
            return Err(diag("es_classes: string method callee"));
        };
        if *optional {
            return Err(diag("es_classes: optional string method"));
        }
        let obj = self.emit_object_expr(object)?;
        let key = self.member_key_cstr(property)?;
        let fptr_slot = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&fptr_slot, &format!("ptr {obj}, ptr {key}"))
        )
        .ok();
        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => return Err(diag("es_classes: spread args")),
            }
        }
        while arg_vals.len() < MAX_METHOD_ARGS {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }
        let mut call_args = format!("ptr {obj}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        let mut ty_params = String::from("ptr");
        for _ in 0..MAX_METHOD_ARGS {
            ty_params.push_str(", double");
        }
        let ret = self.fresh();
        writeln!(
            self.body,
            "  {ret} = call ptr ({ty_params}) {fptr_slot}({call_args})"
        )
        .ok();
        Ok(ret)
    }

    fn emit_number_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::Number { raw, .. } => format_number_const(raw),
            Expr::Local { id, .. } => {
                if let Some(ptr) = self.param_allocas.get(id).cloned() {
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load double, ptr {ptr}").ok();
                    return Ok(t);
                }
                let kind = *self
                    .slot_of
                    .get(id)
                    .ok_or_else(|| diag("es_classes: number local unknown"))?;
                if kind != SlotTy::Number {
                    return Err(diag("es_classes: expected number local"));
                }
                let ptr = self.number_slot_ptr(*id)?;
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
                    return Err(diag("es_classes: optional member not supported"));
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
                // null get → undefined sentinel; else unbox tagged number
                let is_null = self.fresh();
                writeln!(self.body, "  {is_null} = icmp eq ptr {raw}, null").ok();
                let und_l = format!("mget_und_{}", self.tmp);
                let num_l = format!("mget_num_{}", self.tmp + 1);
                let end_l = format!("mget_end_{}", self.tmp + 2);
                self.tmp += 3;
                let d = self.fresh();
                let slot = self.fresh();
                writeln!(self.body, "  {slot} = alloca double, align 8").ok();
                writeln!(
                    self.body,
                    "  br i1 {is_null}, label %{und_l}, label %{num_l}"
                )
                .ok();
                writeln!(self.body, "{und_l}:").ok();
                writeln!(
                    self.body,
                    "  store double {}, ptr {slot}",
                    undef_double_const()
                )
                .ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{num_l}:").ok();
                let dn = self.unbox_number(&raw)?;
                writeln!(self.body, "  store double {dn}, ptr {slot}").ok();
                writeln!(self.body, "  br label %{end_l}").ok();
                writeln!(self.body, "{end_l}:").ok();
                writeln!(self.body, "  {d} = load double, ptr {slot}").ok();
                Ok(d)
            }
            Expr::Assign {
                target:
                    AssignTarget::Member {
                        object,
                        property,
                        ..
                    },
                op: AssignOp::Eq,
                value,
                ..
            } => {
                let obj = self.emit_object_expr(object)?;
                let key = self.member_key_cstr(property)?;
                if is_undefined_expr(value) {
                    // Leave missing.
                    let _ = (obj, key);
                    return Ok(undef_double_const());
                }
                let n = self.emit_number_expr(value)?;
                let p = self.box_number(&n)?;
                writeln!(
                    self.body,
                    "  {}",
                    OBJECT_SET.call(&format!("ptr {obj}, ptr {key}, ptr {p}"))
                )
                .ok();
                Ok(n)
            }
            Expr::Binary {
                left, op, right, ..
            } => {
                let l = self.emit_number_expr(left)?;
                let r = self.emit_number_expr(right)?;
                let inst = match op {
                    BinaryOp::Add => "fadd",
                    BinaryOp::Sub => "fsub",
                    BinaryOp::Mul => "fmul",
                    BinaryOp::Div => "fdiv",
                    BinaryOp::Rem => "frem",
                    _ => return Err(diag("es_classes: unsupported binary")),
                };
                let t = self.fresh();
                writeln!(self.body, "  {t} = {inst} double {l}, {r}").ok();
                Ok(t)
            }
            Expr::Call {
                callee,
                args,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional call not supported"));
                }
                self.emit_method_call(callee, args)
            }
            _ => Err(diag("es_classes: unsupported number expr")),
        }
    }

    fn emit_method_call(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Member {
            object,
            property,
            optional,
            ..
        } = callee
        else {
            return Err(diag("es_classes: method call requires member callee"));
        };
        if *optional {
            return Err(diag("es_classes: optional member call not supported"));
        }
        if matches!(object.as_ref(), Expr::Super { .. }) {
            return self.emit_super_method_call(property, args);
        }
        let recv = self.emit_object_expr(object)?;
        let key = self.member_key_cstr(property)?;
        let fn_ptr = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&fn_ptr, &format!("ptr {recv}, ptr {key}"))
        )
        .ok();

        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_classes: spread args not supported"));
                }
            }
        }
        while arg_vals.len() < MAX_METHOD_ARGS {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }

        let mut call_args = format!("ptr {recv}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        let mut ty_params = String::from("ptr");
        for _ in 0..MAX_METHOD_ARGS {
            ty_params.push_str(", double");
        }
        let ret = self.fresh();
        writeln!(
            self.body,
            "  {ret} = call double ({ty_params}) {fn_ptr}({call_args})"
        )
        .ok();
        Ok(ret)
    }

    /// `super.m(args)` — call parent prototype method with current `this`.
    fn emit_super_method_call(
        &mut self,
        property: &Expr,
        args: &[Arg],
    ) -> Result<String, Diagnostic> {
        let name = match property {
            Expr::String { value, .. } => value.to_string_lossy(),
            _ => {
                return Err(diag(
                    "es_classes: super method name must be string key",
                ))
            }
        };
        let start = self
            .active_super_class
            .ok_or_else(|| diag("es_classes: super.m outside derived method"))?;
        let fn_idx = self
            .resolve_super_method(start, &name)
            .ok_or_else(|| diag(&format!("es_classes: super.{name} not found on parent")))?;
        let this = self
            .this_ssa
            .clone()
            .ok_or_else(|| diag("es_classes: super.m without this"))?;

        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_classes: spread super method args not supported"));
                }
            }
        }
        while arg_vals.len() < MAX_METHOD_ARGS {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }

        let mut call_args = format!("ptr {this}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        let mut ty_params = String::from("ptr");
        for _ in 0..MAX_METHOD_ARGS {
            ty_params.push_str(", double");
        }
        let ret = self.fresh();
        writeln!(
            self.body,
            "  {ret} = call double ({ty_params}) @m_fn_{fn_idx}({call_args})"
        )
        .ok();
        Ok(ret)
    }

    fn resolve_super_method(&self, mut class_idx: usize, name: &str) -> Option<usize> {
        loop {
            let cls = self.info.classes.get(class_idx)?;
            if let Some((_, fn_idx)) = cls.methods.iter().find(|(n, _)| n == name) {
                return Some(*fn_idx);
            }
            class_idx = cls.parent?;
        }
    }

    fn emit_new(&mut self, callee: &Expr, args: &[Arg]) -> Result<String, Diagnostic> {
        let Expr::Local { id, .. } = callee else {
            return Err(diag("es_classes: new callee must be local class"));
        };
        let ci = *self
            .info
            .class_of
            .get(id)
            .ok_or_else(|| diag("es_classes: unknown class constructor"))?;
        let ctor_idx = self.info.classes[ci].ctor_fn_idx;

        let ctor = {
            let ptr = self
                .allocas
                .get(id)
                .cloned()
                .ok_or_else(|| diag("es_classes: class binding missing alloca"))?;
            let t = self.fresh();
            writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
            t
        };
        let proto_key = self.string_const("prototype")?;
        let proto = self.fresh();
        writeln!(
            self.body,
            "  {}",
            OBJECT_GET.call_to(&proto, &format!("ptr {ctor}, ptr {proto_key}"))
        )
        .ok();
        let obj = self.fresh();
        writeln!(self.body, "  {}", ALLOC_OBJECT.call_to(&obj, "")).ok();
        writeln!(
            self.body,
            "  {}",
            OBJECT_SET_PROTO.call(&format!("ptr {obj}, ptr {proto}"))
        )
        .ok();

        let mut arg_vals = Vec::new();
        for a in args {
            match a {
                Arg::Expr(e) => arg_vals.push(self.emit_number_expr(e)?),
                Arg::Spread(_) => {
                    return Err(diag("es_classes: spread args not supported"));
                }
            }
        }
        while arg_vals.len() < MAX_METHOD_ARGS {
            arg_vals.push("0.00000000000000000e+00".to_string());
        }

        let mut call_args = format!("ptr {obj}");
        for v in &arg_vals {
            write!(call_args, ", double {v}").ok();
        }
        let mut ty_params = String::from("ptr");
        for _ in 0..MAX_METHOD_ARGS {
            ty_params.push_str(", double");
        }
        let ret = self.fresh();
        writeln!(
            self.body,
            "  {ret} = call double ({ty_params}) @m_fn_{ctor_idx}({call_args})"
        )
        .ok();
        let _ = ret;
        Ok(obj)
    }

    fn emit_object_expr(&mut self, expr: &Expr) -> Result<String, Diagnostic> {
        match expr {
            Expr::This { .. } => self
                .this_ssa
                .clone()
                .ok_or_else(|| diag("es_classes: This outside method")),
            Expr::New { callee, args, .. } => self.emit_new(callee, args),
            Expr::Local { id, .. } => {
                if self.slot_of.get(id) == Some(&SlotTy::Object)
                    || self.info.class_of.contains_key(id)
                {
                    let ptr = self
                        .allocas
                        .get(id)
                        .cloned()
                        .ok_or_else(|| diag("es_classes: object local unknown"))?;
                    let t = self.fresh();
                    writeln!(self.body, "  {t} = load ptr, ptr {ptr}").ok();
                    return Ok(t);
                }
                Err(diag("es_classes: object local unknown"))
            }
            Expr::Member {
                object,
                property,
                optional,
                ..
            } => {
                if *optional {
                    return Err(diag("es_classes: optional member not supported"));
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
            _ => Err(diag("es_classes: unsupported object expr")),
        }
    }

    fn member_key_cstr(&mut self, property: &Expr) -> Result<String, Diagnostic> {
        match property {
            Expr::String { value, .. } => self.string_const(&value.to_string_lossy()),
            _ => Err(diag("es_classes: member key must be string")),
        }
    }

    fn number_slot_ptr(&self, id: LocalId) -> Result<String, Diagnostic> {
        if let Some(ptr) = self.allocas.get(&id) {
            return Ok(ptr.clone());
        }
        if self.slot_of.get(&id) == Some(&SlotTy::Number) {
            return Ok(format!("@{}", number_global_name(id)));
        }
        Err(diag("es_classes: number slot missing"))
    }

    fn string_const(&mut self, s: &str) -> Result<String, Diagnostic> {
        let gname = if let Some((_, g)) = self.str_globals.iter().find(|(c, _)| c == s) {
            g.clone()
        } else {
            let g = format!(".es_cls_str.{}", self.str_n);
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


#[cfg(test)]
mod private_fields_tests {
    use super::*;
    use draconic_frontend::compile_source;

    #[test]
    fn private_fields_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/annex-b/private_fields.drac");
        let module = compile_source(src).expect("compile");
        assert!(is_es_classes_module(&module), "should classify as es_classes");
        let ir = emit_es_classes(&module).expect("emit");
        assert!(ir.contains("draconic_rt_print_f64"), "{ir}");
        assert!(ir.contains("draconic_rt_print_str"), "{ir}");
    }

    #[test]
    fn class_fields_classifies_and_emits() {
        let src = include_str!("../../../tests/conformance/fixtures/es/annex-b/class_fields.drac");
        let module = compile_source(src).expect("compile");
        assert!(
            is_es_classes_module(&module),
            "should classify as es_classes (public fields)"
        );
        let ir = emit_es_classes(&module).expect("emit");
        assert!(ir.contains("draconic_rt_print_f64"), "{ir}");
        assert!(!ir.contains("draconic_rt_hello"), "{ir}");
    }
}
