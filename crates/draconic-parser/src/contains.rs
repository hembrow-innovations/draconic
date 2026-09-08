use super::*;

/// AwaitExpression in FormalParameters of async functions (E19.67).
pub(crate) fn params_contain_await_expr(params: &[Param]) -> bool {
    params.iter().any(|p| {
        p.default.as_ref().is_some_and(expr_contains_await_expr)
            || binding_pattern_contains_await_expr(&p.binding)
    })
}

fn binding_pattern_contains_await_expr(b: &BindingPattern) -> bool {
    match b {
        BindingPattern::Ident(_) | BindingPattern::Member(_) => false,
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Elision => false,
            ArrayPatternElement::Pattern { binding, default } => {
                binding_pattern_contains_await_expr(binding)
                    || default.as_ref().is_some_and(expr_contains_await_expr)
            }
            ArrayPatternElement::Rest(inner) => binding_pattern_contains_await_expr(inner),
        }),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                object_key_contains_await_expr(key)
                    || binding_pattern_contains_await_expr(binding)
                    || default.as_ref().is_some_and(expr_contains_await_expr)
            }
            ObjectPatternProp::Rest(inner) => binding_pattern_contains_await_expr(inner),
        }),
    }
}

fn object_key_contains_await_expr(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_await_expr(e),
        ObjectKey::Ident(_) | ObjectKey::String(_) => false,
    }
}

fn expr_contains_await_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Unary {
            op: UnaryOp::Await, ..
        } => true,
        // Nested functions/classes hide await of their bodies for outer Contains.
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::ArrowFunction { body, params, .. } => {
            params_contain_await_expr(params)
                || match body {
                    ArrowBody::Expr(e) => expr_contains_await_expr(e),
                    ArrowBody::Block(s) => stmt_contains_await_expr(s),
                }
        }
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_await_expr(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_await_expr(left) || expr_contains_await_expr(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_await_expr(test)
                || expr_contains_await_expr(consequent)
                || expr_contains_await_expr(alternate)
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_await_expr(object) || expr_contains_await_expr(property),
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_await_expr(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_await_expr(e),
                })
        }
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_await_expr(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value, .. } => {
                object_key_contains_await_expr(key) || expr_contains_await_expr(value)
            }
            ObjectProp::Spread { expr, .. } => expr_contains_await_expr(expr),
            ObjectProp::Accessor { key, .. } => object_key_contains_await_expr(key),
        }),
        Expr::TemplateLiteral { expressions, .. } => {
            expressions.iter().any(expr_contains_await_expr)
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => expr_contains_await_expr(tag) || expressions.iter().any(expr_contains_await_expr),
        Expr::ImportCall {
            source, options, ..
        } => {
            expr_contains_await_expr(source)
                || options
                    .as_ref()
                    .is_some_and(|o| expr_contains_await_expr(o))
        }
        Expr::PrivateIn { object, .. } => expr_contains_await_expr(object),
        _ => false,
    }
}

fn stmt_contains_await_expr(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expression { expr, .. } => expr_contains_await_expr(expr),
        Stmt::Throw { argument, .. } => expr_contains_await_expr(argument),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_await_expr),
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_await_expr),
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_await_expr),
        _ => false,
    }
}

pub(crate) fn params_contain_super_call(params: &[Param]) -> bool {
    params.iter().any(|p| {
        p.default.as_ref().is_some_and(expr_contains_super_call)
            || binding_pattern_contains_super_call(&p.binding)
    })
}

pub(crate) fn params_contain_yield_expr(params: &[Param]) -> bool {
    params.iter().any(|p| {
        p.default.as_ref().is_some_and(expr_contains_yield_expr)
            || binding_pattern_contains_yield_expr(&p.binding)
    })
}

fn binding_pattern_contains_yield_expr(b: &BindingPattern) -> bool {
    match b {
        BindingPattern::Ident(_) | BindingPattern::Member(_) => false,
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Elision => false,
            ArrayPatternElement::Pattern { binding, default } => {
                binding_pattern_contains_yield_expr(binding)
                    || default.as_ref().is_some_and(expr_contains_yield_expr)
            }
            ArrayPatternElement::Rest(inner) => binding_pattern_contains_yield_expr(inner),
        }),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                object_key_contains_yield_expr(key)
                    || binding_pattern_contains_yield_expr(binding)
                    || default.as_ref().is_some_and(expr_contains_yield_expr)
            }
            ObjectPatternProp::Rest(inner) => binding_pattern_contains_yield_expr(inner),
        }),
    }
}

