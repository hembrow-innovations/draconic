use super::classify::*;
use super::*;

pub(super) fn cmp_number_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Binary {
            left,
            op: BinaryOp::EqEqEq | BinaryOp::EqEq | BinaryOp::NotEqEq | BinaryOp::NotEq,
            right,
            ..
        } => number_expr_ok(left, by_id, slot_of) && number_expr_ok(right, by_id, slot_of),
        _ => false,
    }
}

pub(super) fn local_assign_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    let Expr::Assign {
        target: AssignTarget::Local(id),
        op: AssignOp::Eq,
        value,
        ..
    } = expr
    else {
        return false;
    };
    match slot_of.get(id) {
        Some(SlotTy::Number) => number_expr_ok(value, by_id, slot_of),
        Some(SlotTy::String) => string_expr_ok(value, by_id, slot_of),
        Some(SlotTy::Array) => array_expr_ok(value, by_id, slot_of),
        _ => false,
    }
}

pub(super) fn const_index(expr: &Expr) -> Option<usize> {
    match expr {
        Expr::Number { raw, .. } => {
            let cleaned: String = raw.chars().filter(|c| *c != '_').collect();
            let f: f64 = cleaned.parse().ok()?;
            if f.is_finite() && f >= 0.0 && f.fract() == 0.0 {
                Some(f as usize)
            } else {
                None
            }
        }
        _ => None,
    }
}

pub(super) fn infer_expr_slot(
    expr: &Expr,
    arr_inits: &HashMap<LocalId, Expr>,
    arr_elem: &HashMap<LocalId, ElemKind>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<SlotTy> {
    match expr {
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Boolean { .. } => Some(SlotTy::Bool),
        Expr::Null { .. } => Some(SlotTy::Null),
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            if *optional {
                return None;
            }
            if !*computed && member_key_is_length(property) {
                return Some(SlotTy::Number);
            }
            // obj.prop / obj["k"] — number observations on object props.
            if object_expr_ok(object, &HashMap::new(), slot_of) {
                let string_key = if *computed {
                    matches!(property.as_ref(), Expr::String { .. })
                } else {
                    matches!(property.as_ref(), Expr::String { .. })
                };
                if string_key {
                    return Some(SlotTy::Number);
                }
            }
            if *computed {
                if let Some(idx) = const_index(property) {
                    if let Some(elem) = resolve_array_elem(object, idx, arr_inits) {
                        return literal_or_array_slot(&elem);
                    }
                }
                // obj.arr[i] — array property then index.
                if array_expr_ok(object, &HashMap::new(), slot_of) {
                    return slot_from_elem_kind(
                        array_expr_elem_kind(object, arr_inits, arr_elem, slot_of)
                            .unwrap_or(ElemKind::Number),
                    )
                    .or(Some(SlotTy::Number));
                }
                return slot_from_elem_kind(array_expr_elem_kind(
                    object, arr_inits, arr_elem, slot_of,
                )?);
            }
            None
        }
        _ => None,
    }
}

pub(super) fn slot_from_elem_kind(k: ElemKind) -> Option<SlotTy> {
    match k {
        ElemKind::Number => Some(SlotTy::Number),
        ElemKind::String => Some(SlotTy::String),
        ElemKind::Array => Some(SlotTy::Array),
        ElemKind::Unknown => None,
    }
}

pub(super) fn resolve_array_elem(
    array_expr: &Expr,
    idx: usize,
    arr_inits: &HashMap<LocalId, Expr>,
) -> Option<Expr> {
    let lit = match array_expr {
        Expr::Array { .. } => array_expr.clone(),
        Expr::Local { id, .. } => arr_inits.get(id).cloned()?,
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            if *optional || !*computed {
                return None;
            }
            let outer_idx = const_index(property)?;

            resolve_array_elem(object, outer_idx, arr_inits)?
        }
        _ => return None,
    };
    let Expr::Array { elements, .. } = lit else {
        return None;
    };
    match elements.get(idx)? {
        ArrayElement::Expr(e) => Some(e.clone()),
        ArrayElement::Elision => Some(Expr::Null { ty: Type::Null }),
        ArrayElement::Spread(_) => None,
    }
}

