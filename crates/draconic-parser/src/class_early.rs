use super::*;

/// Literal IdentifierName `constructor` only — not `"constructor"` or `['constructor']`.
pub(crate) fn class_key_is_literal_constructor(key: &ObjectKey) -> bool {
    matches!(key, ObjectKey::Ident(id) if id.name == "constructor")
}

/// PropName of a non-computed class element key (Ident / String / numeric→String).
fn class_element_prop_name(key: &ObjectKey) -> Option<String> {
    match key {
        ObjectKey::Ident(id) => Some(id.name.clone()),
        ObjectKey::String(s) => Some(s.value.to_string_lossy()),
        ObjectKey::Computed(_) => None,
    }
}

/// True when `expr` is (possibly parenthesized) `….#private` (E19.36 delete early error).
pub(crate) fn expr_is_private_member_reference(expr: &Expr) -> bool {
    match expr {
        Expr::Paren { expr: inner, .. } => expr_is_private_member_reference(inner),
        Expr::MemberExpression { private: true, .. } => true,
        _ => false,
    }
}

pub(crate) fn register_private_names_from_element(el: &ClassElement, frame: &mut Vec<String>) {
    match el {
        ClassElement::Field {
            key,
            is_private: true,
            ..
        }
        | ClassElement::Method {
            key,
            is_private: true,
            ..
        }
        | ClassElement::Accessor {
            key,
            is_private: true,
            ..
        } => {
            if let ObjectKey::Ident(id) = key {
                if !frame.iter().any(|n| n == &id.name) {
                    frame.push(id.name.clone());
                }
            }
        }
        _ => {}
    }
}