fn object_key_contains_yield_expr(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_yield_expr(e),
        ObjectKey::Ident(_) | ObjectKey::String(_) => false,
    }
}

fn expr_contains_yield_expr(expr: &Expr) -> bool {
    match expr {
        Expr::Unary {
            op: UnaryOp::Yield | UnaryOp::YieldStar,
            ..
        } => true,
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::ArrowFunction { body, params, .. } => {
            params_contain_yield_expr(params)
                || match body {
                    ArrowBody::Expr(e) => expr_contains_yield_expr(e),
                    ArrowBody::Block(s) => stmt_contains_yield_expr(s),
                }
        }
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_yield_expr(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_yield_expr(left) || expr_contains_yield_expr(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_yield_expr(test)
                || expr_contains_yield_expr(consequent)
                || expr_contains_yield_expr(alternate)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_yield_expr(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_yield_expr(e),
                })
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_yield_expr(object) || expr_contains_yield_expr(property),
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_yield_expr(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, .. } => expr_contains_yield_expr(value),
            ObjectProp::Spread { expr, .. } => expr_contains_yield_expr(expr),
            ObjectProp::Accessor { .. } => false,
        }),
        _ => false,
    }
}

fn stmt_contains_yield_expr(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expression { expr, .. } => expr_contains_yield_expr(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_yield_expr),
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_yield_expr),
        _ => false,
    }
}

fn binding_pattern_contains_super_call(b: &BindingPattern) -> bool {
    match b {
        BindingPattern::Ident(_) | BindingPattern::Member(_) => false,
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Elision => false,
            ArrayPatternElement::Pattern { binding, default } => {
                binding_pattern_contains_super_call(binding)
                    || default.as_ref().is_some_and(expr_contains_super_call)
            }
            ArrayPatternElement::Rest(inner) => binding_pattern_contains_super_call(inner),
        }),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                object_key_contains_super_call(key)
                    || binding_pattern_contains_super_call(binding)
                    || default.as_ref().is_some_and(expr_contains_super_call)
            }
            ObjectPatternProp::Rest(inner) => binding_pattern_contains_super_call(inner),
        }),
    }
}

pub(crate) fn stmt_contains_return(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Return { .. } => true,
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_return),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            stmt_contains_return(consequent)
                || alternate.as_ref().is_some_and(|a| stmt_contains_return(a))
        }
        Stmt::While { body, .. }
        | Stmt::DoWhile { body, .. }
        | Stmt::For { body, .. }
        | Stmt::ForOf { body, .. }
        | Stmt::ForIn { body, .. }
        | Stmt::Labeled { body, .. }
        | Stmt::With { body, .. } => stmt_contains_return(body),
        Stmt::Switch { cases, .. } => cases
            .iter()
            .any(|c| c.body.iter().any(stmt_contains_return)),
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            stmt_contains_return(block)
                || handler.as_ref().is_some_and(|h| stmt_contains_return(h))
                || finalizer.as_ref().is_some_and(|f| stmt_contains_return(f))
        }
        Stmt::FunctionDeclaration { .. } | Stmt::ClassDeclaration { .. } => false,
        _ => false,
    }
}

/// Deeper ContainsArguments walk for static blocks: enter nested class expressions
/// and scan computed property names / heritage (E19.39 static-init-invalid-arguments).
pub(crate) fn stmt_contains_arguments_deep(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expression { expr, .. } => expr_contains_arguments_deep(expr),
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_arguments_deep),
        Stmt::Return { argument, .. } => {
            argument.as_ref().is_some_and(expr_contains_arguments_deep)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_arguments_deep),
        _ => false,
    }
}

fn expr_contains_arguments_deep(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(id) if id.name == "arguments" => true,
        Expr::ClassExpression {
            super_class, body, ..
        } => {
            super_class
                .as_ref()
                .is_some_and(|sc| expr_contains_arguments_deep(sc))
                || body.iter().any(class_element_contains_arguments_deep)
        }
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_arguments_deep(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_arguments_deep(left) || expr_contains_arguments_deep(right),
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_arguments_deep(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_arguments_deep(e),
                })
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_arguments_deep(object) || expr_contains_arguments_deep(property),
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_arguments_deep(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value, .. } => {
                object_key_contains_arguments_deep(key) || expr_contains_arguments_deep(value)
            }
            ObjectProp::Spread { expr, .. } => expr_contains_arguments_deep(expr),
            ObjectProp::Accessor { key, .. } => object_key_contains_arguments_deep(key),
        }),
        Expr::FunctionExpression { .. } | Expr::ArrowFunction { .. } => false,
        _ => false,
    }
}

