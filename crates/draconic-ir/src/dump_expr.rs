use draconic_ast::AccessorKind;

use crate::dump::{
    dump_array_pattern_els, dump_object_pattern_els, dump_pattern, dump_stmt, indent,
};
use crate::{
    Arg, ArrayElement, AssignTarget, Expr, ObjectProp, ObjectPropKey, Param, Pattern, UpdateTarget,
};

pub(crate) fn dump_expr(expr: &Expr, level: usize, out: &mut String) {
    match expr {
        Expr::Local { id, ty } => {
            indent(level, out);
            out.push_str(&format!("Local %{} : {ty}\n", id.0));
        }
        Expr::IdentName { name, ty } => {
            indent(level, out);
            out.push_str(&format!("IdentName {name} : {ty}\n"));
        }
        Expr::Number { raw, ty } => {
            indent(level, out);
            out.push_str(&format!("Number {raw} : {ty}\n"));
        }
        Expr::BigInt { raw, ty } => {
            indent(level, out);
            out.push_str(&format!("BigInt {raw} : {ty}\n"));
        }
        Expr::String { value, ty } => {
            indent(level, out);
            out.push_str(&format!("String {:?} : {ty}\n", value.to_string_lossy()));
        }
        Expr::RegExp { pattern, flags, ty } => {
            indent(level, out);
            out.push_str(&format!("RegExp /{pattern}/{flags} : {ty}\n"));
        }
        Expr::Template {
            quasis,
            expressions,
            ty,
        } => {
            indent(level, out);
            out.push_str(&format!("Template : {ty}\n"));
            for (i, q) in quasis.iter().enumerate() {
                indent(level + 1, out);
                out.push_str(&format!("Quasi {:?}\n", q.to_string_lossy()));
                if i < expressions.len() {
                    dump_expr(&expressions[i], level + 1, out);
                }
            }
        }
        Expr::TaggedTemplate {
            tag,
            quasis,
            expressions,
            ty,
        } => {
            indent(level, out);
            out.push_str(&format!("TaggedTemplate : {ty}\n"));
            indent(level + 1, out);
            out.push_str("tag:\n");
            dump_expr(tag, level + 2, out);
            for (i, q) in quasis.iter().enumerate() {
                indent(level + 1, out);
                out.push_str(&format!("Quasi {:?}\n", q.to_string_lossy()));
                if i < expressions.len() {
                    dump_expr(&expressions[i], level + 1, out);
                }
            }
        }
        Expr::Boolean { value, ty } => {
            indent(level, out);
            out.push_str(&format!("Boolean {value} : {ty}\n"));
        }
        Expr::Null { ty } => {
            indent(level, out);
            out.push_str(&format!("Null : {ty}\n"));
        }
        Expr::This { ty } => {
            indent(level, out);
            out.push_str(&format!("This : {ty}\n"));
        }
        Expr::NewTarget { ty } => {
            indent(level, out);
            out.push_str(&format!("NewTarget : {ty}\n"));
        }
        Expr::ImportMeta { ty } => {
            indent(level, out);
            out.push_str(&format!("ImportMeta : {ty}\n"));
        }
        Expr::ImportCall {
            phase,
            source,
            options,
            ty,
        } => {
            indent(level, out);
            match phase {
                draconic_ast::ImportPhase::Evaluation => {
                    out.push_str(&format!("ImportCall : {ty}\n"))
                }
                draconic_ast::ImportPhase::Defer => {
                    out.push_str(&format!("ImportCall defer : {ty}\n"))
                }
                draconic_ast::ImportPhase::Source => {
                    out.push_str(&format!("ImportCall source : {ty}\n"))
                }
            }
            dump_expr(source, level + 1, out);
            if let Some(opts) = options {
                dump_expr(opts, level + 1, out);
            }
        }
        Expr::Super { ty } => {
            indent(level, out);
            out.push_str(&format!("Super : {ty}\n"));
        }
        Expr::Unary { op, arg, ty } => {
            indent(level, out);
            out.push_str(&format!("Unary {op} : {ty}\n"));
            dump_expr(arg, level + 1, out);
        }
        Expr::Binary {
            left,
            op,
            right,
            ty,
        } => {
            indent(level, out);
            out.push_str(&format!("Binary {op} : {ty}\n"));
            dump_expr(left, level + 1, out);
            dump_expr(right, level + 1, out);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ty,
        } => {
            indent(level, out);
            out.push_str(&format!("Conditional : {ty}\n"));
            dump_expr(test, level + 1, out);
            dump_expr(consequent, level + 1, out);
            dump_expr(alternate, level + 1, out);
        }
        Expr::Assign {
            target,
            op,
            value,
            ty,
        } => {
            indent(level, out);
            match target {
                AssignTarget::Local(id) => {
                    out.push_str(&format!("Assign {op} %{} : {ty}\n", id.0));
                }
                AssignTarget::Name(name) => {
                    out.push_str(&format!("Assign {op} {name} : {ty}\n"));
                }
                AssignTarget::Member {
                    object,
                    property,
                    computed,
                } => {
                    if *computed {
                        out.push_str(&format!("Assign {op} member computed : {ty}\n"));
                    } else {
                        out.push_str(&format!("Assign {op} member : {ty}\n"));
                    }
                    indent(level + 1, out);
                    out.push_str("object:\n");
                    dump_expr(object, level + 2, out);
                    indent(level + 1, out);
                    out.push_str("property:\n");
                    dump_expr(property, level + 2, out);
                }
                AssignTarget::Deref(ptr) => {
                    out.push_str(&format!("Assign {op} deref : {ty}\n"));
                    dump_expr(ptr, level + 1, out);
                }
                AssignTarget::ArrayPattern { elements } => {
                    out.push_str(&format!("Assign {op} ArrayPattern : {ty}\n"));
                    dump_array_pattern_els(elements, level + 1, out);
                }
                AssignTarget::ObjectPattern { properties } => {
                    out.push_str(&format!("Assign {op} ObjectPattern : {ty}\n"));
                    dump_object_pattern_els(properties, level + 1, out);
                }
            }
            dump_expr(value, level + 1, out);
        }
        Expr::Update {
            op,
            target,
            prefix,
            ty,
        } => {
            indent(level, out);
            let kind = if *prefix { "prefix" } else { "postfix" };
            match target {
                UpdateTarget::Local(id) => {
                    out.push_str(&format!("Update {kind} {op} %{} : {ty}\n", id.0));
                }
                UpdateTarget::Name(name) => {
                    out.push_str(&format!("Update {kind} {op} {name} : {ty}\n"));
                }
                UpdateTarget::Member {
                    object,
                    property,
                    computed,
                } => {
                    out.push_str(&format!("Update {kind} {op} Member : {ty}\n"));
                    dump_expr(object, level + 1, out);
                    if *computed {
                        dump_expr(property, level + 1, out);
                    } else {
                        dump_expr(property, level + 1, out);
                    }
                }
            }
        }
        Expr::Call {
            callee,
            args,
            optional,
            ty,
        } => {
            indent(level, out);
            if *optional {
                out.push_str(&format!("Call optional : {ty}\n"));
            } else {
                out.push_str(&format!("Call : {ty}\n"));
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
        Expr::New { callee, args, ty } => {
            indent(level, out);
            out.push_str(&format!("New : {ty}\n"));
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
        Expr::Function {
            name,
            params,
            body,
            is_async,
            is_generator,
            is_arrow,
            is_method,
            ty,
        } => {
            indent(level, out);
            out.push_str(&format!("FunctionExpr : {ty}\n"));
            if *is_async {
                indent(level + 1, out);
                out.push_str("async: true\n");
            }
            if *is_generator {
                indent(level + 1, out);
                out.push_str("generator: true\n");
            }
            if *is_arrow {
                indent(level + 1, out);
                out.push_str("arrow: true\n");
            }
            if *is_method {
                indent(level + 1, out);
                out.push_str("method: true\n");
            }
            if let Some(local) = name {
                indent(level + 1, out);
                out.push_str(&format!("name: %{}\n", local.0));
            }
            dump_params(params, level + 1, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            for s in body {
                dump_stmt(s, level + 2, out);
            }
        }
        Expr::Object { properties, ty } => {
            indent(level, out);
            out.push_str(&format!("Object : {ty}\n"));
            for prop in properties {
                indent(level + 1, out);
                match prop {
                    ObjectProp::Property { key, value } => match key {
                        ObjectPropKey::Static(k) => {
                            out.push_str(&format!("prop {:?}:\n", k.to_string_lossy()));
                            dump_expr(value, level + 2, out);
                        }
                        ObjectPropKey::Computed(k) => {
                            out.push_str("prop computed:\n");
                            indent(level + 2, out);
                            out.push_str("key:\n");
                            dump_expr(k, level + 3, out);
                            indent(level + 2, out);
                            out.push_str("value:\n");
                            dump_expr(value, level + 3, out);
                        }
                    },
                    ObjectProp::Accessor { kind, key, value } => {
                        let kind_s = match kind {
                            AccessorKind::Get => "get",
                            AccessorKind::Set => "set",
                        };
                        match key {
                            ObjectPropKey::Static(k) => {
                                out.push_str(&format!(
                                    "accessor {kind_s} {:?}:\n",
                                    k.to_string_lossy()
                                ));
                                dump_expr(value, level + 2, out);
                            }
                            ObjectPropKey::Computed(k) => {
                                out.push_str(&format!("accessor {kind_s} computed:\n"));
                                indent(level + 2, out);
                                out.push_str("key:\n");
                                dump_expr(k, level + 3, out);
                                indent(level + 2, out);
                                out.push_str("value:\n");
                                dump_expr(value, level + 3, out);
                            }
                        }
                    }
                    ObjectProp::Spread(expr) => {
                        out.push_str("spread:\n");
                        dump_expr(expr, level + 2, out);
                    }
                }
            }
        }
        Expr::Array { elements, ty } => {
            indent(level, out);
            out.push_str(&format!("Array : {ty}\n"));
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
        Expr::Member {
            object,
            property,
            computed,
            optional,
            ty,
        } => {
            indent(level, out);
            match (*optional, *computed) {
                (true, true) => out.push_str(&format!("Member optional computed : {ty}\n")),
                (true, false) => out.push_str(&format!("Member optional : {ty}\n")),
                (false, true) => out.push_str(&format!("Member computed : {ty}\n")),
                (false, false) => out.push_str(&format!("Member : {ty}\n")),
            }
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("property:\n");
            dump_expr(property, level + 2, out);
        }
    }
}

pub(crate) fn dump_params(params: &[Param], level: usize, out: &mut String) {
    if params.is_empty() {
        return;
    }
    indent(level, out);
    out.push_str("params:\n");
    for p in params {
        match (&p.pattern, p.rest) {
            (Pattern::Local(id), true) => {
                indent(level + 1, out);
                out.push_str(&format!("rest %{}\n", id.0));
            }
            (Pattern::Local(id), false) => {
                indent(level + 1, out);
                out.push_str(&format!("%{}\n", id.0));
            }
            (pat, true) => {
                indent(level + 1, out);
                out.push_str("rest:\n");
                dump_pattern(pat, level + 2, out);
            }
            (pat, false) => {
                dump_pattern(pat, level + 1, out);
            }
        }
        if let Some(default) = &p.default {
            indent(level + 2, out);
            out.push_str("default:\n");
            dump_expr(default, level + 3, out);
        }
    }
}
