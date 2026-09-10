use std::collections::HashMap;

use draconic_ast::{AssignOp, BinaryOp, UnaryOp};
use draconic_ir::{
    Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, ObjectProp, ObjectPropKey, Param,
    Pattern, Stmt,
};

use super::{
    find_method_function, is_object_define_property, string_arg, FieldVal, MAX_METHOD_ARGS,
};

pub(super) fn filter_ctor_body(body: &[Stmt]) -> (Vec<Stmt>, Vec<(String, FieldVal)>) {
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
                } else if matches!(
                    expr,
                    Expr::Call { .. }
                        | Expr::Binary {
                            op: BinaryOp::Comma,
                            ..
                        }
                ) {
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
pub(super) fn filter_derived_ctor_body(body: &[Stmt]) -> (Vec<Stmt>, Vec<(String, FieldVal)>) {
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
                    ty: *ty,
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
                            ty: *ty,
                        },
                    });
                }
            } else if matches!(
                expr,
                Expr::Call { .. }
                    | Expr::Binary {
                        op: BinaryOp::Comma,
                        ..
                    }
            ) {
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

pub(super) fn static_field_val_from_desc(desc: &Expr) -> Option<FieldVal> {
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
            left, op, right, ..
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

pub(super) fn descriptor_direct_method_fn(desc: &Expr) -> Option<&Expr> {
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

pub(super) fn filter_method_body(body: &[Stmt]) -> Vec<Stmt> {
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

pub(super) fn simple_param_ids(
    params: &[Param],
    by_id: &HashMap<LocalId, &Local>,
) -> Option<Vec<LocalId>> {
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

#[cfg(test)]
mod class_fields_tests {
    use super::super::walk_es_classes;
    use draconic_frontend::compile_source;

    #[test]
    fn class_fields_classifies_and_emits() {
        let src =
            include_str!("../../../../tests/conformance/fixtures/es/annex-b/class_fields.drac");
        let module = compile_source(src).expect("compile");
        let ir = walk_es_classes(&module)
            .expect("should classify as es_classes (public fields)")
            .expect("emit");
        assert!(ir.contains("draconic_rt_print_f64"), "{ir}");
        assert!(!ir.contains("draconic_rt_hello"), "{ir}");
    }
}
