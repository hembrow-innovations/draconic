use crate::dump::{
    dump_binding_pattern, dump_class_element_key, dump_object_pattern_props, dump_params,
    dump_stmt, dump_type_ann, indent,
};
use crate::{
    AccessorKind, Arg, ArrayElement, ArrayPatternElement, ArrowBody, ClassElement, Expr,
    ImportPhase, ObjectKey, ObjectProp,
};

pub(crate) fn dump_expr(expr: &Expr, level: usize, out: &mut String) {
    match expr {
        Expr::Ident(id) => {
            indent(level, out);
            out.push_str(&format!("Ident {}\n", id.name));
        }
        Expr::Number(n) => {
            indent(level, out);
            out.push_str(&format!("Number {}\n", n.raw));
        }
        Expr::BigInt(n) => {
            indent(level, out);
            out.push_str(&format!("BigInt {}\n", n.raw));
        }
        Expr::String(s) => {
            indent(level, out);
            out.push_str(&format!("String {:?}\n", s.value.to_string_lossy()));
        }
        Expr::RegExp { pattern, flags, .. } => {
            indent(level, out);
            out.push_str(&format!("RegExp /{pattern}/{flags}\n"));
        }
        Expr::TemplateLiteral {
            quasis,
            expressions,
            ..
        } => {
            indent(level, out);
            out.push_str("TemplateLiteral\n");
            for (i, q) in quasis.iter().enumerate() {
                indent(level + 1, out);
                out.push_str(&format!("Quasi {:?}\n", q.cooked.to_string_lossy()));
                if i < expressions.len() {
                    dump_expr(&expressions[i], level + 1, out);
                }
            }
        }
        Expr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            ..
        } => {
            indent(level, out);
            out.push_str("TaggedTemplate\n");
            indent(level + 1, out);
            out.push_str("tag:\n");
            dump_expr(tag, level + 2, out);
            for (i, q) in quasis.iter().enumerate() {
                indent(level + 1, out);
                out.push_str(&format!("Quasi {:?}\n", q.cooked.to_string_lossy()));
                if i < expressions.len() {
                    dump_expr(&expressions[i], level + 1, out);
                }
            }
        }
        Expr::Boolean { value, .. } => {
            indent(level, out);
            out.push_str(&format!("Boolean {value}\n"));
        }
        Expr::Null { .. } => {
            indent(level, out);
            out.push_str("Null\n");
        }
        Expr::This { .. } => {
            indent(level, out);
            out.push_str("This\n");
        }
        Expr::Super { .. } => {
            indent(level, out);
            out.push_str("Super\n");
        }
        Expr::NewTarget { .. } => {
            indent(level, out);
            out.push_str("NewTarget\n");
        }
        Expr::ImportMeta { .. } => {
            indent(level, out);
            out.push_str("ImportMeta\n");
        }
        Expr::ImportCall {
            phase,
            source,
            options,
            ..
        } => {
            indent(level, out);
            match phase {
                ImportPhase::Evaluation => out.push_str("ImportCall\n"),
                ImportPhase::Defer => out.push_str("ImportCall defer\n"),
                ImportPhase::Source => out.push_str("ImportCall source\n"),
            }
            dump_expr(source, level + 1, out);
            if let Some(opts) = options {
                dump_expr(opts, level + 1, out);
            }
        }
        Expr::Unary { op, arg, .. } => {
            indent(level, out);
            out.push_str(&format!("Unary {op}\n"));
            dump_expr(arg, level + 1, out);
        }
        Expr::Binary {
            left, op, right, ..
        } => {
            indent(level, out);
            out.push_str(&format!("Binary {op}\n"));
            dump_expr(left, level + 1, out);
            dump_expr(right, level + 1, out);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            indent(level, out);
            out.push_str("Conditional\n");
            dump_expr(test, level + 1, out);
            dump_expr(consequent, level + 1, out);
            dump_expr(alternate, level + 1, out);
        }
        Expr::Assign {
            target, op, value, ..
        } => {
            indent(level, out);
            out.push_str(&format!("Assign {op}\n"));
            dump_expr(target, level + 1, out);
            dump_expr(value, level + 1, out);
        }
        Expr::Update {
            op, arg, prefix, ..
        } => {
            indent(level, out);
            if *prefix {
                out.push_str(&format!("Update prefix {op}\n"));
            } else {
                out.push_str(&format!("Update postfix {op}\n"));
            }
            dump_expr(arg, level + 1, out);
        }
        Expr::Call {
            callee,
            args,
            optional,
            ..
        } => {
            indent(level, out);
            if *optional {
                out.push_str("Call optional\n");
            } else {
                out.push_str("Call\n");
            }
            indent(level + 1, out);
            out.push_str("callee:\n");
            dump_expr(callee, level + 2, out);
            for (i, arg) in args.iter().enumerate() {
                indent(level + 1, out);
                match arg {
                    Arg::Expr(expr) => {
                        out.push_str(&format!("arg[{i}]:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                    Arg::Spread(expr) => {
                        out.push_str(&format!("arg[{i}] spread:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                }
            }
        }
        Expr::New { callee, args, .. } => {
            indent(level, out);
            out.push_str("New\n");
            indent(level + 1, out);
            out.push_str("callee:\n");
            dump_expr(callee, level + 2, out);
            for (i, arg) in args.iter().enumerate() {
                indent(level + 1, out);
                match arg {
                    Arg::Expr(expr) => {
                        out.push_str(&format!("arg[{i}]:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                    Arg::Spread(expr) => {
                        out.push_str(&format!("arg[{i}] spread:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                }
            }
        }
        Expr::FunctionExpression {
            name,
            params,
            return_type,
            body,
            is_async,
            is_generator,
            is_method,
            ..
        } => {
            indent(level, out);
            out.push_str("FunctionExpression\n");
            if *is_async {
                indent(level + 1, out);
                out.push_str("async: true\n");
            }
            if *is_generator {
                indent(level + 1, out);
                out.push_str("generator: true\n");
            }
            if *is_method {
                indent(level + 1, out);
                out.push_str("method: true\n");
            }
            if let Some(name) = name {
                indent(level + 1, out);
                out.push_str(&format!("name: {}\n", name.name));
            }
            dump_params(params, level + 1, out);
            if let Some(ret) = return_type {
                indent(level + 1, out);
                out.push_str("returnType:\n");
                dump_type_ann(ret, level + 2, out);
            }
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Expr::ClassExpression {
            name,
            super_class,
            body,
            ..
        } => {
            indent(level, out);
            out.push_str("ClassExpression\n");
            if let Some(name) = name {
                indent(level + 1, out);
                out.push_str(&format!("name: {}\n", name.name));
            }
            if let Some(sc) = super_class {
                indent(level + 1, out);
                out.push_str("extends:\n");
                dump_expr(sc, level + 2, out);
            }
            for el in body {
                match el {
                    ClassElement::Constructor { params, body, .. } => {
                        indent(level + 1, out);
                        out.push_str("Constructor\n");
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::Method {
                        key,
                        params,
                        body,
                        is_static,
                        is_async,
                        is_generator,
                        is_private,
                        ..
                    } => {
                        indent(level + 1, out);
                        match (*is_static, *is_private) {
                            (true, true) => out.push_str("StaticPrivateMethod\n"),
                            (true, false) => out.push_str("StaticMethod\n"),
                            (false, true) => out.push_str("PrivateMethod\n"),
                            (false, false) => out.push_str("Method\n"),
                        }
                        dump_class_element_key(key, *is_private, level + 2, out);
                        if *is_async {
                            indent(level + 2, out);
                            out.push_str("async: true\n");
                        }
                        if *is_generator {
                            indent(level + 2, out);
                            out.push_str("generator: true\n");
                        }
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::Accessor {
                        kind,
                        key,
                        params,
                        body,
                        is_static,
                        is_private,
                        ..
                    } => {
                        indent(level + 1, out);
                        let kind_s = match kind {
                            AccessorKind::Get => "get",
                            AccessorKind::Set => "set",
                        };
                        match (*is_static, *is_private) {
                            (true, true) => {
                                out.push_str(&format!("StaticPrivateAccessor {kind_s}\n"))
                            }
                            (true, false) => out.push_str(&format!("StaticAccessor {kind_s}\n")),
                            (false, true) => out.push_str(&format!("PrivateAccessor {kind_s}\n")),
                            (false, false) => out.push_str(&format!("Accessor {kind_s}\n")),
                        }
                        dump_class_element_key(key, *is_private, level + 2, out);
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        indent(level + 1, out);
                        out.push_str("StaticBlock\n");
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ClassElement::Field {
                        key,
                        value,
                        is_static,
                        is_private,
                        ..
                    } => {
                        indent(level + 1, out);
                        match (*is_static, *is_private) {
                            (true, true) => out.push_str("StaticPrivateField\n"),
                            (true, false) => out.push_str("StaticField\n"),
                            (false, true) => out.push_str("PrivateField\n"),
                            (false, false) => out.push_str("Field\n"),
                        }
                        dump_class_element_key(key, *is_private, level + 2, out);
                        if let Some(v) = value {
                            indent(level + 2, out);
                            out.push_str("value:\n");
                            dump_expr(v, level + 3, out);
                        }
                    }
                }
            }
        }
        Expr::ArrowFunction {
            params,
            return_type,
            body,
            is_async,
            ..
        } => {
            indent(level, out);
            out.push_str("ArrowFunction\n");
            if *is_async {
                indent(level + 1, out);
                out.push_str("async: true\n");
            }
            dump_params(params, level + 1, out);
            if let Some(ret) = return_type {
                indent(level + 1, out);
                out.push_str("returnType:\n");
                dump_type_ann(ret, level + 2, out);
            }
            indent(level + 1, out);
            out.push_str("body:\n");
            match body {
                ArrowBody::Expr(expr) => dump_expr(expr, level + 2, out),
                ArrowBody::Block(stmt) => dump_stmt(stmt, level + 2, out),
            }
        }
        Expr::ObjectExpression { properties, .. } => {
            indent(level, out);
            out.push_str("ObjectExpression\n");
            for prop in properties {
                match prop {
                    ObjectProp::Property {
                        key,
                        value,
                        shorthand,
                        ..
                    } => {
                        indent(level + 1, out);
                        if *shorthand {
                            out.push_str("prop shorthand:\n");
                        } else {
                            out.push_str("prop:\n");
                        }
                        indent(level + 2, out);
                        match key {
                            ObjectKey::Ident(id) => {
                                out.push_str(&format!("key: Ident {}\n", id.name))
                            }
                            ObjectKey::String(s) => out.push_str(&format!(
                                "key: String {:?}\n",
                                s.value.to_string_lossy()
                            )),
                            ObjectKey::Computed(expr) => {
                                out.push_str("key: Computed\n");
                                dump_expr(expr, level + 3, out);
                            }
                        }
                        indent(level + 2, out);
                        out.push_str("value:\n");
                        dump_expr(value, level + 3, out);
                    }
                    ObjectProp::Accessor {
                        kind,
                        key,
                        params,
                        body,
                        ..
                    } => {
                        indent(level + 1, out);
                        let kind_s = match kind {
                            AccessorKind::Get => "get",
                            AccessorKind::Set => "set",
                        };
                        out.push_str(&format!("accessor {kind_s}:\n"));
                        indent(level + 2, out);
                        match key {
                            ObjectKey::Ident(id) => {
                                out.push_str(&format!("key: Ident {}\n", id.name))
                            }
                            ObjectKey::String(s) => out.push_str(&format!(
                                "key: String {:?}\n",
                                s.value.to_string_lossy()
                            )),
                            ObjectKey::Computed(expr) => {
                                out.push_str("key: Computed\n");
                                dump_expr(expr, level + 3, out);
                            }
                        }
                        dump_params(params, level + 2, out);
                        indent(level + 2, out);
                        out.push_str("body:\n");
                        dump_stmt(body, level + 3, out);
                    }
                    ObjectProp::Spread { expr, .. } => {
                        indent(level + 1, out);
                        out.push_str("spread:\n");
                        dump_expr(expr, level + 2, out);
                    }
                }
            }
        }
        Expr::ArrayExpression { elements, .. } => {
            indent(level, out);
            out.push_str("ArrayExpression\n");
            for (i, el) in elements.iter().enumerate() {
                indent(level + 1, out);
                match el {
                    ArrayElement::Expr(expr) => {
                        out.push_str(&format!("element[{i}]:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                    ArrayElement::Spread(expr) => {
                        out.push_str(&format!("element[{i}] spread:\n"));
                        dump_expr(expr, level + 2, out);
                    }
                    ArrayElement::Elision => {
                        out.push_str(&format!("element[{i}] elision\n"));
                    }
                }
            }
        }
        Expr::MemberExpression {
            object,
            property,
            computed,
            optional,
            private,
            ..
        } => {
            indent(level, out);
            if *private {
                match *optional {
                    true => out.push_str("PrivateMemberExpression optional\n"),
                    false => out.push_str("PrivateMemberExpression\n"),
                }
            } else {
                match (*optional, *computed) {
                    (true, true) => out.push_str("MemberExpression optional computed\n"),
                    (true, false) => out.push_str("MemberExpression optional\n"),
                    (false, true) => out.push_str("MemberExpression computed\n"),
                    (false, false) => out.push_str("MemberExpression\n"),
                }
            }
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("property:\n");
            dump_expr(property, level + 2, out);
        }
        Expr::PrivateIn { name, object, .. } => {
            indent(level, out);
            out.push_str("PrivateIn\n");
            indent(level + 1, out);
            out.push_str(&format!("name: #{}\n", name.name));
            dump_expr(object, level + 1, out);
        }
        Expr::Paren { expr, .. } => {
            indent(level, out);
            out.push_str("Paren\n");
            dump_expr(expr, level + 1, out);
        }
        Expr::As { expr, ty, .. } => {
            indent(level, out);
            out.push_str("As\n");
            dump_expr(expr, level + 1, out);
            indent(level + 1, out);
            out.push_str("type:\n");
            dump_type_ann(ty, level + 2, out);
        }
        Expr::ArrayPattern { elements, .. } => {
            indent(level, out);
            out.push_str("ArrayPattern\n");
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {
                        indent(level + 1, out);
                        out.push_str("elision\n");
                    }
                    ArrayPatternElement::Pattern { binding, default } => {
                        dump_binding_pattern(binding, level + 1, out);
                        if let Some(def) = default {
                            indent(level + 1, out);
                            out.push_str("default:\n");
                            dump_expr(def, level + 2, out);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        indent(level + 1, out);
                        out.push_str("rest:\n");
                        dump_binding_pattern(binding, level + 2, out);
                    }
                }
            }
        }
        Expr::ObjectPattern { properties, .. } => {
            indent(level, out);
            out.push_str("ObjectPattern\n");
            dump_object_pattern_props(properties, level + 1, out);
        }
    }
}