pub(super) fn literal_or_array_slot(expr: &Expr) -> Option<SlotTy> {
    match expr {
        Expr::Number { .. } => Some(SlotTy::Number),
        Expr::String { .. } => Some(SlotTy::String),
        Expr::Boolean { .. } => Some(SlotTy::Bool),
        Expr::Null { .. } => Some(SlotTy::Null),
        Expr::Array { .. } => Some(SlotTy::Array),
        _ => None,
    }
}

pub(super) fn array_expr_elem_kind(
    expr: &Expr,
    arr_inits: &HashMap<LocalId, Expr>,
    arr_elem: &HashMap<LocalId, ElemKind>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<ElemKind> {
    match expr {
        Expr::Array { elements, .. } => array_lit_elem_kind(elements, arr_inits, arr_elem, slot_of),
        Expr::Local { id, .. } => arr_elem.get(id).copied().or_else(|| {
            arr_inits
                .get(id)
                .and_then(|e| array_expr_elem_kind(e, arr_inits, arr_elem, slot_of))
        }),
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            if *optional || !*computed {
                return None;
            }
            let idx = const_index(property)?;
            let elem = resolve_array_elem(object, idx, arr_inits)?;
            match elem {
                Expr::Array { elements, .. } => {
                    array_lit_elem_kind(&elements, arr_inits, arr_elem, slot_of)
                }
                Expr::String { .. } => Some(ElemKind::String),
                Expr::Number { .. } => Some(ElemKind::Number),
                _ => None,
            }
        }
        _ => None,
    }
}

pub(super) fn array_lit_elem_kind(
    elements: &[ArrayElement],
    arr_inits: &HashMap<LocalId, Expr>,
    arr_elem: &HashMap<LocalId, ElemKind>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<ElemKind> {
    let mut kind: Option<ElemKind> = None;
    for el in elements {
        let k = match el {
            ArrayElement::Elision => continue,
            ArrayElement::Expr(e) => expr_as_elem_kind(e, arr_inits, arr_elem, slot_of)?,
            ArrayElement::Spread(e) => spread_source_elem_kind(e, arr_inits, arr_elem, slot_of)?,
        };
        kind = Some(match kind {
            None => k,
            Some(prev) if prev == k => prev,
            Some(_) => ElemKind::Unknown,
        });
    }
    Some(kind.unwrap_or(ElemKind::Unknown))
}

pub(super) fn expr_as_elem_kind(
    expr: &Expr,
    arr_inits: &HashMap<LocalId, Expr>,
    arr_elem: &HashMap<LocalId, ElemKind>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<ElemKind> {
    match expr {
        Expr::Number { .. } => Some(ElemKind::Number),
        Expr::String { .. } => Some(ElemKind::String),
        Expr::Array { .. } => Some(ElemKind::Array),
        _ if is_undefined_expr(expr) || matches!(expr, Expr::Null { .. }) => {
            // Holes / undefined do not pin element kind.
            Some(ElemKind::Unknown)
        }
        Expr::Local { id, .. } => match slot_of.get(id) {
            Some(SlotTy::Number) => Some(ElemKind::Number),
            Some(SlotTy::String) => Some(ElemKind::String),
            Some(SlotTy::Array) => Some(ElemKind::Array),
            // Global `undefined` binding — hole-like.
            None => Some(ElemKind::Unknown),
            _ => None,
        },
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            if *optional {
                return None;
            }
            if !*computed && member_key_is_length(property) {
                return Some(ElemKind::Number);
            }
            if *computed {
                if let Some(idx) = const_index(property) {
                    if let Some(elem) = resolve_array_elem(object, idx, arr_inits) {
                        return expr_as_elem_kind(&elem, arr_inits, arr_elem, slot_of);
                    }
                }
                return array_expr_elem_kind(object, arr_inits, arr_elem, slot_of);
            }
            None
        }
        _ => None,
    }
}

pub(super) fn spread_source_elem_kind(
    expr: &Expr,
    arr_inits: &HashMap<LocalId, Expr>,
    arr_elem: &HashMap<LocalId, ElemKind>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> Option<ElemKind> {
    match expr {
        Expr::String { .. } => Some(ElemKind::String),
        Expr::Local { id, .. } => match slot_of.get(id) {
            Some(SlotTy::String) => Some(ElemKind::String),
            Some(SlotTy::Array) => arr_elem.get(id).copied().or_else(|| {
                arr_inits
                    .get(id)
                    .and_then(|e| array_expr_elem_kind(e, arr_inits, arr_elem, slot_of))
            }),
            _ => None,
        },
        Expr::Array { elements, .. } => array_lit_elem_kind(elements, arr_inits, arr_elem, slot_of),
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            if *optional || !*computed {
                return None;
            }
            let idx = const_index(property)?;
            let elem = resolve_array_elem(object, idx, arr_inits)?;
            match elem {
                Expr::Array { elements, .. } => {
                    array_lit_elem_kind(&elements, arr_inits, arr_elem, slot_of)
                }
                Expr::String { .. } => Some(ElemKind::String),
                _ => None,
            }
        }
        _ => None,
    }
}

pub(super) fn is_array_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Object | Type::Any)
}

