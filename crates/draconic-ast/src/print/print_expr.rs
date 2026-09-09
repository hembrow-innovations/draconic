use std::fmt::Write as _;

use crate::print::{
    print_binding_pattern, print_class_body, print_function_body, print_param_list,
};
use crate::print::print_type::print_type_ann;
use crate::{
    AccessorKind, Arg, ArrayElement, ArrayPatternElement, ArrowBody, BinaryOp, BindingPattern,
    Expr, ImportPhase, ObjectKey, ObjectPatternProp, ObjectProp, UnaryOp,
};

/// Precedence levels (higher = tighter). Used for minimal parentheses.
const PREC_COMMA: u8 = 1;
pub(crate) const PREC_ASSIGN: u8 = 2;
const PREC_COND: u8 = 3;
const PREC_NULLISH: u8 = 4;
const PREC_OR: u8 = 5;
const PREC_AND: u8 = 6;
const PREC_BIT_OR: u8 = 7;
const PREC_BIT_XOR: u8 = 8;
const PREC_BIT_AND: u8 = 9;
const PREC_EQ: u8 = 10;
const PREC_REL: u8 = 11;
const PREC_SHIFT: u8 = 12;
const PREC_ADD: u8 = 13;
const PREC_MUL: u8 = 14;
const PREC_POW: u8 = 15;
const PREC_UNARY: u8 = 16;
const PREC_UPDATE: u8 = 17;
const PREC_CALL: u8 = 18;
const PREC_MEMBER: u8 = 19;
const PREC_PRIMARY: u8 = 20;

fn binary_prec(op: BinaryOp) -> u8 {
    match op {
        BinaryOp::Comma => PREC_COMMA,
        BinaryOp::Nullish => PREC_NULLISH,
        BinaryOp::Or => PREC_OR,
        BinaryOp::And => PREC_AND,
        BinaryOp::BitOr => PREC_BIT_OR,
        BinaryOp::BitXor => PREC_BIT_XOR,
        BinaryOp::BitAnd => PREC_BIT_AND,
        BinaryOp::EqEq | BinaryOp::NotEq | BinaryOp::EqEqEq | BinaryOp::NotEqEq => PREC_EQ,
        BinaryOp::Lt
        | BinaryOp::LtEq
        | BinaryOp::Gt
        | BinaryOp::GtEq
        | BinaryOp::In
        | BinaryOp::InstanceOf => PREC_REL,
        BinaryOp::Shl | BinaryOp::Shr | BinaryOp::UShr => PREC_SHIFT,
        BinaryOp::Add | BinaryOp::Sub => PREC_ADD,
        BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => PREC_MUL,
        BinaryOp::Pow => PREC_POW,
    }
}

fn binary_right_assoc(op: BinaryOp) -> bool {
    matches!(op, BinaryOp::Pow)
}

pub(crate) fn print_expr(expr: &Expr, parent_prec: u8, out: &mut String) {
    print_expr_inner(expr, parent_prec, out);
}

