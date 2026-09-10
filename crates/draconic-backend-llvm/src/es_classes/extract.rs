use std::collections::HashMap;

use draconic_ast::AssignOp;
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, ObjectProp, ObjectPropKey, Stmt,
};

use super::ctor::{
    descriptor_direct_method_fn, filter_ctor_body, filter_derived_ctor_body, filter_method_body,
    simple_param_ids, static_field_val_from_desc,
};
use super::ok::{method_body_ok, method_ret_kind};
use super::private::{
    is_ident_name, is_weakmap_local, private_field_name_from_wm, rewrite_private_stmts,
};
use super::{
    find_method_function, is_define_on_ctor, is_define_on_proto, is_object_define_property,
    is_object_set_prototype_of, string_arg, ClassInfo, FieldVal, FnInfo, LocalSlot, MethodRet,
};

pub(super) fn try_extract_class(
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
    let mut class_name: Option<String> = None;

    for stmt in body {
        match stmt {
            Stmt::Expr {
                expr: Expr::String { value, .. },
            } if value.to_string_lossy() == "use strict" => {}
            // Private instance field storage: `let __drac_pf_N_name = new WeakMap()`
            Stmt::Declare {
                local,
                init: Some(Expr::New { callee, args, .. }),
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
                init:
                    Some(Expr::Function {
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
                expr:
                    Expr::Call {
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
                    if key == "name" {
                        if let Some(n) = descriptor_string_value(desc_expr) {
                            class_name = Some(n);
                        }
                        continue;
                    }
                    if key.is_empty() || key == "prototype" {
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
                        let mut rewritten = rewrite_private_stmts(&filtered, &wm_fields);
                        if let (Some(cl), Some(n)) = (ctor_local, class_name.as_ref()) {
                            rewritten = rewrite_ctor_name_refs(&rewritten, cl, n);
                        }
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
                let mut rewritten = rewrite_private_stmts(&filtered, &wm_fields);
                if let (Some(cl), Some(n)) = (ctor_local, class_name.as_ref()) {
                    rewritten = rewrite_ctor_name_refs(&rewritten, cl, n);
                }
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
                expr:
                    Expr::Call {
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

/// `Object.defineProperty(ctor, "name", { value: "Counter", … })` → `"Counter"`.
fn descriptor_string_value(desc: &Expr) -> Option<String> {
    let Expr::Object { properties, .. } = desc else {
        return None;
    };
    for p in properties {
        let ObjectProp::Property { key, value } = p else {
            continue;
        };
        let k = match key {
            ObjectPropKey::Static(s) => s.to_string_lossy(),
            ObjectPropKey::Computed(Expr::String { value, .. }) => value.to_string_lossy(),
            _ => continue,
        };
        if k == "value" {
            if let Expr::String { value, .. } = value {
                return Some(value.to_string_lossy());
            }
        }
    }
    None
}

/// Named class expression methods may close over the ctor local and read `.name`
/// (`return Counter.name`). Fold to the string set on the constructor.
fn rewrite_ctor_name_refs(body: &[Stmt], ctor: LocalId, name: &str) -> Vec<Stmt> {
    body.iter()
        .map(|s| rewrite_ctor_name_stmt(s, ctor, name))
        .collect()
}

fn rewrite_ctor_name_stmt(stmt: &Stmt, ctor: LocalId, name: &str) -> Stmt {
    match stmt {
        Stmt::Return { value: Some(e) } => Stmt::Return {
            value: Some(rewrite_ctor_name_expr(e, ctor, name)),
        },
        Stmt::Block { body } => Stmt::Block {
            body: body
                .iter()
                .map(|s| rewrite_ctor_name_stmt(s, ctor, name))
                .collect(),
        },
        Stmt::Expr { expr } => Stmt::Expr {
            expr: rewrite_ctor_name_expr(expr, ctor, name),
        },
        other => other.clone(),
    }
}

fn rewrite_ctor_name_expr(expr: &Expr, ctor: LocalId, name: &str) -> Expr {
    match expr {
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ty,
        } if !*optional
            && matches!(object.as_ref(), Expr::Local { id, .. } if *id == ctor)
            && matches!(
                property.as_ref(),
                Expr::String { value, .. } if value.to_string_lossy() == "name"
            ) =>
        {
            Expr::String {
                value: name.into(),
                ty: Type::String,
            }
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => Expr::Binary {
            left: Box::new(rewrite_ctor_name_expr(left, ctor, name)),
            op: *op,
            right: Box::new(rewrite_ctor_name_expr(right, ctor, name)),
            ty: *ty,
        },
        Expr::Call {
            callee,
            args,
            optional,
            ty,
        } => Expr::Call {
            callee: Box::new(rewrite_ctor_name_expr(callee, ctor, name)),
            args: args
                .iter()
                .map(|a| match a {
                    Arg::Expr(e) => Arg::Expr(rewrite_ctor_name_expr(e, ctor, name)),
                    Arg::Spread(e) => Arg::Spread(rewrite_ctor_name_expr(e, ctor, name)),
                })
                .collect(),
            optional: *optional,
            ty: *ty,
        },
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ty,
        } => Expr::Member {
            object: Box::new(rewrite_ctor_name_expr(object, ctor, name)),
            property: Box::new(rewrite_ctor_name_expr(property, ctor, name)),
            computed: *computed,
            optional: *optional,
            ty: *ty,
        },
        other => other.clone(),
    }
}

/// `let i = new (class { constructor() { this.x = 42 } })().x` → number observation.
pub(super) fn try_fold_new_class_iife_member(
    init: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    functions: &mut Vec<FnInfo>,
    class_of: &HashMap<LocalId, usize>,
    classes: &[ClassInfo],
) -> Option<(LocalSlot, Option<String>)> {
    let Expr::Member {
        object,
        property,
        optional,
        ..
    } = init
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
    let Expr::New { callee, args, .. } = object.as_ref() else {
        return None;
    };
    // Only empty-arg `new (class IIFE)()` for constant fold of ctor assigns.
    if !args.is_empty() {
        return None;
    }
    // Callee must be class builder IIFE call (not a binding).
    if !matches!(callee.as_ref(), Expr::Call { .. }) {
        return None;
    }
    let dummy = LocalId(u32::MAX);
    let cls = try_extract_class(callee, dummy, by_id, functions, class_of, classes)?;
    let ctor = functions.get(cls.ctor_fn_idx)?;
    let raw = ctor_this_prop_number(&ctor.body, &key)?;
    // Drop the extracted class methods from functions — observation is folded;
    // keeping orphan m_fn_* is fine but wastes IR. Leave them; emit still works.
    let _ = cls;
    Some((LocalSlot::Number, Some(raw)))
}

fn ctor_this_prop_number(body: &[Stmt], key: &str) -> Option<String> {
    for s in body {
        match s {
            Stmt::Expr {
                expr:
                    Expr::Assign {
                        target:
                            AssignTarget::Member {
                                object, property, ..
                            },
                        op: AssignOp::Eq,
                        value,
                        ..
                    },
            } if matches!(object.as_ref(), Expr::This { .. })
                && matches!(
                    property.as_ref(),
                    Expr::String { value: k, .. } if k.to_string_lossy() == key
                ) =>
            {
                if let Expr::Number { raw, .. } = value.as_ref() {
                    return Some(raw.clone());
                }
            }
            Stmt::Block { body } => {
                if let Some(r) = ctor_this_prop_number(body, key) {
                    return Some(r);
                }
            }
            _ => {}
        }
    }
    None
}
