use draconic_ast::{
    Arg, ArrayElement, ArrayPatternElement, ArrowBody, BindingPattern, ClassElement, Expr,
    ObjectKey, ObjectPatternProp, ObjectProp, Param, Stmt,
};
use draconic_diagnostics::Span;

/// Fresh spans outside typical source ranges so binder span→symbol maps stay unique.
pub(crate) struct SyntheticSpans {
    next: u32,
}

impl SyntheticSpans {
    pub(crate) fn new() -> Self {
        // High half of u32 avoids colliding with real UTF-8 source offsets in fixtures.
        Self { next: 0x8000_0000 }
    }

    pub(crate) fn next(&mut self) -> Span {
        let start = self.next;
        self.next = self.next.saturating_add(2);
        Span::new(start, start + 1)
    }
}

/// Assign fresh unique spans across a linked module body so multi-file programs
/// do not share binder/IR span keys (source offsets are per-file).
pub(crate) fn uniqueify_stmt_spans(stmt: &mut Stmt, spans: &mut SyntheticSpans) {
    match stmt {
        Stmt::Expression { expr, span } => {
            *span = spans.next();
            uniqueify_expr_spans(expr, spans);
        }
        Stmt::Let {
            binding,
            init,
            span,
            ..
        } => {
            *span = spans.next();
            uniqueify_binding_spans(binding, spans);
            if let Some(init) = init {
                uniqueify_expr_spans(init, spans);
            }
        }
        Stmt::Empty { span } => *span = spans.next(),
        Stmt::Block { body, span } => {
            *span = spans.next();
            for s in body {
                uniqueify_stmt_spans(s, spans);
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            span,
        } => {
            *span = spans.next();
            uniqueify_expr_spans(test, spans);
            uniqueify_stmt_spans(consequent, spans);
            if let Some(alt) = alternate {
                uniqueify_stmt_spans(alt, spans);
            }
        }
        Stmt::While { test, body, span } | Stmt::DoWhile { body, test, span } => {
            *span = spans.next();
            uniqueify_expr_spans(test, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            span,
        } => {
            *span = spans.next();
            if let Some(init) = init {
                uniqueify_stmt_spans(init, spans);
            }
            if let Some(test) = test {
                uniqueify_expr_spans(test, spans);
            }
            if let Some(update) = update {
                uniqueify_expr_spans(update, spans);
            }
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::ForIn {
            left,
            right,
            body,
            span,
        } => {
            *span = spans.next();
            uniqueify_stmt_spans(left, spans);
            uniqueify_expr_spans(right, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::ForOf {
            left,
            right,
            body,
            span,
            ..
        } => {
            *span = spans.next();
            uniqueify_stmt_spans(left, spans);
            uniqueify_expr_spans(right, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::Break { label, span } | Stmt::Continue { label, span } => {
            *span = spans.next();
            if let Some(label) = label {
                label.span = spans.next();
            }
        }
        Stmt::Labeled { label, body, span } => {
            *span = spans.next();
            label.span = spans.next();
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::Switch {
            discriminant,
            cases,
            span,
        } => {
            *span = spans.next();
            uniqueify_expr_spans(discriminant, spans);
            for case in cases {
                case.span = spans.next();
                if let Some(test) = &mut case.test {
                    uniqueify_expr_spans(test, spans);
                }
                for s in &mut case.body {
                    uniqueify_stmt_spans(s, spans);
                }
            }
        }
        Stmt::FunctionDeclaration {
            name,
            params,
            body,
            span,
            ..
        } => {
            *span = spans.next();
            name.span = spans.next();
            uniqueify_params_spans(params, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::ClassDeclaration {
            name,
            super_class,
            body,
            span,
        } => {
            *span = spans.next();
            name.span = spans.next();
            if let Some(sc) = super_class {
                uniqueify_expr_spans(sc, spans);
            }
            for el in body {
                match el {
                    ClassElement::Constructor { params, body, span } => {
                        *span = spans.next();
                        uniqueify_params_spans(params, spans);
                        uniqueify_stmt_spans(body, spans);
                    }
                    ClassElement::Method {
                        key,
                        params,
                        body,
                        span,
                        ..
                    } => {
                        *span = spans.next();
                        uniqueify_object_key_spans(key, spans);
                        uniqueify_params_spans(params, spans);
                        uniqueify_stmt_spans(body, spans);
                    }
                    ClassElement::Accessor {
                        key,
                        params,
                        body,
                        span,
                        ..
                    } => {
                        *span = spans.next();
                        uniqueify_object_key_spans(key, spans);
                        uniqueify_params_spans(params, spans);
                        uniqueify_stmt_spans(body, spans);
                    }
                    ClassElement::Field {
                        key, value, span, ..
                    } => {
                        *span = spans.next();
                        uniqueify_object_key_spans(key, spans);
                        if let Some(v) = value {
                            uniqueify_expr_spans(v, spans);
                        }
                    }
                    ClassElement::StaticBlock { body, span } => {
                        *span = spans.next();
                        uniqueify_stmt_spans(body, spans);
                    }
                }
            }
        }
        Stmt::Return { argument, span } => {
            *span = spans.next();
            if let Some(arg) = argument {
                uniqueify_expr_spans(arg, spans);
            }
        }
        Stmt::Throw { argument, span } => {
            *span = spans.next();
            uniqueify_expr_spans(argument, spans);
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            span,
        } => {
            *span = spans.next();
            uniqueify_stmt_spans(block, spans);
            if let Some(param) = handler_param {
                uniqueify_binding_spans(param, spans);
            }
            if let Some(handler) = handler {
                uniqueify_stmt_spans(handler, spans);
            }
            if let Some(finalizer) = finalizer {
                uniqueify_stmt_spans(finalizer, spans);
            }
        }
        Stmt::With { object, body, span } => {
            *span = spans.next();
            uniqueify_expr_spans(object, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Stmt::ImportDeclaration { span, .. }
        | Stmt::ExportNamedDeclaration { span, .. }
        | Stmt::ExportDefaultDeclaration { span, .. }
        | Stmt::ExportAllDeclaration { span, .. } => {
            *span = spans.next();
        }
        Stmt::TypeAlias { name, span, .. } => {
            *span = spans.next();
            name.span = spans.next();
        }
        Stmt::ExternFunctionDeclaration {
            abi,
            name,
            params,
            span,
            ..
        } => {
            *span = spans.next();
            abi.span = spans.next();
            name.span = spans.next();
            uniqueify_params_spans(params, spans);
        }
    }
}

pub(crate) fn uniqueify_binding_spans(pat: &mut BindingPattern, spans: &mut SyntheticSpans) {
    match pat {
        BindingPattern::Ident(id) => id.span = spans.next(),
        BindingPattern::Member(expr) => uniqueify_expr_spans(expr, spans),
        BindingPattern::Array { elements, span } => {
            *span = spans.next();
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        uniqueify_binding_spans(binding, spans);
                        if let Some(def) = default {
                            uniqueify_expr_spans(def, spans);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => uniqueify_binding_spans(binding, spans),
                }
            }
        }
        BindingPattern::Object { properties, span } => {
            *span = spans.next();
            for p in properties {
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        default,
                        span: prop_span,
                        ..
                    } => {
                        *prop_span = spans.next();
                        uniqueify_object_key_spans(key, spans);
                        uniqueify_binding_spans(binding, spans);
                        if let Some(def) = default {
                            uniqueify_expr_spans(def, spans);
                        }
                    }
                    ObjectPatternProp::Rest(binding) => uniqueify_binding_spans(binding, spans),
                }
            }
        }
    }
}

pub(crate) fn uniqueify_object_key_spans(key: &mut ObjectKey, spans: &mut SyntheticSpans) {
    match key {
        ObjectKey::Ident(id) => id.span = spans.next(),
        ObjectKey::String(s) => s.span = spans.next(),
        ObjectKey::Computed(e) => uniqueify_expr_spans(e, spans),
    }
}

pub(crate) fn uniqueify_params_spans(params: &mut [Param], spans: &mut SyntheticSpans) {
    for p in params {
        uniqueify_binding_spans(&mut p.binding, spans);
        if let Some(default) = &mut p.default {
            uniqueify_expr_spans(default, spans);
        }
    }
}

pub(crate) fn uniqueify_expr_spans(expr: &mut Expr, spans: &mut SyntheticSpans) {
    match expr {
        Expr::Ident(id) => id.span = spans.next(),
        Expr::Number(n) => n.span = spans.next(),
        Expr::BigInt(n) => n.span = spans.next(),
        Expr::String(s) => s.span = spans.next(),
        Expr::RegExp { span, .. } => *span = spans.next(),
        Expr::Boolean { span, .. }
        | Expr::Null { span }
        | Expr::This { span }
        | Expr::Super { span }
        | Expr::NewTarget { span }
        | Expr::ImportMeta { span } => *span = spans.next(),
        Expr::ImportCall {
            source,
            options,
            span,
            ..
        } => {
            *span = spans.next();
            uniqueify_expr_spans(source, spans);
            if let Some(opts) = options {
                uniqueify_expr_spans(opts, spans);
            }
        }
        Expr::TemplateLiteral {
            quasis,
            expressions,
            span,
        } => {
            *span = spans.next();
            for q in quasis {
                q.span = spans.next();
            }
            for e in expressions {
                uniqueify_expr_spans(e, spans);
            }
        }
        Expr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            span,
        } => {
            *span = spans.next();
            uniqueify_expr_spans(tag, spans);
            for q in quasis {
                q.span = spans.next();
            }
            for e in expressions {
                uniqueify_expr_spans(e, spans);
            }
        }
        Expr::Unary { arg, span, .. }
        | Expr::Update { arg, span, .. }
        | Expr::Paren { expr: arg, span } => {
            *span = spans.next();
            uniqueify_expr_spans(arg, spans);
        }
        Expr::As { expr, span, .. } => {
            *span = spans.next();
            uniqueify_expr_spans(expr, spans);
        }
        Expr::Binary {
            left, right, span, ..
        } => {
            *span = spans.next();
            uniqueify_expr_spans(left, spans);
            uniqueify_expr_spans(right, spans);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            span,
        } => {
            *span = spans.next();
            uniqueify_expr_spans(test, spans);
            uniqueify_expr_spans(consequent, spans);
            uniqueify_expr_spans(alternate, spans);
        }
        Expr::Assign {
            target,
            value,
            span,
            ..
        } => {
            *span = spans.next();
            uniqueify_expr_spans(target, spans);
            uniqueify_expr_spans(value, spans);
        }
        Expr::Call {
            callee, args, span, ..
        }
        | Expr::New { callee, args, span } => {
            *span = spans.next();
            uniqueify_expr_spans(callee, spans);
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => uniqueify_expr_spans(e, spans),
                }
            }
        }
        Expr::FunctionExpression {
            name,
            params,
            body,
            span,
            ..
        } => {
            *span = spans.next();
            if let Some(name) = name {
                name.span = spans.next();
            }
            uniqueify_params_spans(params, spans);
            uniqueify_stmt_spans(body, spans);
        }
        Expr::ClassExpression {
            name,
            super_class,
            body,
            span,
        } => {
            *span = spans.next();
            if let Some(name) = name {
                name.span = spans.next();
            }
            if let Some(sc) = super_class {
                uniqueify_expr_spans(sc, spans);
            }
            for el in body {
                match el {
                    ClassElement::Constructor { params, body, span } => {
                        *span = spans.next();
                        uniqueify_params_spans(params, spans);
                        uniqueify_stmt_spans(body, spans);
                    }
                    ClassElement::Method {
                        key,
                        params,
                        body,
                        span,
                        ..
                    } => {
                        *span = spans.next();
                        uniqueify_object_key_spans(key, spans);
                        uniqueify_params_spans(params, spans);
                        uniqueify_stmt_spans(body, spans);
                    }
                    ClassElement::Accessor {
                        key,
                        params,
                        body,
                        span,
                        ..
                    } => {
                        *span = spans.next();
                        uniqueify_object_key_spans(key, spans);
                        uniqueify_params_spans(params, spans);
                        uniqueify_stmt_spans(body, spans);
                    }
                    ClassElement::Field {
                        key, value, span, ..
                    } => {
                        *span = spans.next();
                        uniqueify_object_key_spans(key, spans);
                        if let Some(v) = value {
                            uniqueify_expr_spans(v, spans);
                        }
                    }
                    ClassElement::StaticBlock { body, span } => {
                        *span = spans.next();
                        uniqueify_stmt_spans(body, spans);
                    }
                }
            }
        }
        Expr::ArrowFunction {
            params, body, span, ..
        } => {
            *span = spans.next();
            uniqueify_params_spans(params, spans);
            match body {
                ArrowBody::Expr(e) => uniqueify_expr_spans(e, spans),
                ArrowBody::Block(b) => uniqueify_stmt_spans(b, spans),
            }
        }
        Expr::ObjectExpression { properties, span } => {
            *span = spans.next();
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key,
                        value,
                        span: prop_span,
                        ..
                    } => {
                        *prop_span = spans.next();
                        match key {
                            ObjectKey::Ident(id) => id.span = spans.next(),
                            ObjectKey::String(s) => s.span = spans.next(),
                            ObjectKey::Computed(e) => uniqueify_expr_spans(e, spans),
                        }
                        uniqueify_expr_spans(value, spans);
                    }
                    ObjectProp::Accessor {
                        key,
                        params,
                        body,
                        span: prop_span,
                        ..
                    } => {
                        *prop_span = spans.next();
                        match key {
                            ObjectKey::Ident(id) => id.span = spans.next(),
                            ObjectKey::String(s) => s.span = spans.next(),
                            ObjectKey::Computed(e) => uniqueify_expr_spans(e, spans),
                        }
                        uniqueify_params_spans(params, spans);
                        uniqueify_stmt_spans(body, spans);
                    }
                    ObjectProp::Spread {
                        expr,
                        span: prop_span,
                    } => {
                        *prop_span = spans.next();
                        uniqueify_expr_spans(expr, spans);
                    }
                }
            }
        }
        Expr::ArrayExpression { elements, span, .. } => {
            *span = spans.next();
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        uniqueify_expr_spans(e, spans)
                    }
                    ArrayElement::Elision => {}
                }
            }
        }
        Expr::ArrayPattern { elements, span } => {
            *span = spans.next();
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        uniqueify_binding_spans(binding, spans);
                        if let Some(def) = default {
                            uniqueify_expr_spans(def, spans);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => uniqueify_binding_spans(binding, spans),
                }
            }
        }
        Expr::ObjectPattern { properties, span } => {
            *span = spans.next();
            for p in properties {
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        default,
                        span: prop_span,
                        ..
                    } => {
                        *prop_span = spans.next();
                        uniqueify_object_key_spans(key, spans);
                        uniqueify_binding_spans(binding, spans);
                        if let Some(def) = default {
                            uniqueify_expr_spans(def, spans);
                        }
                    }
                    ObjectPatternProp::Rest(binding) => uniqueify_binding_spans(binding, spans),
                }
            }
        }
        Expr::MemberExpression {
            object,
            property,
            span,
            ..
        } => {
            *span = spans.next();
            uniqueify_expr_spans(object, spans);
            uniqueify_expr_spans(property, spans);
        }
        Expr::PrivateIn { name, object, span } => {
            *span = spans.next();
            name.span = spans.next();
            uniqueify_expr_spans(object, spans);
        }
    }
}