pub(super) fn is_number_slot_ty(ty: &Type) -> bool {
    matches!(ty, Type::Number | Type::Any)
}

pub(super) fn array_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Array { elements, .. } => elements.iter().all(|el| match el {
            ArrayElement::Expr(e) => value_expr_ok(e, by_id, slot_of),
            ArrayElement::Elision => true,
            ArrayElement::Spread(e) => {
                array_expr_ok(e, by_id, slot_of) || string_expr_ok(e, by_id, slot_of)
            }
        }),
        Expr::Local { id, ty } => {
            slot_of.get(id) == Some(&SlotTy::Array)
                || is_array_slot_ty(ty)
                || by_id
                    .get(id)
                    .is_some_and(|l| is_array_slot_ty(&l.ty) || matches!(l.ty, Type::Any))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            if *optional {
                return false;
            }
            // obj.arr property holding an array (rest member target read-back).
            if object_expr_ok(object, by_id, slot_of) {
                return if *computed {
                    string_expr_ok(property, by_id, slot_of)
                        || matches!(property.as_ref(), Expr::String { .. })
                } else {
                    matches!(property.as_ref(), Expr::String { .. })
                };
            }
            array_expr_ok(object, by_id, slot_of)
                && if *computed {
                    number_expr_ok(property, by_id, slot_of)
                } else {
                    member_key_is_length(property)
                }
        }
        _ => false,
    }
}

pub(super) fn value_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    number_expr_ok(expr, by_id, slot_of)
        || string_expr_ok(expr, by_id, slot_of)
        || bool_expr_ok(expr, by_id, slot_of)
        || null_expr_ok(expr, by_id, slot_of)
        || array_expr_ok(expr, by_id, slot_of)
        || object_expr_ok(expr, by_id, slot_of)
        || is_undefined_expr(expr)
}

pub(super) fn number_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Number { .. } => true,
        Expr::Local { id, ty } => {
            slot_of.get(id) == Some(&SlotTy::Number)
                || is_number_slot_ty(ty)
                || by_id
                    .get(id)
                    .is_some_and(|l| matches!(l.ty, Type::Number | Type::Any))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            if *optional {
                return false;
            }
            // obj.prop / obj["k"] number property (string key only — not arr index).
            if object_expr_ok(object, by_id, slot_of) {
                let string_key = if *computed {
                    string_expr_ok(property, by_id, slot_of)
                        || matches!(property.as_ref(), Expr::String { .. })
                } else {
                    matches!(property.as_ref(), Expr::String { .. })
                };
                if string_key {
                    return true;
                }
            }
            // arr[i] / obj.arr[i] — computed number index on array-valued base.
            array_expr_ok(object, by_id, slot_of)
                && if *computed {
                    number_expr_ok(property, by_id, slot_of)
                } else {
                    member_key_is_length(property)
                }
        }
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    computed: true,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            array_expr_ok(object, by_id, slot_of)
                && number_expr_ok(property, by_id, slot_of)
                && number_expr_ok(value, by_id, slot_of)
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem,
            right,
            ..
        } => number_expr_ok(left, by_id, slot_of) && number_expr_ok(right, by_id, slot_of),
        _ => false,
    }
}