pub(crate) fn print_expr_inner(expr: &Expr, parent_prec: u8, out: &mut String) {
    match expr {
        Expr::Ident(id) => out.push_str(&id.name),
        Expr::Number(n) => out.push_str(&n.raw),
        Expr::BigInt(n) => out.push_str(&n.raw),
        Expr::String(s) => print_string_lit(&s.value, out),
        Expr::RegExp { pattern, flags, .. } => {
            out.push('/');
            out.push_str(pattern);
            out.push('/');
            out.push_str(flags);
        }
        Expr::TemplateLiteral {
            quasis,
            expressions,
            ..
        } => print_template(quasis, expressions, out),
        Expr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            ..
        } => {
            match tag.as_ref() {
                Expr::MemberExpression { .. } | Expr::Call { .. } | Expr::Ident(_) => {
                    print_expr_inner(tag, PREC_MEMBER, out);
                }
                _ => {
                    out.push('(');
                    print_expr_inner(tag, 0, out);
                    out.push(')');
                }
            }
            print_template(quasis, expressions, out);
        }
        Expr::Boolean { value, .. } => out.push_str(if *value { "true" } else { "false" }),
        Expr::Null { .. } => out.push_str("null"),
        Expr::This { .. } => out.push_str("this"),
        Expr::Super { .. } => out.push_str("super"),
        Expr::NewTarget { .. } => out.push_str("new.target"),
        Expr::ImportMeta { .. } => out.push_str("import.meta"),
        Expr::ImportCall {
            phase,
            source,
            options,
            ..
        } => {
            match phase {
                ImportPhase::Evaluation => out.push_str("import("),
                ImportPhase::Defer => out.push_str("import.defer("),
                ImportPhase::Source => out.push_str("import.source("),
            }
            print_expr_inner(source, 0, out);
            if let Some(opts) = options {
                out.push_str(", ");
                print_expr_inner(opts, 0, out);
            }
            out.push(')');
        }
        Expr::Unary { op, arg, .. } => {
            let wrap = PREC_UNARY < parent_prec;
            if wrap {
                out.push('(');
            }
            match op {
                UnaryOp::TypeOf
                | UnaryOp::Void
                | UnaryOp::Delete
                | UnaryOp::Await
                | UnaryOp::Yield => {
                    let _ = write!(out, "{op} ");
                }
                UnaryOp::YieldStar => out.push_str("yield* "),
                _ => {
                    let _ = write!(out, "{op}");
                }
            }
            print_expr_inner(arg, PREC_UNARY + 1, out);
            if wrap {
                out.push(')');
            }
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            let p = binary_prec(*op);
            let wrap = p < parent_prec;
            if wrap {
                out.push('(');
            }
            let left_min = if binary_right_assoc(*op) { p + 1 } else { p };
            print_expr_inner(left, left_min, out);
            out.push(' ');
            let _ = write!(out, "{op}");
            out.push(' ');
            let right_min = if binary_right_assoc(*op) { p } else { p + 1 };
            print_expr_inner(right, right_min, out);
            if wrap {
                out.push(')');
            }
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            let wrap = PREC_COND < parent_prec;
            if wrap {
                out.push('(');
            }
            print_expr_inner(test, PREC_COND + 1, out);
            out.push_str(" ? ");
            print_expr_inner(consequent, 0, out);
            out.push_str(" : ");
            print_expr_inner(alternate, PREC_COND, out);
            if wrap {
                out.push(')');
            }
        }
        Expr::Assign {
            target, op, value, ..
        } => {
            let wrap = PREC_ASSIGN < parent_prec;
            if wrap {
                out.push('(');
            }
            print_expr_inner(target, PREC_ASSIGN + 1, out);
            out.push(' ');
            let _ = write!(out, "{op}");
            out.push(' ');
            print_expr_inner(value, PREC_ASSIGN, out);
            if wrap {
                out.push(')');
            }
        }
        Expr::Update {
            op, arg, prefix, ..
        } => {
            let p = if *prefix { PREC_UNARY } else { PREC_UPDATE };
            let wrap = p < parent_prec;
            if wrap {
                out.push('(');
            }
            if *prefix {
                let _ = write!(out, "{op}");
                print_expr_inner(arg, PREC_UNARY + 1, out);
            } else {
                print_expr_inner(arg, PREC_UPDATE + 1, out);
                let _ = write!(out, "{op}");
            }
            if wrap {
                out.push(')');
            }
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            let wrap = PREC_CALL < parent_prec;
            if wrap {
                out.push('(');
            }
            print_expr_inner(callee, PREC_CALL, out);
            if *optional {
                out.push_str("?.(");
            } else {
                out.push('(');
            }
            print_args(args, out);
            out.push(')');
            if wrap {
                out.push(')');
            }
        }
        Expr::New { callee, args, .. } => {
            let wrap = PREC_CALL < parent_prec;
            if wrap {
                out.push('(');
            }
            out.push_str("new ");
            print_expr_inner(callee, PREC_MEMBER, out);
            out.push('(');
            print_args(args, out);
            out.push(')');
            if wrap {
                out.push(')');
            }
        }
        Expr::FunctionExpression {
            name,
            params,
            return_type,
            body,
            is_async,
            is_generator,
            is_method: _,
            ..
        } => {
            if *is_async {
                out.push_str("async ");
            }
            out.push_str("function");
            if *is_generator {
                out.push('*');
            }
            if let Some(n) = name {
                out.push(' ');
                out.push_str(&n.name);
            }
            print_param_list(params, out);
            if let Some(ret) = return_type {
                out.push_str(": ");
                print_type_ann(ret, out);
            }
            out.push(' ');
            print_function_body(body, 0, out);
        }
        Expr::ClassExpression {
            name,
            super_class,
            body,
            ..
        } => {
            out.push_str("class");
            if let Some(n) = name {
                out.push(' ');
                out.push_str(&n.name);
            }
            if let Some(sup) = super_class {
                out.push_str(" extends ");
                print_expr_inner(sup, PREC_MEMBER, out);
            }
            out.push(' ');
            print_class_body(body, 0, out);
        }
        Expr::ArrowFunction {
            params,
            return_type,
            body,
            is_async,
            ..
        } => {
            // Arrows associate like assignment; wrap when nested in tighter contexts.
            let wrap = parent_prec > PREC_ASSIGN;
            if wrap {
                out.push('(');
            }
            if *is_async {
                out.push_str("async ");
            }
            let bare = params.len() == 1
                && !params[0].rest
                && params[0].default.is_none()
                && params[0].type_ann.is_none()
                && matches!(params[0].binding, BindingPattern::Ident(_))
                && return_type.is_none();
            if bare {
                if let BindingPattern::Ident(id) = &params[0].binding {
                    out.push_str(&id.name);
                }
            } else {
                print_param_list(params, out);
                if let Some(ret) = return_type {
                    out.push_str(": ");
                    print_type_ann(ret, out);
                }
            }
            out.push_str(" => ");
            match body {
                ArrowBody::Expr(e) => {
                    if matches!(e.as_ref(), Expr::ObjectExpression { .. }) {
                        out.push('(');
                        print_expr_inner(e, 0, out);
                        out.push(')');
                    } else {
                        print_expr_inner(e, PREC_ASSIGN, out);
                    }
                }
                ArrowBody::Block(b) => print_function_body(b, 0, out),
            }
            if wrap {
                out.push(')');
            }
        }
        Expr::ObjectExpression { properties, .. } => {
            if properties.is_empty() {
                out.push_str("{}");
            } else {
                out.push_str("{ ");
                for (i, p) in properties.iter().enumerate() {
                    if i > 0 {
                        out.push_str(", ");
                    }
                    print_object_prop(p, out);
                }
                out.push_str(" }");
            }
        }
        Expr::ArrayExpression {
            elements,
            trailing_comma,
            ..
        } => {
            out.push('[');
            for (i, el) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match el {
                    ArrayElement::Expr(e) => print_expr_inner(e, 0, out),
                    ArrayElement::Spread(e) => {
                        out.push_str("...");
                        print_expr_inner(e, 0, out);
                    }
                    ArrayElement::Elision => {}
                }
            }
            if *trailing_comma && !elements.is_empty() {
                out.push(',');
            }
            out.push(']');
        }
        Expr::MemberExpression {
            object,
            property,
            computed,
            optional,
            private,
            ..
        } => {
            let wrap = PREC_MEMBER < parent_prec;
            if wrap {
                out.push('(');
            }
            print_expr_inner(object, PREC_MEMBER, out);
            if *computed {
                if *optional {
                    out.push_str("?.[");
                } else {
                    out.push('[');
                }
                print_expr_inner(property, 0, out);
                out.push(']');
            } else {
                if *optional {
                    out.push_str("?.");
                } else {
                    out.push('.');
                }
                if *private {
                    out.push('#');
                }
                if let Expr::Ident(id) = property.as_ref() {
                    out.push_str(&id.name);
                } else {
                    print_expr_inner(property, PREC_PRIMARY, out);
                }
            }
            if wrap {
                out.push(')');
            }
        }
        Expr::PrivateIn { name, object, .. } => {
            let wrap = PREC_REL < parent_prec;
            if wrap {
                out.push('(');
            }
            out.push('#');
            out.push_str(&name.name);
            out.push_str(" in ");
            print_expr_inner(object, PREC_REL + 1, out);
            if wrap {
                out.push(')');
            }
        }
        Expr::Paren { expr, .. } => {
            // Handled at top — unreachable if we always peel. Keep as force-group.
            out.push('(');
            print_expr_inner(expr, 0, out);
            out.push(')');
        }
        Expr::As { expr, ty, .. } => {
            let wrap = PREC_UNARY < parent_prec;
            if wrap {
                out.push('(');
            }
            print_expr_inner(expr, PREC_UNARY + 1, out);
            out.push_str(" as ");
            print_type_ann(ty, out);
            if wrap {
                out.push(')');
            }
        }
        Expr::ArrayPattern { elements, .. } => {
            out.push('[');
            for (i, el) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        print_binding_pattern(binding, out);
                        if let Some(d) = default {
                            out.push_str(" = ");
                            print_expr_inner(d, 0, out);
                        }
                    }
                    ArrayPatternElement::Rest(b) => {
                        out.push_str("...");
                        print_binding_pattern(b, out);
                    }
                }
            }
            out.push(']');
        }
        Expr::ObjectPattern { properties, .. } => {
            out.push_str("{ ");
            for (i, p) in properties.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        shorthand,
                        default,
                        ..
                    } => {
                        if *shorthand {
                            print_binding_pattern(binding, out);
                            if let Some(d) = default {
                                out.push_str(" = ");
                                print_expr_inner(d, 0, out);
                            }
                        } else {
                            print_object_key(key, out);
                            out.push_str(": ");
                            print_binding_pattern(binding, out);
                            if let Some(d) = default {
                                out.push_str(" = ");
                                print_expr_inner(d, 0, out);
                            }
                        }
                    }
                    ObjectPatternProp::Rest(b) => {
                        out.push_str("...");
                        print_binding_pattern(b, out);
                    }
                }
            }
            out.push_str(" }");
        }
    }
}

