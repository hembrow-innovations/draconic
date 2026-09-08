use crate::dump_expr::{dump_expr, dump_params};
use crate::{
    ArrayPatternEl, AssignTarget, BindingKind, Module, ObjectPatternEl, ObjectPropKey, Pattern,
    Stmt,
};

pub(crate) fn dump_module(module: &Module) -> String {
    let mut out = String::new();
    out.push_str("Module\n");
    if !module.locals.is_empty() {
        out.push_str("  locals:\n");
        for local in &module.locals {
            out.push_str(&format!(
                "    %{} {}: {}\n",
                local.id.0, local.name, local.ty
            ));
        }
    }
    out.push_str("  body:\n");
    for stmt in &module.body {
        dump_stmt(stmt, 2, &mut out);
    }
    out
}

pub(crate) fn indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

pub(crate) fn dump_assign_target(target: &AssignTarget, level: usize, out: &mut String) {
    match target {
        AssignTarget::Local(id) => {
            indent(level, out);
            out.push_str(&format!("Local %{}\n", id.0));
        }
        AssignTarget::Name(name) => {
            indent(level, out);
            out.push_str(&format!("Name {name}\n"));
        }
        AssignTarget::Member {
            object,
            property,
            computed,
        } => {
            indent(level, out);
            if *computed {
                out.push_str("Member computed\n");
            } else {
                out.push_str("Member\n");
            }
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("property:\n");
            dump_expr(property, level + 2, out);
        }
        AssignTarget::Deref(ptr) => {
            indent(level, out);
            out.push_str("Deref\n");
            dump_expr(ptr, level + 1, out);
        }
        AssignTarget::ArrayPattern { elements } => {
            indent(level, out);
            out.push_str("ArrayPattern\n");
            dump_array_pattern_els(elements, level + 1, out);
        }
        AssignTarget::ObjectPattern { properties } => {
            indent(level, out);
            out.push_str("ObjectPattern\n");
            dump_object_pattern_els(properties, level + 1, out);
        }
    }
}

pub(crate) fn dump_array_pattern_els(elements: &[ArrayPatternEl], level: usize, out: &mut String) {
    for el in elements {
        match el {
            ArrayPatternEl::Elision => {
                indent(level, out);
                out.push_str("elision\n");
            }
            ArrayPatternEl::Pattern { binding, default } => {
                dump_pattern(binding, level, out);
                if let Some(def) = default {
                    indent(level, out);
                    out.push_str("default:\n");
                    dump_expr(def, level + 1, out);
                }
            }
            ArrayPatternEl::Rest(pat) => {
                indent(level, out);
                out.push_str("rest:\n");
                dump_pattern(pat, level + 1, out);
            }
        }
    }
}

pub(crate) fn dump_object_pattern_els(
    properties: &[ObjectPatternEl],
    level: usize,
    out: &mut String,
) {
    for p in properties {
        match p {
            ObjectPatternEl::Prop {
                key,
                binding,
                shorthand,
                default,
            } => {
                indent(level, out);
                match key {
                    ObjectPropKey::Static(k) => {
                        let name = k.to_string_lossy();
                        if *shorthand {
                            out.push_str(&format!("prop shorthand {name}:\n"));
                        } else {
                            out.push_str(&format!("prop {name}:\n"));
                        }
                    }
                    ObjectPropKey::Computed(e) => {
                        if *shorthand {
                            out.push_str("prop shorthand Computed:\n");
                        } else {
                            out.push_str("prop Computed:\n");
                        }
                        dump_expr(e, level + 1, out);
                    }
                }
                dump_pattern(binding, level + 1, out);
                if let Some(def) = default {
                    indent(level + 1, out);
                    out.push_str("default:\n");
                    dump_expr(def, level + 2, out);
                }
            }
            ObjectPatternEl::Rest(pat) => {
                indent(level, out);
                out.push_str("rest:\n");
                dump_pattern(pat, level + 1, out);
            }
        }
    }
}

pub(crate) fn dump_pattern_inline(pat: &Pattern, out: &mut String) {
    match pat {
        Pattern::Local(id) => out.push_str(&format!("%{}", id.0)),
        Pattern::Name(name) => out.push_str(name),
        Pattern::Member { .. } => out.push_str("<member>"),
        Pattern::Array(els) => {
            out.push('[');
            for (i, el) in els.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match el {
                    ArrayPatternEl::Elision => {}
                    ArrayPatternEl::Pattern { binding, default } => {
                        dump_pattern_inline(binding, out);
                        if default.is_some() {
                            out.push_str(" = …");
                        }
                    }
                    ArrayPatternEl::Rest(p) => {
                        out.push_str("...");
                        dump_pattern_inline(p, out);
                    }
                }
            }
            out.push(']');
        }
        Pattern::Object(props) => {
            out.push('{');
            for (i, p) in props.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match p {
                    ObjectPatternEl::Prop {
                        key,
                        binding,
                        shorthand,
                        default,
                    } => {
                        match key {
                            ObjectPropKey::Static(k) => out.push_str(&k.to_string_lossy()),
                            ObjectPropKey::Computed(_) => out.push_str("[…]"),
                        }
                        if !*shorthand {
                            out.push_str(": ");
                            dump_pattern_inline(binding, out);
                        }
                        if default.is_some() {
                            out.push_str(" = …");
                        }
                    }
                    ObjectPatternEl::Rest(p) => {
                        out.push_str("...");
                        dump_pattern_inline(p, out);
                    }
                }
            }
            out.push('}');
        }
    }
}