/// `a[i] = v` / `nested[0][0] = v` as a statement expression (N08.06.02).
pub(super) fn member_assign_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Assign {
            target:
                AssignTarget::Member {
                    object,
                    property,
                    computed: true,
                    ..
                },
            op: AssignOp::Eq,
            value,
            ..
        } => {
            array_expr_ok(object, by_id, slot_of)
                && number_expr_ok(property, by_id, slot_of)
                && value_expr_ok(value, by_id, slot_of)
        }
        _ => false,
    }
}

pub(super) fn string_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::String { .. } => true,
        Expr::Local { id, ty } => {
            slot_of.get(id) == Some(&SlotTy::String)
                || matches!(ty, Type::String)
                || by_id.get(id).is_some_and(|l| matches!(l.ty, Type::String))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && *computed
                && array_expr_ok(object, by_id, slot_of)
                && number_expr_ok(property, by_id, slot_of)
        }
        Expr::Binary {
            left,
            op: BinaryOp::Add,
            right,
            ..
        } => string_expr_ok(left, by_id, slot_of) && string_expr_ok(right, by_id, slot_of),
        _ => false,
    }
}

pub(super) fn bool_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Boolean { .. } => true,
        Expr::Local { id, ty } => {
            slot_of.get(id) == Some(&SlotTy::Bool)
                || matches!(ty, Type::Boolean)
                || by_id.get(id).is_some_and(|l| matches!(l.ty, Type::Boolean))
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && *computed
                && array_expr_ok(object, by_id, slot_of)
                && number_expr_ok(property, by_id, slot_of)
        }
        _ => false,
    }
}

pub(super) fn null_expr_ok(
    expr: &Expr,
    by_id: &HashMap<LocalId, &Local>,
    slot_of: &HashMap<LocalId, SlotTy>,
) -> bool {
    match expr {
        Expr::Null { .. } => true,
        _ if is_undefined_expr(expr) => true,
        Expr::Local { id, ty } => {
            is_undefined_local(*id, by_id)
                || slot_of.get(id) == Some(&SlotTy::Null)
                || matches!(ty, Type::Null | Type::Any)
                    && by_id.get(id).is_some_and(|l| {
                        matches!(l.ty, Type::Null | Type::Any) || l.name == "undefined"
                    })
        }
        Expr::Member {
            object,
            property,
            optional,
            computed,
            ..
        } => {
            !*optional
                && *computed
                && array_expr_ok(object, by_id, slot_of)
                && number_expr_ok(property, by_id, slot_of)
        }
        _ => false,
    }
}

pub(super) fn member_key_is_length(property: &Expr) -> bool {
    matches!(property, Expr::String { value, .. } if value.to_string_lossy() == "length")
}

pub(super) fn is_undefined_expr(expr: &Expr) -> bool {
    matches!(expr, Expr::IdentName { name, .. } if name == "undefined")
        || matches!(
            expr,
            Expr::Unary {
                op: draconic_ast::UnaryOp::Void,
                ..
            }
        )
}

pub(super) fn is_undefined_local(id: LocalId, by_id: &HashMap<LocalId, &Local>) -> bool {
    by_id.get(&id).is_some_and(|l| l.name == "undefined")
}

pub(super) fn member_key_string(property: &Expr) -> Option<String> {
    match property {
        Expr::String { value, .. } => Some(value.to_string_lossy().to_string()),
        _ => None,
    }
}