pub(crate) fn stmt_span_approx(stmt: &Stmt) -> Span {
    match stmt {
        Stmt::Expression { span, .. }
        | Stmt::Let { span, .. }
        | Stmt::Empty { span }
        | Stmt::Block { span, .. }
        | Stmt::If { span, .. }
        | Stmt::While { span, .. }
        | Stmt::DoWhile { span, .. }
        | Stmt::For { span, .. }
        | Stmt::ForIn { span, .. }
        | Stmt::ForOf { span, .. }
        | Stmt::Break { span, .. }
        | Stmt::Continue { span, .. }
        | Stmt::Labeled { span, .. }
        | Stmt::Switch { span, .. }
        | Stmt::FunctionDeclaration { span, .. }
        | Stmt::ClassDeclaration { span, .. }
        | Stmt::Return { span, .. }
        | Stmt::Throw { span, .. }
        | Stmt::Try { span, .. }
        | Stmt::With { span, .. }
        | Stmt::ImportDeclaration { span, .. }
        | Stmt::ExportNamedDeclaration { span, .. }
        | Stmt::ExportDefaultDeclaration { span, .. }
        | Stmt::ExportAllDeclaration { span, .. }
        | Stmt::TypeAlias { span, .. }
        | Stmt::ExternFunctionDeclaration { span, .. } => *span,
    }
}