fn print_args(args: &[Arg], out: &mut String) {
    for (i, a) in args.iter().enumerate() {
        if i > 0 {
            out.push_str(", ");
        }
        match a {
            Arg::Expr(e) => print_expr_inner(e, 0, out),
            Arg::Spread(e) => {
                out.push_str("...");
                print_expr_inner(e, 0, out);
            }
        }
    }
}

pub(crate) fn print_object_key(key: &ObjectKey, out: &mut String) {
    match key {
        ObjectKey::Ident(id) => out.push_str(&id.name),
        ObjectKey::String(s) => print_string_lit(&s.value, out),
        ObjectKey::Computed(e) => {
            out.push('[');
            print_expr_inner(e, 0, out);
            out.push(']');
        }
    }
}

fn print_object_prop(prop: &ObjectProp, out: &mut String) {
    match prop {
        ObjectProp::Property {
            key,
            value,
            shorthand,
            ..
        } => {
            if *shorthand {
                print_expr_inner(value, 0, out);
            } else if let Expr::FunctionExpression {
                name: _,
                params,
                return_type,
                body,
                is_async,
                is_generator,
                is_method: true,
                ..
            } = value
            {
                if *is_async {
                    out.push_str("async ");
                }
                if *is_generator {
                    out.push('*');
                }
                print_object_key(key, out);
                print_param_list(params, out);
                if let Some(ret) = return_type {
                    out.push_str(": ");
                    print_type_ann(ret, out);
                }
                out.push(' ');
                print_function_body(body, 0, out);
            } else {
                print_object_key(key, out);
                out.push_str(": ");
                print_expr_inner(value, 0, out);
            }
        }
        ObjectProp::Accessor {
            kind,
            key,
            params,
            body,
            ..
        } => {
            match kind {
                AccessorKind::Get => out.push_str("get "),
                AccessorKind::Set => out.push_str("set "),
            }
            print_object_key(key, out);
            print_param_list(params, out);
            out.push(' ');
            print_function_body(body, 0, out);
        }
        ObjectProp::Spread { expr, .. } => {
            out.push_str("...");
            print_expr_inner(expr, 0, out);
        }
    }
}

