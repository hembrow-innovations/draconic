//! N08.05.01–N08.05.03: native observations for ES class declarations (E05.01 /
//! `class_basic`), class heritage (E05.02 / `class_extends`), and static methods
//! (E05.03 / `class_static`).
//!
//! Classes lower to builder IIFEs (`const C = (function(){ … return ctor })()`).
//! This adapter recognizes that shape for base and derived classes (no fields /
//! private), extracts the constructor + prototype methods + static methods +
//! optional `extends` parent, and emits the Runtime GC/object ABI (`new` +
//! prototype chain + `super()` as parent-ctor call + method / static call).
//! Number locals print via `print_f64`.

use std::collections::HashMap;
use std::fmt::Write as _;

use draconic_ast::{AssignOp, BinaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, ObjectProp, Param, Pattern,
    Stmt,
};
use draconic_runtime::abi::{
    llvm_declares, ALLOC_OBJECT, GC_INIT, OBJECT_GET, OBJECT_SET, OBJECT_SET_PROTO, PRINT_F64,
};

const MAX_METHOD_ARGS: usize = 4;

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
}

#[derive(Clone)]
struct FnInfo {
    idx: usize,
    params: Vec<LocalId>,
    body: Vec<Stmt>,
    /// When set, this function is a derived constructor; `super(...)` calls this parent ctor.
    parent_ctor_fn_idx: Option<usize>,
}

#[derive(Clone)]
struct ClassInfo {
    ctor_fn_idx: usize,
    /// Prototype method name → function index.
    methods: Vec<(String, usize)>,
    /// Static method name → function index (own props on the constructor).
    static_methods: Vec<(String, usize)>,
    /// Parent class index in `ModuleInfo::classes` when `extends` is present.
    parent: Option<usize>,
}

struct ModuleInfo {
    slots: Vec<(LocalId, SlotTy)>,
    number_locals: Vec<LocalId>,
    functions: Vec<FnInfo>,
    classes: Vec<ClassInfo>,
    /// Class binding → index in `classes`.
    class_of: HashMap<LocalId, usize>,
}

fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut functions = Vec::new();
    let mut classes = Vec::new();
    let mut class_of = HashMap::new();
    let mut slots = Vec::new();
    let mut number_locals = Vec::new();
    let mut saw_class = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let init = init.as_ref()?;
                if let Some(cls) =
                    try_extract_class(init, *local, &by_id, &mut functions, &class_of, &classes)
                {
                    saw_class = true;
                    let idx = classes.len();
                    class_of.insert(*local, idx);
                    classes.push(cls);
                    slots.push((*local, SlotTy::Object));
                } else if is_object_slot(init, &class_of, &by_id) {
                    if !object_expr_ok(init, &class_of, &by_id, &functions) {
                        return None;
                    }
                    slots.push((*local, SlotTy::Object));
                } else if number_expr_ok(init, &class_of, &by_id, &functions) {
                    slots.push((*local, SlotTy::Number));
                    number_locals.push(*local);
                } else {
                    return None;
                }
            }
            Stmt::Expr { expr } => {
                if !side_effect_ok(expr, &class_of, &by_id, &functions) {
                    return None;
                }
            }
            _ => return None,
        }
    }

    if !saw_class || number_locals.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        slots,
        number_locals,
        functions,
        classes,
        class_of,
    })
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

    // Base: "use strict"; const ctor = function…; defineProperty…; methods; return.
    // Derived: super binding + heritage checks + ctor with super() + setPrototypeOf.
    let mut ctor_local: Option<LocalId> = None;
    let mut ctor_fn_idx: Option<usize> = None;
    let mut methods: Vec<(String, usize)> = Vec::new();
    let mut static_methods: Vec<(String, usize)> = Vec::new();
    let mut pending_key: Option<String> = None;
    let mut parent_idx: Option<usize> = None;
    let mut parent_ctor_fn_idx: Option<usize> = None;

    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            // Derived: `let __drac_super_N = Parent`
            Stmt::Declare {
                init: Some(Expr::Local { id, .. }),
                ..
            } if ctor_local.is_none() && parent_idx.is_none() => {
                let pidx = *class_of.get(id)?;
                parent_idx = Some(pidx);
                parent_ctor_fn_idx = Some(classes[pidx].ctor_fn_idx);
            }
            // Derived: bare `let __drac_sproto_N`
            Stmt::Declare { init: None, .. } if ctor_local.is_none() => {}
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
                let filtered = if parent_idx.is_some() {
                    filter_derived_ctor_body(cbody)
                } else {
                    filter_ctor_body(cbody)
                };
                if !method_body_ok(&filtered, by_id) {
                    return None;
                }
                let idx = functions.len();
                functions.push(FnInfo {
                    idx,
                    params: param_ids,
                    body: filtered,
                    parent_ctor_fn_idx,
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
                    // Static methods (and skip non-method own props like `name`).
                    let Arg::Expr(desc_expr) = &def_args[2] else {
                        continue;
                    };
                    let Some(method_fn) = find_method_function(desc_expr) else {
                        continue;
                    };
                    let key = pending_key.take().or_else(|| string_arg(&def_args[1]))?;
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
                    if !method_body_ok(&filtered, by_id) {
                        return None;
                    }
                    let idx = functions.len();
                    functions.push(FnInfo {
                        idx,
                        params: param_ids,
                        body: filtered,
                        parent_ctor_fn_idx: None,
                    });
                    static_methods.push((key, idx));
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
                if !method_body_ok(&filtered, by_id) {
                    return None;
                }
                let idx = functions.len();
                functions.push(FnInfo {
                    idx,
                    params: param_ids,
                    body: filtered,
                    parent_ctor_fn_idx: None,
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
        parent: parent_idx,
    })
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

fn filter_ctor_body(body: &[Stmt]) -> Vec<Stmt> {
    body.iter()
        .filter(|s| match s {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => false,
            Stmt::If { .. } => false, // new.target check
            Stmt::Expr {
                expr:
                    Expr::Assign {
                        target: AssignTarget::Member { .. },
                        op: AssignOp::Eq,
                        ..
                    },
            } => true,
            Stmt::Return { .. } => true,
            Stmt::Block { .. } => true,
            _ => false,
        })
        .cloned()
        .collect()
}

/// Collapse derived-ctor IR (this-TDZ + Reflect.construct super IIFE) into:
/// `super(args…); this.prop = …;`
fn filter_derived_ctor_body(body: &[Stmt]) -> Vec<Stmt> {
    let mut out = Vec::new();
    collect_derived_ctor_stmts(body, &mut out);
    out
}

fn collect_derived_ctor_stmts(body: &[Stmt], out: &mut Vec<Stmt>) {
    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            Stmt::If { .. } | Stmt::Declare { .. } | Stmt::Return { .. } => {}
            Stmt::Labeled { body, .. } => collect_derived_ctor_stmts_one(body, out),
            Stmt::Block { body } => collect_derived_ctor_stmts(body, out),
            other => collect_derived_ctor_stmts_one(other, out),
        }
    }
}

fn collect_derived_ctor_stmts_one(stmt: &Stmt, out: &mut Vec<Stmt>) {
    match stmt {
        Stmt::Block { body } => collect_derived_ctor_stmts(body, out),
        Stmt::Labeled { body, .. } => collect_derived_ctor_stmts_one(body, out),
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
        }
        Stmt::Expr {
            expr:
                Expr::Assign {
                    target:
                        AssignTarget::Member {
                            object: _,
                            property,
                            computed,
                        },
                    op: AssignOp::Eq,
                    value,
                    ty,
                },
        } if matches!(property.as_ref(), Expr::String { .. }) => {
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
        _ => {}
    }
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
        Stmt::Return { value: Some(e) } => number_expr_ok_method(e, by_id),
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
                && number_expr_ok_method(value, by_id)
        }
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
                Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions),
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
                && object_expr_ok(object, class_of, by_id, functions)
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
                && object_expr_ok(object, class_of, by_id, functions)
                && matches!(property.as_ref(), Expr::String { .. })
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && method_callee_ok(callee, class_of, by_id, functions)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions),
                    Arg::Spread(_) => false,
                })
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            matches!(
                op,
                BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem
            ) && number_expr_ok(left, class_of, by_id, functions)
                && number_expr_ok(right, class_of, by_id, functions)
        }
        Expr::New { .. } => false,
        _ => false,
    }
}

