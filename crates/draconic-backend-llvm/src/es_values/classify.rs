use super::*;

pub(super) fn classify(module: &Module) -> Option<ModuleInfo> {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    let mut user_locals = Vec::new();
    let mut symbols = std::collections::HashSet::new();
    let mut objects = std::collections::HashSet::new();
    let mut undefineds = std::collections::HashSet::new();
    let mut seen = std::collections::HashSet::new();
    let mut saw_symbol = false;
    let mut needs_gc = false;

    for stmt in &module.body {
        match stmt {
            Stmt::Declare { local, init, .. } => {
                let init = init.as_ref()?;
                let loc = by_id.get(local)?;
                let slot = match &loc.ty {
                    Type::Boolean => {
                        if !expr_is_boolean(init, &by_id, &symbols) {
                            return None;
                        }
                        LocalSlot::Boolean
                    }
                    Type::String => {
                        if !expr_is_string(init, &by_id, &symbols, &undefineds) {
                            return None;
                        }
                        LocalSlot::String
                    }
                    Type::Number => {
                        if !expr_is_number(init) {
                            return None;
                        }
                        LocalSlot::Number
                    }
                    ty if is_object_ty(ty) => {
                        if !object_expr_ok(init, &by_id, &symbols) {
                            return None;
                        }
                        needs_gc = true;
                        objects.insert(*local);
                        LocalSlot::Object
                    }
                    Type::Object => {
                        if !object_expr_ok(init, &by_id, &symbols) {
                            return None;
                        }
                        needs_gc = true;
                        objects.insert(*local);
                        LocalSlot::Object
                    }
                    Type::Any => {
                        if expr_is_symbol_key_for(init, &by_id, &symbols) {
                            LocalSlot::String
                        } else if expr_is_symbol_new(init, &by_id) {
                            saw_symbol = true;
                            symbols.insert(*local);
                            LocalSlot::Symbol
                        } else if let Some(st) = member_get_ok(init, &by_id, &symbols, &objects) {
                            needs_gc = true;
                            if st == LocalSlot::Undefined {
                                undefineds.insert(*local);
                            }
                            st
                        } else {
                            return None;
                        }
                    }
                    _ => return None,
                };
                if matches!(
                    init,
                    Expr::Unary {
                        op: UnaryOp::TypeOf,
                        ..
                    }
                ) || matches!(
                    init,
                    Expr::Binary {
                        op: BinaryOp::EqEqEq | BinaryOp::NotEqEq,
                        ..
                    }
                ) || expr_is_symbol_key_for(init, &by_id, &symbols)
                    || expr_is_symbol_new(init, &by_id)
                    || matches!(init, Expr::Object { .. })
                    || matches!(init, Expr::Member { .. })
                {
                    saw_symbol = true;
                }
                if seen.insert(*local) {
                    user_locals.push((*local, slot));
                }
            }
            Stmt::Expr { expr } => {
                if !member_assign_ok(expr, &by_id, &symbols, &objects) {
                    return None;
                }
                needs_gc = true;
                saw_symbol = true;
            }
            _ => return None,
        }
    }

    if user_locals.is_empty() || !saw_symbol || symbols.is_empty() {
        return None;
    }
    Some(ModuleInfo {
        user_locals,
        symbol_locals: symbols,
        undefined_locals: undefineds,
        needs_gc,
    })
}
