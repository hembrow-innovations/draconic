mod dump_expr;

use dump_expr::dump_expr;
use crate::{
    AccessorKind, ArrayPatternElement, BindingKind, BindingPattern, ClassElement, ImportAttribute,
    ImportAttributeKey, ImportPhase, ObjectKey, ObjectPatternProp, Param, Program, Stmt, TypeAnn,
};

/// Stable, indentation-based AST dump for snapshots and `draconic parse`.
pub fn dump_program(program: &Program) -> String {
    let mut out = String::new();
    out.push_str("Program\n");
    for stmt in &program.body {
        dump_stmt(stmt, 1, &mut out);
    }
    out
}

pub(crate) fn indent(level: usize, out: &mut String) {
    for _ in 0..level {
        out.push_str("  ");
    }
}

/// Compact single-line-ish dump for catch param headers (`catch (e)` / `catch ([a, b])`).
fn dump_binding_pattern_inline(pat: &BindingPattern, out: &mut String) {
    match pat {
        BindingPattern::Ident(name) => out.push_str(&name.name),
        BindingPattern::Member(_) => out.push_str("<member>"),
        BindingPattern::Array { elements, .. } => {
            out.push('[');
            for (i, el) in elements.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        dump_binding_pattern_inline(binding, out);
                        if default.is_some() {
                            out.push_str(" = …");
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        out.push_str("...");
                        dump_binding_pattern_inline(binding, out);
                    }
                }
            }
            out.push(']');
        }
        BindingPattern::Object { properties, .. } => {
            out.push('{');
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
                            dump_object_key_inline(key, out);
                        } else {
                            dump_object_key_inline(key, out);
                            out.push_str(": ");
                            dump_binding_pattern_inline(binding, out);
                        }
                        if default.is_some() {
                            out.push_str(" = …");
                        }
                    }
                    ObjectPatternProp::Rest(binding) => {
                        out.push_str("...");
                        dump_binding_pattern_inline(binding, out);
                    }
                }
            }
            out.push('}');
        }
    }
}