fn method_callee_ok(
    callee: &Expr,
    class_of: &HashMap<LocalId, usize>,
    by_id: &HashMap<LocalId, &Local>,
    functions: &[FnInfo],
) -> bool {
    match callee {
        Expr::Member {
            object,
            property,
            optional,
            ..
        } => {
            !*optional
                && object_expr_ok(object, class_of, by_id, functions)
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
) -> bool {
    match expr {
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            !*optional
                && method_callee_ok(callee, class_of, by_id, functions)
                && args.iter().all(|a| match a {
                    Arg::Expr(e) => number_expr_ok(e, class_of, by_id, functions),
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
            "; Draconic LLVM backend (N08.05 ES class decl/heritage/static via Runtime ABI)"
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
            ])
        )
        .ok();
        writeln!(self.out).ok();

        for (id, kind) in &info.slots {
            if *kind == SlotTy::Number {
                let g = number_global_name(*id);
                writeln!(
                    self.out,
                    "@{g} = internal global double 0.00000000000000000e+00, align 8"
                )
                .ok();
                self.allocas.insert(*id, format!("@{g}"));
            }
        }
        if info.slots.iter().any(|(_, k)| *k == SlotTy::Number) {
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

        for id in &info.number_locals {
            let ptr = self.number_slot_ptr(*id)?;
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

    fn emit_method_fn(&mut self, f: &FnInfo) -> Result<(), Diagnostic> {
        let name = format!("m_fn_{}", f.idx);
        let mut params_s = String::from("ptr %this");
        for i in 0..MAX_METHOD_ARGS {
            write!(params_s, ", double %a{i}").ok();
        }
        writeln!(self.out, "define double @{name}({params_s}) {{").ok();
        writeln!(self.out, "entry:").ok();

        let saved_body = std::mem::take(&mut self.body);
        let saved_this = self.this_ssa.take();
        let saved_params = std::mem::take(&mut self.param_allocas);
        let saved_allocas = std::mem::take(&mut self.allocas);
        let saved_parent = self.active_parent_ctor.take();

        self.this_ssa = Some("%this".to_string());
        self.active_parent_ctor = f.parent_ctor_fn_idx;
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
            writeln!(
                self.body,
                "  ret double 0.00000000000000000e+00"
            )
            .ok();
        }

        self.out.push_str(&self.body);
        writeln!(self.out, "}}").ok();
        writeln!(self.out).ok();

        self.body = saved_body;
        self.this_ssa = saved_this;
        self.param_allocas = saved_params;
        self.allocas = saved_allocas;
        self.active_parent_ctor = saved_parent;
        Ok(())
    }

    fn emit_method_stmt(&mut self, stmt: &Stmt) -> Result<(), Diagnostic> {
        match stmt {
            Stmt::Return { value: Some(e) } => {
                let v = self.emit_number_expr(e)?;
                writeln!(self.body, "  ret double {v}").ok();
                Ok(())
            }
            Stmt::Return { value: None } => {
                writeln!(
                    self.body,
                    "  ret double 0.00000000000000000e+00"
                )
                .ok();
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
                let kind = *self
                    .slot_of
                    .get(local)
                    .ok_or_else(|| diag("es_classes: declare unknown slot"))?;
                match kind {
                    SlotTy::Number => {
                        let v = self.emit_number_expr(init)?;
                        let ptr = self.number_slot_ptr(*local)?;
                        writeln!(self.body, "  store double {v}, ptr {ptr}").ok();
                    }
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
        Ok(ctor)
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
                let n = self.emit_number_expr(value)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                let p = self.fresh();
                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
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
                let i = self.fresh();
                writeln!(self.body, "  {i} = ptrtoint ptr {raw} to i64").ok();
                let d = self.fresh();
                writeln!(self.body, "  {d} = sitofp i64 {i} to double").ok();
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
                let n = self.emit_number_expr(value)?;
                let i = self.fresh();
                writeln!(self.body, "  {i} = fptosi double {n} to i64").ok();
                let p = self.fresh();
                writeln!(self.body, "  {p} = inttoptr i64 {i} to ptr").ok();
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

