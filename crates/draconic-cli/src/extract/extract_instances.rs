use std::collections::HashSet;

use draconic_ast::{ArrayPatternElement, BindingPattern, Expr, ObjectPatternProp, Param, Stmt};

use super::unwrap_parens;

pub(super) fn delete_params(params: &[Param], names: &mut HashSet<String>) {
    for param in params {
        delete_bound_names(&param.binding, names);
    }
}

pub(super) fn delete_bound_names(binding: &BindingPattern, names: &mut HashSet<String>) {
    binding.for_each_ident(&mut |id| {
        names.remove(&id.name);
    });
}

pub(super) fn seed_import_locals(stmt: &Stmt, names: &mut HashSet<String>) {
    let Stmt::ImportDeclaration {
        specifiers,
        namespace,
        type_only,
        ..
    } = stmt
    else {
        return;
    };
    if *type_only {
        return;
    }
    for spec in specifiers {
        if spec.is_type {
            continue;
        }
        names.insert(spec.local.name.clone());
    }
    if let Some(ns) = namespace {
        names.insert(ns.name.clone());
    }
}

pub(super) fn take_let_binding(
    binding: &BindingPattern,
    init: Option<&Expr>,
    names: &mut HashSet<String>,
) {
    match binding {
        BindingPattern::Ident(id) => apply_instance_binding(&id.name, init, names),
        other => delete_bound_names(other, names),
    }
}

pub(super) fn apply_instance_binding(
    name: &str,
    value: Option<&Expr>,
    names: &mut HashSet<String>,
) {
    let Some(value) = value else {
        names.remove(name);
        return;
    };
    match unwrap_parens(value) {
        Expr::New { callee, .. } => {
            if ctor_unwraps_to_ident_or_member(callee) {
                names.insert(name.to_string());
            } else {
                names.remove(name);
            }
        }
        Expr::Ident(id) if names.contains(&id.name) => {
            names.insert(name.to_string());
        }
        _ => {
            names.remove(name);
        }
    }
}

fn ctor_unwraps_to_ident_or_member(callee: &Expr) -> bool {
    match unwrap_parens(callee) {
        Expr::Ident(_) => true,
        Expr::MemberExpression {
            computed, private, ..
        } if !*computed && !*private => true,
        _ => false,
    }
}

pub(super) fn take_assign_target(target: &Expr, value: &Expr, names: &mut HashSet<String>) {
    match unwrap_parens(target) {
        Expr::Ident(id) => apply_instance_binding(&id.name, Some(value), names),
        Expr::ArrayPattern { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, .. }
                    | ArrayPatternElement::Rest(binding) => {
                        delete_bound_names(binding, names);
                    }
                }
            }
        }
        Expr::ObjectPattern { properties, .. } => {
            for prop in properties {
                match prop {
                    ObjectPatternProp::Prop { binding, .. } | ObjectPatternProp::Rest(binding) => {
                        delete_bound_names(binding, names);
                    }
                }
            }
        }
        _ => {}
    }
}

pub(super) fn clear_for_binding(stmt: &Stmt, names: &mut HashSet<String>) {
    match stmt {
        Stmt::Let { binding, .. } => delete_bound_names(binding, names),
        Stmt::Expression { expr, .. } => match unwrap_parens(expr) {
            Expr::Ident(id) => {
                names.remove(&id.name);
            }
            Expr::ArrayPattern { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Elision => {}
                        ArrayPatternElement::Pattern { binding, .. }
                        | ArrayPatternElement::Rest(binding) => {
                            delete_bound_names(binding, names);
                        }
                    }
                }
            }
            Expr::ObjectPattern { properties, .. } => {
                for prop in properties {
                    match prop {
                        ObjectPatternProp::Prop { binding, .. }
                        | ObjectPatternProp::Rest(binding) => {
                            delete_bound_names(binding, names);
                        }
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
}