fn print_template(quasis: &[crate::TemplateElement], expressions: &[Expr], out: &mut String) {
    out.push('`');
    for (i, q) in quasis.iter().enumerate() {
        print_template_chars(&q.cooked, out);
        if i < expressions.len() {
            out.push_str("${");
            print_expr_inner(&expressions[i], 0, out);
            out.push('}');
        }
    }
    out.push('`');
}

pub(crate) fn print_string_lit(value: &crate::JsString, out: &mut String) {
    out.push('"');
    print_js_string_units(out, value.units());
    out.push('"');
}

fn print_js_string_units(out: &mut String, units: &[u16]) {
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        match u {
            0x5C => out.push_str("\\\\"),
            0x22 => out.push_str("\\\""),
            0x0A => out.push_str("\\n"),
            0x0D => out.push_str("\\r"),
            0x09 => out.push_str("\\t"),
            0x2028 => out.push_str("\\u2028"),
            0x2029 => out.push_str("\\u2029"),
            u if u < 0x20 || (0xD800..=0xDFFF).contains(&u) => {
                let _ = write!(out, "\\u{u:04x}");
            }
            u if u < 0x80 => out.push(u as u8 as char),
            u => {
                if let Some(c) = char::from_u32(u as u32) {
                    out.push(c);
                } else {
                    let _ = write!(out, "\\u{u:04x}");
                }
            }
        }
        i += 1;
    }
}