pub(crate) fn dump_pattern(pat: &Pattern, level: usize, out: &mut String) {
    match pat {
        Pattern::Local(id) => {
            indent(level, out);
            out.push_str(&format!("local %{}\n", id.0));
        }
        Pattern::Name(name) => {
            indent(level, out);
            out.push_str(&format!("name {name}\n"));
        }
        Pattern::Member {
            object,
            property,
            computed,
        } => {
            indent(level, out);
            out.push_str(&format!("Member computed={computed}\n"));
            dump_expr(object, level + 1, out);
            dump_expr(property, level + 1, out);
        }
        Pattern::Array(els) => {
            indent(level, out);
            out.push_str("ArrayPattern\n");
            dump_array_pattern_els(els, level + 1, out);
        }
        Pattern::Object(props) => {
            indent(level, out);
            out.push_str("ObjectPattern\n");
            dump_object_pattern_els(props, level + 1, out);
        }
    }
}

pub(crate) fn dump_stmt(stmt: &Stmt, level: usize, out: &mut String) {
    match stmt {
        Stmt::Declare { local, init, kind } => {
            indent(level, out);
            let kw = match kind {
                BindingKind::Let => "let",
                BindingKind::Const => "const",
                BindingKind::Var => "var",
                BindingKind::Function => "function",
                BindingKind::Using => "using",
                BindingKind::AwaitUsing => "await using",
            };
            out.push_str(&format!("Declare {kw} %{}\n", local.0));
            if let Some(init) = init {
                indent(level + 1, out);
                out.push_str("init:\n");
                dump_expr(init, level + 2, out);
            }
        }
        Stmt::DeclareArrayPattern {
            kind,
            elements,
            init,
        } => {
            indent(level, out);
            let kw = match kind {
                BindingKind::Let => "let",
                BindingKind::Const => "const",
                BindingKind::Var => "var",
                BindingKind::Function => "function",
                BindingKind::Using => "using",
                BindingKind::AwaitUsing => "await using",
            };
            out.push_str(&format!("DeclareArrayPattern {kw}\n"));
            dump_array_pattern_els(elements, level + 1, out);
            if let Some(init) = init {
                indent(level + 1, out);
                out.push_str("init:\n");
                dump_expr(init, level + 2, out);
            }
        }
        Stmt::DeclareObjectPattern {
            kind,
            properties,
            init,
        } => {
            indent(level, out);
            let kw = match kind {
                BindingKind::Let => "let",
                BindingKind::Const => "const",
                BindingKind::Var => "var",
                BindingKind::Function => "function",
                BindingKind::Using => "using",
                BindingKind::AwaitUsing => "await using",
            };
            out.push_str(&format!("DeclareObjectPattern {kw}\n"));
            dump_object_pattern_els(properties, level + 1, out);
            if let Some(init) = init {
                indent(level + 1, out);
                out.push_str("init:\n");
                dump_expr(init, level + 2, out);
            }
        }
        Stmt::AssignLeft { target } => {
            indent(level, out);
            out.push_str("AssignLeft\n");
            dump_assign_target(target, level + 1, out);
        }
        Stmt::Expr { expr } => {
            indent(level, out);
            out.push_str("Expr\n");
            dump_expr(expr, level + 1, out);
        }
        Stmt::Block { body } => {
            indent(level, out);
            out.push_str("Block\n");
            for s in body {
                dump_stmt(s, level + 1, out);
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
        } => {
            indent(level, out);
            out.push_str("If\n");
            indent(level + 1, out);
            out.push_str("test:\n");
            dump_expr(test, level + 2, out);
            indent(level + 1, out);
            out.push_str("consequent:\n");
            dump_stmt(consequent, level + 2, out);
            if let Some(alt) = alternate {
                indent(level + 1, out);
                out.push_str("alternate:\n");
                dump_stmt(alt, level + 2, out);
            }
        }
        Stmt::While { test, body } => {
            indent(level, out);
            out.push_str("While\n");
            indent(level + 1, out);
            out.push_str("test:\n");
            dump_expr(test, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::DoWhile { body, test } => {
            indent(level, out);
            out.push_str("DoWhile\n");
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
            indent(level + 1, out);
            out.push_str("test:\n");
            dump_expr(test, level + 2, out);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
        } => {
            indent(level, out);
            out.push_str("For\n");
            if let Some(init) = init {
                indent(level + 1, out);
                out.push_str("init:\n");
                dump_stmt(init, level + 2, out);
            }
            if let Some(test) = test {
                indent(level + 1, out);
                out.push_str("test:\n");
                dump_expr(test, level + 2, out);
            }
            if let Some(update) = update {
                indent(level + 1, out);
                out.push_str("update:\n");
                dump_expr(update, level + 2, out);
            }
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::ForIn { left, right, body } => {
            indent(level, out);
            out.push_str("ForIn\n");
            indent(level + 1, out);
            out.push_str("left:\n");
            dump_stmt(left, level + 2, out);
            indent(level + 1, out);
            out.push_str("right:\n");
            dump_expr(right, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
        } => {
            indent(level, out);
            if *is_await {
                out.push_str("ForOf await\n");
            } else {
                out.push_str("ForOf\n");
            }
            indent(level + 1, out);
            out.push_str("left:\n");
            dump_stmt(left, level + 2, out);
            indent(level + 1, out);
            out.push_str("right:\n");
            dump_expr(right, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::Break { label } => {
            indent(level, out);
            if let Some(label) = label {
                out.push_str(&format!("Break {label}\n"));
            } else {
                out.push_str("Break\n");
            }
        }
        Stmt::Continue { label } => {
            indent(level, out);
            if let Some(label) = label {
                out.push_str(&format!("Continue {label}\n"));
            } else {
                out.push_str("Continue\n");
            }
        }
        Stmt::Labeled { label, body } => {
            indent(level, out);
            out.push_str(&format!("Labeled {label}\n"));
            dump_stmt(body, level + 1, out);
        }
        Stmt::Switch {
            discriminant,
            cases,
        } => {
            indent(level, out);
            out.push_str("Switch\n");
            indent(level + 1, out);
            out.push_str("discriminant:\n");
            dump_expr(discriminant, level + 2, out);
            for case in cases {
                indent(level + 1, out);
                if let Some(test) = &case.test {
                    out.push_str("Case\n");
                    indent(level + 2, out);
                    out.push_str("test:\n");
                    dump_expr(test, level + 3, out);
                } else {
                    out.push_str("Default\n");
                }
                for s in &case.body {
                    dump_stmt(s, level + 2, out);
                }
            }
        }
        Stmt::Function {
            local,
            params,
            body,
            is_async,
            is_generator,
        } => {
            indent(level, out);
            out.push_str(&format!("Function %{}\n", local.0));
            if *is_async {
                indent(level + 1, out);
                out.push_str("async: true\n");
            }
            if *is_generator {
                indent(level + 1, out);
                out.push_str("generator: true\n");
            }
            dump_params(params, level + 1, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            for s in body {
                dump_stmt(s, level + 2, out);
            }
        }
        Stmt::ExternFunction {
            local,
            abi,
            name,
            params,
            ret,
        } => {
            indent(level, out);
            out.push_str(&format!(
                "ExternFunction %{} abi={abi:?} name={name}\n",
                local.0
            ));
            indent(level + 1, out);
            out.push_str("params:\n");
            for (i, ty) in params.iter().enumerate() {
                indent(level + 2, out);
                out.push_str(&format!("[{i}] {ty}\n"));
            }
            indent(level + 1, out);
            match ret {
                Some(ty) => out.push_str(&format!("ret: {ty}\n")),
                None => out.push_str("ret: void\n"),
            }
        }
        Stmt::Return { value } => {
            indent(level, out);
            out.push_str("Return\n");
            if let Some(value) = value {
                dump_expr(value, level + 1, out);
            }
        }
        Stmt::Throw { value } => {
            indent(level, out);
            out.push_str("Throw\n");
            dump_expr(value, level + 1, out);
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
        } => {
            indent(level, out);
            out.push_str("Try\n");
            indent(level + 1, out);
            out.push_str("block:\n");
            for s in block {
                dump_stmt(s, level + 2, out);
            }
            if let Some(handler) = handler {
                indent(level + 1, out);
                out.push_str("catch");
                if let Some(param) = handler_param {
                    out.push(' ');
                    dump_pattern_inline(param, out);
                }
                out.push_str(":\n");
                for s in handler {
                    dump_stmt(s, level + 2, out);
                }
            }
            if let Some(finalizer) = finalizer {
                indent(level + 1, out);
                out.push_str("finally:\n");
                for s in finalizer {
                    dump_stmt(s, level + 2, out);
                }
            }
        }
        Stmt::With { object, body } => {
            indent(level, out);
            out.push_str("With\n");
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            for s in body {
                dump_stmt(s, level + 2, out);
            }
        }
    }
}