fn object_key_contains_arguments_deep(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_arguments_deep(e),
        ObjectKey::Ident(id) => id.name == "arguments",
        ObjectKey::String(_) => false,
    }
}

fn class_element_contains_arguments_deep(el: &ClassElement) -> bool {
    match el {
        ClassElement::Method { key, .. }
        | ClassElement::Accessor { key, .. }
        | ClassElement::Field { key, .. } => object_key_contains_arguments_deep(key),
        ClassElement::Constructor { .. } | ClassElement::StaticBlock { .. } => false,
    }
}

/// `Contains SuperCall` for field initializers: recurse into arrows; skip nested functions/classes.
pub(crate) fn expr_contains_super_call(expr: &Expr) -> bool {
    match expr {
        Expr::Call { callee, args, .. } => {
            matches!(callee.as_ref(), Expr::Super { .. })
                || expr_contains_super_call(callee)
                || args.iter().any(arg_contains_super_call)
        }
        Expr::ArrowFunction { body, params, .. } => {
            params
                .iter()
                .any(|p| p.default.as_ref().is_some_and(expr_contains_super_call))
                || match body {
                    ArrowBody::Expr(e) => expr_contains_super_call(e),
                    ArrowBody::Block(s) => stmt_contains_super_call(s),
                }
        }
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_super_call(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_super_call(left) || expr_contains_super_call(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super_call(test)
                || expr_contains_super_call(consequent)
                || expr_contains_super_call(alternate)
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_contains_super_call(object) || expr_contains_super_call(property),
        Expr::New { callee, args, .. } => {
            expr_contains_super_call(callee) || args.iter().any(arg_contains_super_call)
        }
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_super_call(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value, .. } => {
                object_key_contains_super_call(key) || expr_contains_super_call(value)
            }
            ObjectProp::Spread { expr, .. } => expr_contains_super_call(expr),
            ObjectProp::Accessor { key, .. } => object_key_contains_super_call(key),
        }),
        Expr::TemplateLiteral { expressions, .. } => {
            expressions.iter().any(expr_contains_super_call)
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => expr_contains_super_call(tag) || expressions.iter().any(expr_contains_super_call),
        Expr::ImportCall {
            source, options, ..
        } => {
            expr_contains_super_call(source)
                || options
                    .as_ref()
                    .is_some_and(|o| expr_contains_super_call(o))
        }
        Expr::PrivateIn { object, .. } => expr_contains_super_call(object),
        Expr::ArrayPattern { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Pattern { default, .. } => {
                default.as_ref().is_some_and(expr_contains_super_call)
            }
            _ => false,
        }),
        Expr::ObjectPattern { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop { key, default, .. } => {
                object_key_contains_super_call(key)
                    || default.as_ref().is_some_and(expr_contains_super_call)
            }
            _ => false,
        }),
        _ => false,
    }
}

fn arg_contains_super_call(a: &Arg) -> bool {
    match a {
        Arg::Expr(e) | Arg::Spread(e) => expr_contains_super_call(e),
    }
}

fn object_key_contains_super_call(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_super_call(e),
        ObjectKey::Ident(_) | ObjectKey::String(_) => false,
    }
}

pub(crate) fn stmt_contains_super_call(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_super_call),
        Stmt::Expression { expr, .. } => expr_contains_super_call(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_super_call),
        Stmt::Throw { argument, .. } => expr_contains_super_call(argument),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_super_call(test)
                || stmt_contains_super_call(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_contains_super_call(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_contains_super_call(test) || stmt_contains_super_call(body)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_super_call),
        _ => false,
    }
}