fn print_template_chars(value: &crate::JsString, out: &mut String) {
    let units = value.units();
    let mut i = 0;
    while i < units.len() {
        let u = units[i];
        match u {
            0x5C => out.push_str("\\\\"),
            0x60 => out.push_str("\\`"),
            0x24 => {
                if units.get(i + 1) == Some(&0x7B) {
                    out.push_str("\\$");
                } else {
                    out.push('$');
                }
            }
            0x0D => out.push_str("\\r"),
            0x2028 => out.push_str("\\u2028"),
            0x2029 => out.push_str("\\u2029"),
            u if (0xD800..=0xDFFF).contains(&u) => {
                let _ = write!(out, "\\u{u:04x}");
            }
            u if u < 0x20 && u != 0x0A && u != 0x09 => {
                let _ = write!(out, "\\u{u:04x}");
            }
            u if u < 0x80 => out.push(u as u8 as char),
            u => {
                if let Some(c) = char::from_u32(u as u32) {
                    out.push(c);
                } else {
                    let _ = write!(out, "\\u{u:04x}");
                }
            }
        }
        i += 1;
    }
}

pub(crate) fn expr_needs_stmt_paren(expr: &Expr) -> bool {
    match expr {
        Expr::Paren { expr, .. } => expr_needs_stmt_paren(expr),
        Expr::ObjectExpression { .. }
        | Expr::FunctionExpression { .. }
        | Expr::ClassExpression { .. } => true,
        _ => false,
    }
}
