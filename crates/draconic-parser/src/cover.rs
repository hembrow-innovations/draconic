use super::*;

/// CoverInitializedName encoded as shorthand Property with Assign value (E19.67).
pub(crate) fn expr_contains_cover_initialized_name(expr: &Expr) -> bool {
    match expr {
        Expr::Paren { expr: inner, .. } => expr_contains_cover_initialized_name(inner),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property {
                shorthand: true,
                value: Expr::Assign { .. },
                ..
            } => true,
            ObjectProp::Property { value, .. } => expr_contains_cover_initialized_name(value),
            ObjectProp::Spread { expr, .. } => expr_contains_cover_initialized_name(expr),
            ObjectProp::Accessor { .. } => false,
        }),
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                expr_contains_cover_initialized_name(e)
            }
            ArrayElement::Elision => false,
        }),
        Expr::Assign { target, value, .. } => {
            expr_contains_cover_initialized_name(target)
                || expr_contains_cover_initialized_name(value)
        }
        Expr::Binary { left, right, .. } => {
            expr_contains_cover_initialized_name(left)
                || expr_contains_cover_initialized_name(right)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_cover_initialized_name(test)
                || expr_contains_cover_initialized_name(consequent)
                || expr_contains_cover_initialized_name(alternate)
        }
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } | Expr::As { expr: arg, .. } => {
            expr_contains_cover_initialized_name(arg)
        }
        Expr::Call { callee, args, .. } => {
            expr_contains_cover_initialized_name(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_cover_initialized_name(e),
                })
        }
        Expr::MemberExpression {
            object, property, ..
        } => {
            expr_contains_cover_initialized_name(object)
                || expr_contains_cover_initialized_name(property)
        }
        _ => false,
    }
}

/// Split `pat = default` assignment into binding + default for pattern elements.
fn expr_to_pattern_element(expr: &Expr) -> Option<(BindingPattern, Option<Expr>)> {
    if let Expr::Assign {
        target,
        op: AssignOp::Eq,
        value,
        ..
    } = expr
    {
        let binding = expr_to_binding_pattern(target)?;
        return Some((binding, Some((**value).clone())));
    }
    let binding = expr_to_binding_pattern(expr)?;
    Some((binding, None))
}

/// Reinterpret an array literal as an assignment pattern when every element is
/// a binding/LHS target (`ident`, member, `pat = default`, nested pattern, elision, or trailing rest).
pub(crate) fn array_expr_to_pattern(expr: &Expr) -> Option<Expr> {
    let Expr::ArrayExpression {
        elements,
        trailing_comma,
        span,
    } = expr
    else {
        return None;
    };
    let mut pat_els = Vec::with_capacity(elements.len());
    let mut saw_rest = false;
    for el in elements {
        if saw_rest {
            return None;
        }
        match el {
            ArrayElement::Elision => {
                pat_els.push(ArrayPatternElement::Elision);
            }
            ArrayElement::Expr(inner) => {
                let (binding, default) = expr_to_pattern_element(inner)?;
                pat_els.push(ArrayPatternElement::Pattern { binding, default });
            }
            ArrayElement::Spread(inner) => {
                let binding = expr_to_binding_pattern(inner)?;
                pat_els.push(ArrayPatternElement::Rest(binding));
                saw_rest = true;
            }
        }
    }
    // `[...x,]` — trailing comma after rest is a SyntaxError in assignment patterns.
    if *trailing_comma && saw_rest {
        return None;
    }
    Some(Expr::ArrayPattern {
        elements: pat_els,
        span: *span,
    })
}

