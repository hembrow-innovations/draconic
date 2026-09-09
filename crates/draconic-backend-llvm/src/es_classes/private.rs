use std::collections::HashMap;

use draconic_ast::{AssignOp, BinaryOp};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, ObjectProp, ObjectPropKey, Pattern,
    Stmt,
};

fn is_wm_method_call(
    callee: &Expr,
    args: &[Arg],
    method: &str,
    obj_param: LocalId,
    val_param: Option<LocalId>,
) -> bool {
    let Expr::Member {
        object, property, ..
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

pub(super) fn is_ident_name(expr: &Expr, name: &str) -> bool {
    matches!(expr, Expr::IdentName { name: n, .. } if n == name)
}

pub(super) fn is_weakmap_local(id: LocalId, by_id: &HashMap<LocalId, &Local>) -> bool {
    by_id
        .get(&id)
        .is_some_and(|l| l.name.starts_with("__drac_pf_"))
}

pub(super) fn private_field_name_from_wm(local_name: &str) -> Option<String> {
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

pub(super) fn rewrite_private_stmts(body: &[Stmt], wm_fields: &HashMap<LocalId, String>) -> Vec<Stmt> {
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
            arg: Box::new(
                rewrite_private_expr(arg, wm_fields).unwrap_or_else(|| arg.as_ref().clone()),
            ),
            ty: *ty,
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
                    rewrite_private_expr(right, wm_fields)
                        .unwrap_or_else(|| right.as_ref().clone()),
                ),
                ty: *ty,
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
                    Arg::Expr(e) => {
                        Arg::Expr(rewrite_private_expr(e, wm_fields).unwrap_or_else(|| e.clone()))
                    }
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
                ty: *ty,
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
            ty: *ty,
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
            ty: *ty,
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
        } => {
            find_wm_get_in_expr(consequent, param).or_else(|| find_wm_get_in_expr(alternate, param))
        }
        Expr::Call { callee, args, .. } if is_wm_method_call(callee, args, "get", param, None) => {
            wm_id_from_callee(callee)
        }
        Expr::Call { callee, args, .. } => find_wm_get_in_expr(callee, param).or_else(|| {
            args.iter().find_map(|a| match a {
                Arg::Expr(e) => find_wm_get_in_expr(e, param),
                _ => None,
            })
        }),
        Expr::Binary { left, right, .. } => {
            find_wm_get_in_expr(left, param).or_else(|| find_wm_get_in_expr(right, param))
        }
        _ => None,
    }
}

fn match_private_set_stmt_expr(expr: &Expr, wm_fields: &HashMap<LocalId, String>) -> Option<Expr> {
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
            AssignTarget::Local(_) if matches!(v, Expr::This { .. }) => {
                Some(Expr::This { ty: Type::Any })
            }
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
        } => {
            find_wm_set_in_expr(consequent, param).or_else(|| find_wm_set_in_expr(alternate, param))
        }
        Expr::Binary {
            op: BinaryOp::Comma,
            left,
            right,
            ..
        } => find_wm_set_in_expr(left, param).or_else(|| find_wm_set_in_expr(right, param)),
        Expr::Call { callee, args, .. }
            if is_wm_method_call(callee, args, "set", param, None) && args.len() == 2 =>
        {
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
    let Expr::Call { callee, args, .. } = expr else {
        // recurse
        return match expr {
            Expr::Call { callee, args, .. } => {
                find_field_init_set(callee, wm_fields).or_else(|| {
                    args.iter().find_map(|a| match a {
                        Arg::Expr(e) => find_field_init_set(e, wm_fields),
                        _ => None,
                    })
                })
            }
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
        Expr::Binary { left, right, .. } => find_wm_set_param(left, obj_param, val_param)
            .or_else(|| find_wm_set_param(right, obj_param, val_param)),
        Expr::Unary { arg, .. } => find_wm_set_param(arg, obj_param, val_param),
        Expr::Call { callee, args, .. }
            if is_wm_method_call(callee, args, "set", obj_param, Some(val_param)) =>
        {
            wm_id_from_callee(callee)
        }
        Expr::Call { callee, args, .. } => {
            find_wm_set_param(callee, obj_param, val_param).or_else(|| {
                args.iter().find_map(|a| match a {
                    Arg::Expr(e) => find_wm_set_param(e, obj_param, val_param),
                    _ => None,
                })
            })
        }
        _ => None,
    }
}

#[cfg(test)]
mod private_fields_tests {
    use super::super::walk_es_classes;
    use draconic_frontend::compile_source;

    #[test]
    fn private_fields_classifies_and_emits() {
        let src =
            include_str!("../../../../tests/conformance/fixtures/es/annex-b/private_fields.drac");
        let module = compile_source(src).expect("compile");
        let ir = walk_es_classes(&module)
            .expect("should classify as es_classes")
            .expect("emit");
        assert!(ir.contains("draconic_rt_print_f64"), "{ir}");
        assert!(ir.contains("draconic_rt_print_str"), "{ir}");
    }
}