/// ClassBody early errors: duplicate privates, field PropName, SuperCall/arguments in field init,
/// SuperCall outside constructor, duplicate constructor, `#constructor`, undeclared private refs (E19.39).
///
/// `inherited` = private names from enclosing classes (nested class visibility).
pub(crate) fn validate_class_body(
    body: &[ClassElement],
    has_heritage: bool,
    inherited: &[String],
) -> Result<(), Diagnostic> {
    let mut private_names: Vec<String> = Vec::new();
    let mut ctor_count = 0u32;
    // Track private accessor kinds for static/instance getter+setter pairing rules.
    // name -> (has_instance_get, has_instance_set, has_static_get, has_static_set, has_field_or_method)
    let mut private_kinds: std::collections::HashMap<String, (bool, bool, bool, bool, bool)> =
        std::collections::HashMap::new();

    for el in body {
        match el {
            ClassElement::Field {
                key,
                value,
                is_static,
                is_private,
                span,
            } => {
                if *is_private {
                    if let ObjectKey::Ident(id) = key {
                        if id.name == "constructor" {
                            return Err(Diagnostic::new(
                                "private field cannot be named #constructor".to_string(),
                                *span,
                            ));
                        }
                        if private_names.iter().any(|n| n == &id.name) {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        private_names.push(id.name.clone());
                        let e = private_kinds.entry(id.name.clone()).or_default();
                        e.4 = true;
                    }
                } else if let Some(name) = class_element_prop_name(key) {
                    if name == "constructor" {
                        return Err(Diagnostic::new(
                            "class field cannot be named constructor".to_string(),
                            *span,
                        ));
                    }
                    if *is_static && name == "prototype" {
                        return Err(Diagnostic::new(
                            "static class field cannot be named prototype".to_string(),
                            *span,
                        ));
                    }
                }
                if let Some(v) = value {
                    if expr_contains_super_call(v) {
                        return Err(Diagnostic::new(
                            "class field initializer cannot contain super call".to_string(),
                            *span,
                        ));
                    }
                    if expr_contains_arguments_ref(v) {
                        return Err(Diagnostic::new(
                            "class field initializer cannot contain arguments".to_string(),
                            *span,
                        ));
                    }
                }
            }
            ClassElement::Method {
                key,
                params,
                body: method_body,
                is_static,
                is_private,
                is_async: _,
                is_generator: _,
                span,
            } => {
                if *is_private {
                    if let ObjectKey::Ident(id) = key {
                        if id.name == "constructor" {
                            return Err(Diagnostic::new(
                                "private method cannot be named #constructor".to_string(),
                                *span,
                            ));
                        }
                        if private_names.iter().any(|n| n == &id.name) {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        private_names.push(id.name.clone());
                        let e = private_kinds.entry(id.name.clone()).or_default();
                        e.4 = true;
                    }
                } else if *is_static {
                    if let Some(name) = class_element_prop_name(key) {
                        if name == "prototype" {
                            return Err(Diagnostic::new(
                                "static class method cannot be named prototype".to_string(),
                                *span,
                            ));
                        }
                    }
                }
                // SuperCall only allowed in constructors (not methods / static methods).
                if params_contain_super_call(params) || stmt_contains_super_call(method_body) {
                    return Err(Diagnostic::new(
                        "class method cannot contain super call".to_string(),
                        *span,
                    ));
                }
            }
            ClassElement::Accessor {
                key,
                params,
                body: accessor_body,
                is_static,
                is_private,
                kind,
                span,
            } => {
                if *is_private {
                    if let ObjectKey::Ident(id) = key {
                        if id.name == "constructor" {
                            return Err(Diagnostic::new(
                                "private accessor cannot be named #constructor".to_string(),
                                *span,
                            ));
                        }
                        let e = private_kinds.entry(id.name.clone()).or_default();
                        // Duplicate same-kind accessor (get+get / set+set) is an error.
                        let dup = match (kind, *is_static) {
                            (AccessorKind::Get, false) if e.0 => true,
                            (AccessorKind::Set, false) if e.1 => true,
                            (AccessorKind::Get, true) if e.2 => true,
                            (AccessorKind::Set, true) if e.3 => true,
                            _ => false,
                        };
                        if dup || e.4 {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        // get/set pair may share one PrivateBoundName (allow up to 2 entries).
                        if private_names.iter().filter(|n| *n == &id.name).count() >= 2 {
                            return Err(Diagnostic::new(
                                format!("duplicate private name #{}", id.name),
                                *span,
                            ));
                        }
                        private_names.push(id.name.clone());
                        match (kind, *is_static) {
                            (AccessorKind::Get, false) => e.0 = true,
                            (AccessorKind::Set, false) => e.1 = true,
                            (AccessorKind::Get, true) => e.2 = true,
                            (AccessorKind::Set, true) => e.3 = true,
                        }
                    }
                } else if let Some(name) = class_element_prop_name(key) {
                    // Instance accessors cannot be named "constructor"; static can.
                    if !*is_static && name == "constructor" {
                        return Err(Diagnostic::new(
                            "class accessor cannot be named constructor".to_string(),
                            *span,
                        ));
                    }
                    if *is_static && name == "prototype" {
                        return Err(Diagnostic::new(
                            "static class accessor cannot be named prototype".to_string(),
                            *span,
                        ));
                    }
                }
                if params_contain_super_call(params) || stmt_contains_super_call(accessor_body) {
                    return Err(Diagnostic::new(
                        "class accessor cannot contain super call".to_string(),
                        *span,
                    ));
                }
            }
            ClassElement::Constructor {
                params,
                body: ctor_body,
                span,
            } => {
                ctor_count += 1;
                if ctor_count > 1 {
                    return Err(Diagnostic::new(
                        "class may have at most one constructor".to_string(),
                        *span,
                    ));
                }
                // SuperCall in constructor formals is still an error.
                if params_contain_super_call(params) {
                    return Err(Diagnostic::new(
                        "constructor parameters cannot contain super call".to_string(),
                        *span,
                    ));
                }
                // SuperCall requires ClassHeritage (E19.39 grammar-ctor-super-no-heritage).
                if !has_heritage && stmt_contains_super_call(ctor_body) {
                    return Err(Diagnostic::new(
                        "super call only allowed in derived class constructor".to_string(),
                        *span,
                    ));
                }
            }
            ClassElement::StaticBlock {
                body: block_body,
                span,
            } => {
                if stmt_contains_super_call(block_body) {
                    return Err(Diagnostic::new(
                        "static block cannot contain super call".to_string(),
                        *span,
                    ));
                }
                if stmt_contains_return(block_body) {
                    return Err(Diagnostic::new(
                        "static block cannot contain return".to_string(),
                        *span,
                    ));
                }
                // ContainsArguments includes nested class computed names like `[arguments]`.
                if stmt_contains_arguments_ref(block_body)
                    || stmt_contains_arguments_deep(block_body)
                {
                    return Err(Diagnostic::new(
                        "static block cannot contain arguments".to_string(),
                        *span,
                    ));
                }
            }
        }
    }

    // Private getter/setter must not mix static and instance for the same name.
    for (name, (ig, is, sg, ss, field_or_method)) in &private_kinds {
        let has_instance = *ig || *is;
        let has_static = *sg || *ss;
        if has_instance && has_static {
            return Err(Diagnostic::new(
                format!("private name #{name} cannot mix static and instance accessors"),
                Span::dummy(),
            ));
        }
        if *field_or_method && (has_instance || has_static) {
            // field/method + accessor same name already caught by duplicate private_names
            // for methods/fields; accessors allow 2 entries so field+get may slip — handled above.
            let _ = field_or_method;
        }
    }

    // All private references must be in this class or an enclosing class.
    let mut visible = inherited.to_vec();
    for n in &private_names {
        if !visible.iter().any(|v| v == n) {
            visible.push(n.clone());
        }
    }
    for el in body {
        check_class_element_private_refs(el, &visible)?;
    }
    Ok(())
}

fn check_class_element_private_refs(
    el: &ClassElement,
    declared: &[String],
) -> Result<(), Diagnostic> {
    match el {
        ClassElement::Field {
            key, value, span, ..
        } => {
            if let ObjectKey::Computed(e) = key {
                check_expr_private_refs(e, declared, *span)?;
            }
            if let Some(v) = value {
                check_expr_private_refs(v, declared, *span)?;
            }
            Ok(())
        }
        ClassElement::Method {
            key,
            params,
            body,
            span,
            ..
        }
        | ClassElement::Accessor {
            key,
            params,
            body,
            span,
            ..
        } => {
            if let ObjectKey::Computed(e) = key {
                check_expr_private_refs(e, declared, *span)?;
            }
            for p in params {
                if let Some(d) = &p.default {
                    check_expr_private_refs(d, declared, *span)?;
                }
            }
            check_stmt_private_refs(body, declared)
        }
        ClassElement::Constructor { params, body, span } => {
            for p in params {
                if let Some(d) = &p.default {
                    check_expr_private_refs(d, declared, *span)?;
                }
            }
            check_stmt_private_refs(body, declared)
        }
        ClassElement::StaticBlock { body, .. } => check_stmt_private_refs(body, declared),
    }
}

pub(crate) fn check_stmt_private_refs(stmt: &Stmt, declared: &[String]) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Block { body, .. } => {
            for s in body {
                check_stmt_private_refs(s, declared)?;
            }
            Ok(())
        }
        Stmt::Expression { expr, span, .. } => check_expr_private_refs(expr, declared, *span),
        Stmt::Return { argument, span, .. } => {
            if let Some(a) = argument {
                check_expr_private_refs(a, declared, *span)?;
            }
            Ok(())
        }
        Stmt::Throw { argument, span, .. } => check_expr_private_refs(argument, declared, *span),
        Stmt::If {
            test,
            consequent,
            alternate,
            span,
            ..
        } => {
            check_expr_private_refs(test, declared, *span)?;
            check_stmt_private_refs(consequent, declared)?;
            if let Some(a) = alternate {
                check_stmt_private_refs(a, declared)?;
            }
            Ok(())
        }
        Stmt::While {
            test, body, span, ..
        }
        | Stmt::DoWhile {
            test, body, span, ..
        } => {
            check_expr_private_refs(test, declared, *span)?;
            check_stmt_private_refs(body, declared)
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            span,
            ..
        } => {
            if let Some(i) = init {
                check_stmt_private_refs(i, declared)?;
            }
            if let Some(t) = test {
                check_expr_private_refs(t, declared, *span)?;
            }
            if let Some(u) = update {
                check_expr_private_refs(u, declared, *span)?;
            }
            check_stmt_private_refs(body, declared)
        }
        Stmt::ForOf {
            left,
            right,
            body,
            span,
            ..
        }
        | Stmt::ForIn {
            left,
            right,
            body,
            span,
            ..
        } => {
            check_stmt_private_refs(left, declared)?;
            check_expr_private_refs(right, declared, *span)?;
            check_stmt_private_refs(body, declared)
        }
        Stmt::Labeled { body, .. } | Stmt::With { body, .. } => {
            check_stmt_private_refs(body, declared)
        }
        Stmt::Switch {
            discriminant,
            cases,
            span,
            ..
        } => {
            check_expr_private_refs(discriminant, declared, *span)?;
            for c in cases {
                if let Some(t) = &c.test {
                    check_expr_private_refs(t, declared, *span)?;
                }
                for s in &c.body {
                    check_stmt_private_refs(s, declared)?;
                }
            }
            Ok(())
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            check_stmt_private_refs(block, declared)?;
            if let Some(h) = handler {
                check_stmt_private_refs(h, declared)?;
            }
            if let Some(f) = finalizer {
                check_stmt_private_refs(f, declared)?;
            }
            Ok(())
        }
        Stmt::Let { init, span, .. } => {
            if let Some(i) = init {
                check_expr_private_refs(i, declared, *span)?;
            }
            Ok(())
        }
        Stmt::FunctionDeclaration {
            params, body, span, ..
        } => {
            // Nested functions inherit enclosing class private names (E19.36/E19.39).
            for p in params {
                if let Some(d) = &p.default {
                    check_expr_private_refs(d, declared, *span)?;
                }
            }
            check_stmt_private_refs(body, declared)
        }
        // Nested class introduces its own private environment (validated separately).
        // Heritage is outside that environment — check outer privates only.
        Stmt::ClassDeclaration {
            super_class, span, ..
        } => {
            if let Some(sc) = super_class {
                check_expr_private_refs(sc, declared, *span)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

fn check_expr_private_refs(expr: &Expr, declared: &[String], span: Span) -> Result<(), Diagnostic> {
    match expr {
        Expr::MemberExpression {
            object,
            property,
            private: true,
            ..
        } => {
            if let Expr::Ident(id) = property.as_ref() {
                if !declared.iter().any(|n| n == &id.name) {
                    return Err(Diagnostic::new(
                        format!("undeclared private field #{}", id.name),
                        span,
                    ));
                }
            }
            // `super.#x` is invalid (E19.39).
            if matches!(object.as_ref(), Expr::Super { .. }) {
                return Err(Diagnostic::new(
                    "private fields cannot be accessed on super".to_string(),
                    span,
                ));
            }
            check_expr_private_refs(object, declared, span)
        }
        Expr::PrivateIn { name, object, .. } => {
            if !declared.iter().any(|n| n == &name.name) {
                return Err(Diagnostic::new(
                    format!("undeclared private field #{}", name.name),
                    span,
                ));
            }
            check_expr_private_refs(object, declared, span)
        }
        Expr::FunctionExpression { body, params, .. } => {
            for p in params {
                if let Some(d) = &p.default {
                    check_expr_private_refs(d, declared, span)?;
                }
            }
            check_stmt_private_refs(body, declared)
        }
        Expr::ArrowFunction { body, params, .. } => {
            for p in params {
                if let Some(d) = &p.default {
                    check_expr_private_refs(d, declared, span)?;
                }
            }
            match body {
                ArrowBody::Expr(e) => check_expr_private_refs(e, declared, span),
                ArrowBody::Block(s) => check_stmt_private_refs(s, declared),
            }
        }
        // Nested class has its own private env; still check heritage for outer privates.
        Expr::ClassExpression {
            super_class, body, ..
        } => {
            if let Some(sc) = super_class {
                // Heritage is outside the class private environment.
                check_expr_private_refs(sc, declared, span)?;
            }
            // Body uses nested class's own declared names (already validated in parse).
            let _ = body;
            Ok(())
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            check_expr_private_refs(callee, declared, span)?;
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => check_expr_private_refs(e, declared, span)?,
                }
            }
            Ok(())
        }
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => {
            check_expr_private_refs(left, declared, span)?;
            check_expr_private_refs(right, declared, span)
        }
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } => {
            check_expr_private_refs(arg, declared, span)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            check_expr_private_refs(test, declared, span)?;
            check_expr_private_refs(consequent, declared, span)?;
            check_expr_private_refs(alternate, declared, span)
        }
        Expr::MemberExpression {
            object,
            property,
            private: false,
            ..
        } => {
            check_expr_private_refs(object, declared, span)?;
            check_expr_private_refs(property, declared, span)
        }
        Expr::ArrayExpression { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        check_expr_private_refs(e, declared, span)?;
                    }
                    ArrayElement::Elision => {}
                }
            }
            Ok(())
        }
        Expr::ObjectExpression { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { key, value, .. } => {
                        if let ObjectKey::Computed(e) = key {
                            check_expr_private_refs(e, declared, span)?;
                        }
                        check_expr_private_refs(value, declared, span)?;
                    }
                    ObjectProp::Spread { expr: e, .. } => {
                        check_expr_private_refs(e, declared, span)?;
                    }
                    ObjectProp::Accessor { key, body, .. } => {
                        if let ObjectKey::Computed(e) = key {
                            check_expr_private_refs(e, declared, span)?;
                        }
                        check_stmt_private_refs(body, declared)?;
                    }
                }
            }
            Ok(())
        }
        Expr::Paren { expr: inner, .. } | Expr::As { expr: inner, .. } => {
            check_expr_private_refs(inner, declared, span)
        }
        Expr::TemplateLiteral { expressions, .. } => {
            for e in expressions {
                check_expr_private_refs(e, declared, span)?;
            }
            Ok(())
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            check_expr_private_refs(tag, declared, span)?;
            for e in expressions {
                check_expr_private_refs(e, declared, span)?;
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