/// `ContainsArguments` for field initializers: recurse into arrows; skip nested functions/classes.
pub(crate) fn expr_contains_arguments_ref(expr: &Expr) -> bool {
    match expr {
        Expr::Ident(id) if id.name == "arguments" => true,
        Expr::ArrowFunction { body, params, .. } => {
            params.iter().any(|p| {
                p.default.as_ref().is_some_and(expr_contains_arguments_ref)
                    || binding_contains_arguments(&p.binding)
            }) || match body {
                ArrowBody::Expr(e) => expr_contains_arguments_ref(e),
                ArrowBody::Block(s) => stmt_contains_arguments_ref(s),
            }
        }
        Expr::FunctionExpression { .. } | Expr::ClassExpression { .. } => false,
        Expr::Paren { expr: inner, .. }
        | Expr::Unary { arg: inner, .. }
        | Expr::Update { arg: inner, .. }
        | Expr::As { expr: inner, .. } => expr_contains_arguments_ref(inner),
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => expr_contains_arguments_ref(left) || expr_contains_arguments_ref(right),
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_arguments_ref(test)
                || expr_contains_arguments_ref(consequent)
                || expr_contains_arguments_ref(alternate)
        }
        Expr::MemberExpression {
            object,
            property,
            computed,
            ..
        } => {
            expr_contains_arguments_ref(object)
                || (*computed && expr_contains_arguments_ref(property))
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_contains_arguments_ref(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_contains_arguments_ref(e),
                })
        }
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_contains_arguments_ref(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { key, value, .. } => {
                object_key_contains_arguments(key) || expr_contains_arguments_ref(value)
            }
            ObjectProp::Spread { expr, .. } => expr_contains_arguments_ref(expr),
            ObjectProp::Accessor { key, .. } => object_key_contains_arguments(key),
        }),
        Expr::TemplateLiteral { expressions, .. } => {
            expressions.iter().any(expr_contains_arguments_ref)
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            expr_contains_arguments_ref(tag) || expressions.iter().any(expr_contains_arguments_ref)
        }
        Expr::ImportCall {
            source, options, ..
        } => {
            expr_contains_arguments_ref(source)
                || options
                    .as_ref()
                    .is_some_and(|o| expr_contains_arguments_ref(o))
        }
        Expr::PrivateIn { object, .. } => expr_contains_arguments_ref(object),
        Expr::ArrayPattern { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Pattern { binding, default } => {
                binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ArrayPatternElement::Rest(b) => binding_contains_arguments(b),
            ArrayPatternElement::Elision => false,
        }),
        Expr::ObjectPattern { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                object_key_contains_arguments(key)
                    || binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ObjectPatternProp::Rest(b) => binding_contains_arguments(b),
        }),
        _ => false,
    }
}

fn object_key_contains_arguments(key: &ObjectKey) -> bool {
    match key {
        ObjectKey::Computed(e) => expr_contains_arguments_ref(e),
        ObjectKey::Ident(_) | ObjectKey::String(_) => false,
    }
}

fn binding_contains_arguments(b: &BindingPattern) -> bool {
    match b {
        BindingPattern::Ident(id) => id.name == "arguments",
        BindingPattern::Member(e) => expr_contains_arguments_ref(e),
        BindingPattern::Object { properties, .. } => properties.iter().any(|p| match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                default,
                ..
            } => {
                object_key_contains_arguments(key)
                    || binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ObjectPatternProp::Rest(inner) => binding_contains_arguments(inner),
        }),
        BindingPattern::Array { elements, .. } => elements.iter().any(|el| match el {
            ArrayPatternElement::Pattern { binding, default } => {
                binding_contains_arguments(binding)
                    || default.as_ref().is_some_and(expr_contains_arguments_ref)
            }
            ArrayPatternElement::Rest(inner) => binding_contains_arguments(inner),
            ArrayPatternElement::Elision => false,
        }),
    }
}

pub(crate) fn stmt_contains_arguments_ref(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Block { body, .. } => body.iter().any(stmt_contains_arguments_ref),
        Stmt::Expression { expr, .. } => expr_contains_arguments_ref(expr),
        Stmt::Return { argument, .. } => argument.as_ref().is_some_and(expr_contains_arguments_ref),
        Stmt::Throw { argument, .. } => expr_contains_arguments_ref(argument),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_contains_arguments_ref(test)
                || stmt_contains_arguments_ref(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_contains_arguments_ref(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_contains_arguments_ref(test) || stmt_contains_arguments_ref(body)
        }
        Stmt::Let { init, .. } => init.as_ref().is_some_and(expr_contains_arguments_ref),
        _ => false,
    }
}