/// Reinterpret an object literal as an assignment pattern when every property is
/// a binding target (shorthand, CoverInitializedName, `key: pattern`, or trailing `...ident`).
pub(crate) fn object_expr_to_pattern(expr: &Expr) -> Option<Expr> {
    let Expr::ObjectExpression { properties, span } = expr else {
        return None;
    };
    let mut props = Vec::with_capacity(properties.len());
    let mut saw_rest = false;
    for prop in properties {
        if saw_rest {
            return None;
        }
        match prop {
            ObjectProp::Property {
                key,
                value,
                shorthand,
                span: prop_span,
            } => {
                // CoverInitializedName: `{ a = default }` encoded as shorthand Assign.
                if *shorthand {
                    let ObjectKey::Ident(key_id) = key else {
                        return None;
                    };
                    if let Expr::Assign {
                        target,
                        op: AssignOp::Eq,
                        value: def,
                        ..
                    } = value
                    {
                        let Expr::Ident(id) = target.as_ref() else {
                            return None;
                        };
                        if id.name != key_id.name {
                            return None;
                        }
                        props.push(ObjectPatternProp::Prop {
                            key: key.clone(),
                            binding: BindingPattern::Ident(id.clone()),
                            shorthand: true,
                            default: Some((**def).clone()),
                            span: *prop_span,
                        });
                        continue;
                    }
                }
                let (binding, default) = expr_to_pattern_element(value)?;
                props.push(ObjectPatternProp::Prop {
                    key: key.clone(),
                    binding,
                    shorthand: *shorthand,
                    default,
                    span: *prop_span,
                });
            }
            ObjectProp::Spread { expr: inner, .. } => {
                let binding = expr_to_binding_pattern(inner)?;
                props.push(ObjectPatternProp::Rest(binding));
                saw_rest = true;
            }
            ObjectProp::Accessor { .. } => return None,
        }
    }
    Some(Expr::ObjectPattern {
        properties: props,
        span: *span,
    })
}

pub(crate) fn expr_to_binding_pattern(expr: &Expr) -> Option<BindingPattern> {
    match expr {
        Expr::Ident(id) => Some(BindingPattern::Ident(id.clone())),
        // E19.82.10: private members (`obj.#f`) are valid assignment-pattern targets.
        Expr::MemberExpression {
            optional: false, ..
        } => Some(BindingPattern::Member(Box::new(expr.clone()))),
        Expr::ArrayExpression {
            elements,
            trailing_comma,
            span,
        } => {
            let mut pat_els = Vec::with_capacity(elements.len());
            let mut saw_rest = false;
            for el in elements {
                if saw_rest {
                    return None;
                }
                match el {
                    ArrayElement::Elision => {
                        pat_els.push(ArrayPatternElement::Elision);
                    }
                    ArrayElement::Expr(inner) => {
                        let (binding, default) = expr_to_pattern_element(inner)?;
                        pat_els.push(ArrayPatternElement::Pattern { binding, default });
                    }
                    ArrayElement::Spread(inner) => {
                        let binding = expr_to_binding_pattern(inner)?;
                        pat_els.push(ArrayPatternElement::Rest(binding));
                        saw_rest = true;
                    }
                }
            }
            if *trailing_comma && saw_rest {
                return None;
            }
            Some(BindingPattern::Array {
                elements: pat_els,
                span: *span,
            })
        }
        Expr::ArrayPattern { elements, span } => Some(BindingPattern::Array {
            elements: elements.clone(),
            span: *span,
        }),
        Expr::ObjectExpression { properties, span } => {
            let mut props = Vec::with_capacity(properties.len());
            let mut saw_rest = false;
            for prop in properties {
                if saw_rest {
                    return None;
                }
                match prop {
                    ObjectProp::Property {
                        key,
                        value,
                        shorthand,
                        span: prop_span,
                    } => {
                        if *shorthand {
                            let ObjectKey::Ident(key_id) = key else {
                                return None;
                            };
                            if let Expr::Assign {
                                target,
                                op: AssignOp::Eq,
                                value: def,
                                ..
                            } = value
                            {
                                let Expr::Ident(id) = target.as_ref() else {
                                    return None;
                                };
                                if id.name != key_id.name {
                                    return None;
                                }
                                props.push(ObjectPatternProp::Prop {
                                    key: key.clone(),
                                    binding: BindingPattern::Ident(id.clone()),
                                    shorthand: true,
                                    default: Some((**def).clone()),
                                    span: *prop_span,
                                });
                                continue;
                            }
                        }
                        let (binding, default) = expr_to_pattern_element(value)?;
                        props.push(ObjectPatternProp::Prop {
                            key: key.clone(),
                            binding,
                            shorthand: *shorthand,
                            default,
                            span: *prop_span,
                        });
                    }
                    ObjectProp::Spread { expr: inner, .. } => {
                        let binding = expr_to_binding_pattern(inner)?;
                        props.push(ObjectPatternProp::Rest(binding));
                        saw_rest = true;
                    }
                    ObjectProp::Accessor { .. } => return None,
                }
            }
            Some(BindingPattern::Object {
                properties: props,
                span: *span,
            })
        }
        Expr::ObjectPattern { properties, span } => Some(BindingPattern::Object {
            properties: properties.clone(),
            span: *span,
        }),
        _ => None,
    }
}