pub(crate) fn dump_binding_pattern(pat: &BindingPattern, level: usize, out: &mut String) {
    match pat {
        BindingPattern::Ident(name) => {
            indent(level, out);
            out.push_str(&format!("name: {}\n", name.name));
        }
        BindingPattern::Member(expr) => {
            indent(level, out);
            out.push_str("MemberTarget\n");
            dump_expr(expr, level + 1, out);
        }
        BindingPattern::Array { elements, .. } => {
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
        BindingPattern::Object { properties, .. } => {
            indent(level, out);
            out.push_str("ObjectPattern\n");
            dump_object_pattern_props(properties, level + 1, out);
        }
    }
}

pub(crate) fn dump_object_pattern_props(
    properties: &[ObjectPatternProp],
    level: usize,
    out: &mut String,
) {
    for p in properties {
        match p {
            ObjectPatternProp::Prop {
                key,
                binding,
                shorthand,
                default,
                ..
            } => {
                indent(level, out);
                if *shorthand {
                    out.push_str("prop shorthand:\n");
                } else {
                    out.push_str("prop:\n");
                }
                indent(level + 1, out);
                match key {
                    ObjectKey::Ident(id) => out.push_str(&format!("key: {}\n", id.name)),
                    ObjectKey::String(s) => {
                        out.push_str(&format!("key: {}\n", s.value.to_string_lossy()))
                    }
                    ObjectKey::Computed(expr) => {
                        out.push_str("key: Computed\n");
                        dump_expr(expr, level + 2, out);
                    }
                }
                indent(level + 1, out);
                out.push_str("binding:\n");
                dump_binding_pattern(binding, level + 2, out);
                if let Some(def) = default {
                    indent(level + 1, out);
                    out.push_str("default:\n");
                    dump_expr(def, level + 2, out);
                }
            }
            ObjectPatternProp::Rest(binding) => {
                indent(level, out);
                out.push_str("rest:\n");
                dump_binding_pattern(binding, level + 1, out);
            }
        }
    }
}

fn dump_object_key_inline(key: &ObjectKey, out: &mut String) {
    match key {
        ObjectKey::Ident(id) => out.push_str(&id.name),
        ObjectKey::String(s) => out.push_str(&s.value.to_string_lossy()),
        ObjectKey::Computed(_) => out.push_str("[…]"),
    }
}

fn dump_import_attributes(attributes: &[ImportAttribute], level: usize, out: &mut String) {
    for attr in attributes {
        indent(level, out);
        out.push_str("ImportAttribute\n");
        indent(level + 1, out);
        match &attr.key {
            ImportAttributeKey::Ident(id) => {
                out.push_str("key: ");
                out.push_str(&id.name);
                out.push('\n');
            }
            ImportAttributeKey::String(s) => {
                out.push_str(&format!("key: String {:?}\n", s.value.to_string_lossy()));
            }
        }
        indent(level + 1, out);
        out.push_str(&format!(
            "value: {:?}\n",
            attr.value.value.to_string_lossy()
        ));
    }
}

pub(crate) fn dump_stmt(stmt: &Stmt, level: usize, out: &mut String) {
    match stmt {
        Stmt::Expression { expr, .. } => {
            indent(level, out);
            out.push_str("ExpressionStatement\n");
            dump_expr(expr, level + 1, out);
        }
        Stmt::Let {
            kind,
            binding,
            type_ann,
            init,
            ..
        } => {
            indent(level, out);
            match kind {
                BindingKind::Let => out.push_str("Let\n"),
                BindingKind::Const => out.push_str("Const\n"),
                BindingKind::Var => out.push_str("Var\n"),
                BindingKind::Function => out.push_str("FunctionBinding\n"),
                BindingKind::Using => out.push_str("Using\n"),
                BindingKind::AwaitUsing => out.push_str("AwaitUsing\n"),
            }
            dump_binding_pattern(binding, level + 1, out);
            if let Some(ann) = type_ann {
                indent(level + 1, out);
                out.push_str("type:\n");
                dump_type_ann(ann, level + 2, out);
            }
            if let Some(init) = init {
                indent(level + 1, out);
                out.push_str("init:\n");
                dump_expr(init, level + 2, out);
            }
        }
        Stmt::Empty { .. } => {
            indent(level, out);
            out.push_str("EmptyStatement\n");
        }
        Stmt::Block { body, .. } => {
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
            ..
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
        Stmt::While { test, body, .. } => {
            indent(level, out);
            out.push_str("While\n");
            indent(level + 1, out);
            out.push_str("test:\n");
            dump_expr(test, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
        Stmt::DoWhile { body, test, .. } => {
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
            ..
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
        Stmt::ForIn {
            left, right, body, ..
        } => {
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
            ..
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
        Stmt::Break { label, .. } => {
            indent(level, out);
            if let Some(label) = label {
                out.push_str(&format!("Break {}\n", label.name));
            } else {
                out.push_str("Break\n");
            }
        }
        Stmt::Continue { label, .. } => {
            indent(level, out);
            if let Some(label) = label {
                out.push_str(&format!("Continue {}\n", label.name));
            } else {
                out.push_str("Continue\n");
            }
        }
        Stmt::Labeled { label, body, .. } => {
            indent(level, out);
            out.push_str(&format!("Labeled {}\n", label.name));
            dump_stmt(body, level + 1, out);
        }
        Stmt::Switch {
            discriminant,
            cases,
            ..
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
        Stmt::FunctionDeclaration {
            name,
            type_params,
            params,
            return_type,
            body,
            is_async,
            is_generator,
            ..
        } => {
            indent(level, out);
            out.push_str("FunctionDeclaration\n");
            if *is_async {
                indent(level + 1, out);
                out.push_str("async: true\n");
            }
            if *is_generator {
                indent(level + 1, out);
                out.push_str("generator: true\n");
            }
            indent(level + 1, out);
            out.push_str(&format!("name: {}\n", name.name));
            if !type_params.is_empty() {
                indent(level + 1, out);
                out.push_str("typeParams:\n");
                for tp in type_params {
                    indent(level + 2, out);
                    out.push_str(&format!("{}\n", tp.name.name));
                }
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
        Stmt::TypeAlias {
            name,
            type_params,
            ty,
            ..
        } => {
            indent(level, out);
            out.push_str("TypeAlias\n");
            indent(level + 1, out);
            out.push_str(&format!("name: {}\n", name.name));
            if !type_params.is_empty() {
                indent(level + 1, out);
                out.push_str("typeParams:\n");
                for tp in type_params {
                    indent(level + 2, out);
                    out.push_str(&format!("{}\n", tp.name.name));
                }
            }
            indent(level + 1, out);
            out.push_str("type:\n");
            dump_type_ann(ty, level + 2, out);
        }
        Stmt::ExternFunctionDeclaration {
            abi,
            name,
            params,
            return_type,
            ..
        } => {
            indent(level, out);
            out.push_str("ExternFunctionDeclaration\n");
            indent(level + 1, out);
            out.push_str(&format!("abi: {:?}\n", abi.value.to_string_lossy()));
            indent(level + 1, out);
            out.push_str(&format!("name: {}\n", name.name));
            dump_params(params, level + 1, out);
            if let Some(ret) = return_type {
                indent(level + 1, out);
                out.push_str("returnType:\n");
                dump_type_ann(ret, level + 2, out);
            }
        }
        Stmt::ClassDeclaration {
            name,
            super_class,
            body,
            ..
        } => {
            indent(level, out);
            out.push_str("ClassDeclaration\n");
            indent(level + 1, out);
            out.push_str(&format!("name: {}\n", name.name));
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
        Stmt::Return { argument, .. } => {
            indent(level, out);
            out.push_str("Return\n");
            if let Some(arg) = argument {
                dump_expr(arg, level + 1, out);
            }
        }
        Stmt::Throw { argument, .. } => {
            indent(level, out);
            out.push_str("Throw\n");
            dump_expr(argument, level + 1, out);
        }
        Stmt::ImportDeclaration {
            specifiers,
            namespace,
            source,
            attributes,
            phase,
            ..
        } => {
            indent(level, out);
            out.push_str("ImportDeclaration\n");
            if *phase == ImportPhase::Defer {
                indent(level + 1, out);
                out.push_str("phase: defer\n");
            }
            for spec in specifiers {
                indent(level + 1, out);
                out.push_str("ImportSpecifier\n");
                indent(level + 2, out);
                out.push_str("imported: ");
                out.push_str(&spec.imported.name);
                out.push('\n');
                indent(level + 2, out);
                out.push_str("local: ");
                out.push_str(&spec.local.name);
                out.push('\n');
            }
            if let Some(ns) = namespace {
                indent(level + 1, out);
                out.push_str("namespace: ");
                out.push_str(&ns.name);
                out.push('\n');
            }
            indent(level + 1, out);
            out.push_str("source: ");
            out.push_str(&source.value.to_string_lossy());
            out.push('\n');
            dump_import_attributes(attributes, level + 1, out);
        }
        Stmt::ExportNamedDeclaration {
            declaration,
            specifiers,
            source,
            attributes,
            ..
        } => {
            indent(level, out);
            out.push_str("ExportNamedDeclaration\n");
            if let Some(decl) = declaration {
                indent(level + 1, out);
                out.push_str("declaration:\n");
                dump_stmt(decl, level + 2, out);
            }
            for spec in specifiers {
                indent(level + 1, out);
                out.push_str("ExportSpecifier\n");
                indent(level + 2, out);
                out.push_str("local: ");
                out.push_str(&spec.local.name);
                out.push('\n');
                indent(level + 2, out);
                out.push_str("exported: ");
                out.push_str(&spec.exported.name);
                out.push('\n');
            }
            if let Some(source) = source {
                indent(level + 1, out);
                out.push_str("source: ");
                out.push_str(&source.value.to_string_lossy());
                out.push('\n');
            }
            dump_import_attributes(attributes, level + 1, out);
        }
        Stmt::ExportDefaultDeclaration {
            declaration, local, ..
        } => {
            indent(level, out);
            out.push_str("ExportDefaultDeclaration\n");
            indent(level + 1, out);
            out.push_str("local: ");
            out.push_str(&local.name);
            out.push('\n');
            indent(level + 1, out);
            out.push_str("declaration:\n");
            dump_stmt(declaration, level + 2, out);
        }
        Stmt::ExportAllDeclaration {
            exported,
            source,
            attributes,
            ..
        } => {
            indent(level, out);
            out.push_str("ExportAllDeclaration\n");
            if let Some(exported) = exported {
                indent(level + 1, out);
                out.push_str("exported: ");
                out.push_str(&exported.name);
                out.push('\n');
            }
            indent(level + 1, out);
            out.push_str("source: ");
            out.push_str(&source.value.to_string_lossy());
            out.push('\n');
            dump_import_attributes(attributes, level + 1, out);
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            ..
        } => {
            indent(level, out);
            out.push_str("Try\n");
            indent(level + 1, out);
            out.push_str("block:\n");
            dump_stmt(block, level + 2, out);
            if let Some(handler) = handler {
                indent(level + 1, out);
                out.push_str("catch");
                if let Some(param) = handler_param {
                    out.push_str(" (");
                    dump_binding_pattern_inline(param, out);
                    out.push(')');
                }
                out.push_str(":\n");
                dump_stmt(handler, level + 2, out);
            }
            if let Some(finalizer) = finalizer {
                indent(level + 1, out);
                out.push_str("finally:\n");
                dump_stmt(finalizer, level + 2, out);
            }
        }
        Stmt::With { object, body, .. } => {
            indent(level, out);
            out.push_str("With\n");
            indent(level + 1, out);
            out.push_str("object:\n");
            dump_expr(object, level + 2, out);
            indent(level + 1, out);
            out.push_str("body:\n");
            dump_stmt(body, level + 2, out);
        }
    }
}

pub(crate) fn dump_class_element_key(
    key: &ObjectKey,
    is_private: bool,
    level: usize,
    out: &mut String,
) {
    indent(level, out);
    match key {
        ObjectKey::Ident(id) if is_private => {
            out.push_str(&format!("name: #{}\n", id.name));
        }
        ObjectKey::Ident(id) => {
            out.push_str(&format!("name: {}\n", id.name));
        }
        ObjectKey::String(s) => {
            out.push_str(&format!("key: String {:?}\n", s.value.to_string_lossy()));
        }
        ObjectKey::Computed(expr) => {
            out.push_str("key: Computed\n");
            dump_expr(expr, level + 1, out);
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
        match (&p.binding, p.rest) {
            (BindingPattern::Ident(id), true) => {
                indent(level + 1, out);
                out.push_str(&format!("rest: {}\n", id.name));
            }
            (binding, false) => {
                dump_binding_pattern(binding, level + 1, out);
            }
            (binding, true) => {
                indent(level + 1, out);
                out.push_str("rest:\n");
                dump_binding_pattern(binding, level + 2, out);
            }
        }
        if let Some(ann) = &p.type_ann {
            indent(level + 2, out);
            out.push_str("type:\n");
            dump_type_ann(ann, level + 3, out);
        }
        if let Some(default) = &p.default {
            indent(level + 2, out);
            out.push_str("default:\n");
            dump_expr(default, level + 3, out);
        }
    }
}

pub(crate) fn dump_type_ann(ann: &TypeAnn, level: usize, out: &mut String) {
    match ann {
        TypeAnn::Named { name, .. } => {
            indent(level, out);
            out.push_str(&format!("NamedType {}\n", name));
        }
        TypeAnn::GenericApp { name, args, .. } => {
            indent(level, out);
            out.push_str(&format!("GenericApp {}\n", name));
            for a in args {
                dump_type_ann(a, level + 1, out);
            }
        }
        TypeAnn::Object { props, .. } => {
            indent(level, out);
            out.push_str("ObjectType\n");
            for p in props {
                indent(level + 1, out);
                out.push_str(&format!("prop: {}\n", p.name));
                dump_type_ann(&p.ty, level + 2, out);
            }
        }
        TypeAnn::Tuple { elements, .. } => {
            indent(level, out);
            out.push_str("TupleType\n");
            for el in elements {
                dump_type_ann(el, level + 1, out);
            }
        }
        TypeAnn::Pointer { inner, .. } => {
            indent(level, out);
            out.push_str("PointerType\n");
            dump_type_ann(inner, level + 1, out);
        }
        TypeAnn::Union { types, .. } => {
            indent(level, out);
            out.push_str("UnionType\n");
            for t in types {
                dump_type_ann(t, level + 1, out);
            }
        }
        TypeAnn::Intersection { types, .. } => {
            indent(level, out);
            out.push_str("IntersectionType\n");
            for t in types {
                dump_type_ann(t, level + 1, out);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BindingKind, BindingPattern, Expr, Ident, NumberLit, Program, Stmt};
    use draconic_diagnostics::Span;

    #[test]
    fn dump_let_number() {
        let program = Program {
            body: vec![Stmt::Let {
                kind: BindingKind::Let,
                binding: BindingPattern::Ident(Ident {
                    name: "x".into(),
                    span: Span::dummy(),
                }),
                type_ann: None,
                init: Some(Expr::Number(NumberLit {
                    raw: "1".into(),
                    span: Span::dummy(),
                })),
                span: Span::dummy(),
            }],
            span: Span::dummy(),
        };
        let dump = dump_program(&program);
        assert_eq!(
            dump,
            "\
Program
  Let
    name: x
    init:
      Number 1
"
        );
    }
}
