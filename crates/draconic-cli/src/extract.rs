//! v1 JSON extract for Onic: `draconic extract <file>`.

use std::collections::HashSet;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

use draconic_ast::{
    AccessorKind, Arg, ArrayPatternElement, ArrowBody, AssignOp, BindingKind, BindingPattern,
    ClassElement, Expr, ObjectKey, ObjectPatternProp, ObjectProp, Param, Program, Stmt, TypeAnn,
};
use draconic_diagnostics::Span;
use draconic_parser::parse_module;

struct NamedSpan {
    name: String,
    start_line: u32,
    end_line: u32,
    enclosing: Option<String>,
    abi: Option<String>,
    native: bool,
    member: bool,
    is_static: bool,
    accessor: Option<&'static str>,
}

struct ExtractV1 {
    functions: Vec<NamedSpan>,
    classes: Vec<NamedSpan>,
    type_aliases: Vec<NamedSpan>,
    extern_functions: Vec<NamedSpan>,
    methods: Vec<NamedSpan>,
    constructors: Vec<NamedSpan>,
    accessors: Vec<NamedSpan>,
    imports: Vec<NamedSpan>,
    exports: Vec<NamedSpan>,
    calls: Vec<NamedSpan>,
}

pub fn cmd_extract(args: &[String]) -> ExitCode {
    let path = match args.first() {
        Some(p) => p,
        None => {
            eprintln!("usage: draconic extract <file>");
            return ExitCode::from(2);
        }
    };
    if let Err(code) = super::toolchain_pin::enforce(Path::new(path)) {
        return code;
    }
    let source = match fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("failed to read {path}: {e}");
            return ExitCode::from(1);
        }
    };
    match parse_module(&source) {
        Ok(program) => {
            println!("{}", extract_json(&source, &program));
            ExitCode::SUCCESS
        }
        Err(d) => {
            eprintln!("error: {d}");
            ExitCode::from(1)
        }
    }
}

#[derive(Clone)]
struct WalkCtx {
    class_name: Option<String>,
    call_enclosing: Option<String>,
    module_level: bool,
    collect_calls: bool,
    /// Names whose last matching binding is `new Ctor` or an alias snapshot.
    /// Module visit uses one map; functions copy it at definition.
    instance_names: HashSet<String>,
}

fn extract_json(source: &str, program: &Program) -> String {
    let mut out = ExtractV1 {
        functions: Vec::new(),
        classes: Vec::new(),
        type_aliases: Vec::new(),
        extern_functions: Vec::new(),
        methods: Vec::new(),
        constructors: Vec::new(),
        accessors: Vec::new(),
        imports: Vec::new(),
        exports: Vec::new(),
        calls: Vec::new(),
    };
    let mut ctx = WalkCtx {
        class_name: None,
        call_enclosing: None,
        module_level: true,
        collect_calls: true,
        instance_names: HashSet::new(),
    };
    for stmt in &program.body {
        visit_stmt(source, stmt, &mut ctx, &mut out);
        seed_import_locals(stmt, &mut ctx.instance_names);
    }
    emit_json(&out)
}

fn named(source: &str, name: String, span: Span) -> NamedSpan {
    let (start_line, end_line) = span_lines(source, span);
    NamedSpan {
        name,
        start_line,
        end_line,
        enclosing: None,
        abi: None,
        native: false,
        member: false,
        is_static: false,
        accessor: None,
    }
}

fn type_alias_native(ty: &TypeAnn) -> bool {
    match ty {
        TypeAnn::Named { name, .. } => is_native_scalar(name),
        TypeAnn::Pointer { .. } => true,
        _ => false,
    }
}

fn is_native_scalar(name: &str) -> bool {
    matches!(
        name,
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "f32" | "f64"
    )
}

fn visit_stmt(source: &str, stmt: &Stmt, ctx: &mut WalkCtx, out: &mut ExtractV1) {
    match stmt {
        Stmt::FunctionDeclaration {
            name,
            params,
            body,
            span,
            ..
        } => {
            out.functions.push(named(source, name.name.clone(), *span));
            let mut inner = function_ctx(ctx, Some(name.name.clone()), params, true);
            visit_stmt(source, body, &mut inner, out);
        }
        Stmt::ClassDeclaration {
            name, body, span, ..
        } => {
            let emitted = match &ctx.class_name {
                Some(outer) => format!("{outer}.{}", name.name),
                None => name.name.clone(),
            };
            out.classes.push(named(source, emitted.clone(), *span));
            visit_class_body(source, body, &emitted, !ctx.module_level, out);
        }
        Stmt::TypeAlias { name, ty, span, .. } => {
            let mut item = named(source, name.name.clone(), *span);
            item.native = type_alias_native(ty);
            out.type_aliases.push(item);
        }
        Stmt::ExternFunctionDeclaration {
            name, abi, span, ..
        } => {
            let mut item = named(source, name.name.clone(), *span);
            item.abi = Some(abi.value.to_string_lossy());
            out.extern_functions.push(item);
        }
        Stmt::ImportDeclaration {
            source: spec, span, ..
        } => {
            out.imports
                .push(named(source, spec.value.to_string_lossy(), *span));
        }
        Stmt::Block { body, .. } => {
            for inner in body {
                visit_stmt(source, inner, ctx, out);
            }
        }
        Stmt::Expression { expr, .. } => {
            visit_expr(source, expr, ctx, out);
        }
        Stmt::Return { argument, .. } => {
            if let Some(expr) = argument {
                visit_expr(source, expr, ctx, out);
            }
        }
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            visit_stmt(source, consequent, ctx, out);
            if let Some(alt) = alternate {
                visit_stmt(source, alt, ctx, out);
            }
        }
        Stmt::While { body, .. }
        | Stmt::DoWhile { body, .. }
        | Stmt::Labeled { body, .. }
        | Stmt::With { body, .. } => {
            visit_stmt(source, body, ctx, out);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            let mut inner = loop_ctx(ctx);
            if let Some(init) = init {
                clear_for_binding(init, &mut inner.instance_names);
                visit_stmt(source, init, &mut inner, out);
            }
            if let Some(test) = test {
                visit_expr(source, test, &mut inner, out);
            }
            if let Some(update) = update {
                visit_expr(source, update, &mut inner, out);
            }
            visit_stmt(source, body, &mut inner, out);
        }
        Stmt::ForIn { left, body, .. } | Stmt::ForOf { left, body, .. } => {
            let mut inner = loop_ctx(ctx);
            clear_for_binding(left, &mut inner.instance_names);
            visit_stmt(source, left, &mut inner, out);
            visit_stmt(source, body, &mut inner, out);
        }
        Stmt::Switch { cases, .. } => {
            for case in cases {
                for inner in &case.body {
                    visit_stmt(source, inner, ctx, out);
                }
            }
        }
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            ..
        } => {
            visit_stmt(source, block, ctx, out);
            if let Some(handler) = handler {
                let mut inner = loop_ctx(ctx);
                if let Some(param) = handler_param {
                    delete_bound_names(param, &mut inner.instance_names);
                }
                visit_stmt(source, handler, &mut inner, out);
            }
            if let Some(finalizer) = finalizer {
                visit_stmt(source, finalizer, ctx, out);
            }
        }
        Stmt::Let {
            kind,
            binding,
            init,
            ..
        } => {
            if let Some(init) = init {
                let steal = matches!(binding, BindingPattern::Ident(_));
                if steal
                    && matches!(
                        kind,
                        BindingKind::Const | BindingKind::Let | BindingKind::Var
                    )
                {
                    if let BindingPattern::Ident(id) = binding {
                        if stamp_assigned_object_methods(source, &id.name, init, ctx, out) {
                            take_let_binding(binding, Some(init), &mut ctx.instance_names);
                            return;
                        }
                    }
                }
                visit_assigned_value(source, init, ctx, out, steal);
                take_let_binding(binding, Some(init), &mut ctx.instance_names);
            } else {
                take_let_binding(binding, None, &mut ctx.instance_names);
            }
        }
        Stmt::ExportDefaultDeclaration { declaration, .. } => match declaration.as_ref() {
            Stmt::Let {
                init: Some(init), ..
            } => {
                visit_expr(source, init, ctx, out);
            }
            other => visit_stmt(source, other, ctx, out),
        },
        Stmt::ExportNamedDeclaration {
            source: spec, span, ..
        } => {
            if let Some(spec) = spec {
                out.imports
                    .push(named(source, spec.value.to_string_lossy(), *span));
            }
        }
        Stmt::ExportAllDeclaration {
            source: spec, span, ..
        } => {
            out.imports
                .push(named(source, spec.value.to_string_lossy(), *span));
        }
        Stmt::Empty { .. } | Stmt::Break { .. } | Stmt::Continue { .. } | Stmt::Throw { .. } => {}
    }
}

fn visit_class_body(
    source: &str,
    elements: &[ClassElement],
    class_id: &str,
    nested_class: bool,
    out: &mut ExtractV1,
) {
    let nested_enclosing = if nested_class {
        Some(class_id.to_string())
    } else {
        None
    };
    let mut class_instances = HashSet::new();
    let member_ctx = WalkCtx {
        class_name: None,
        call_enclosing: nested_enclosing.clone(),
        module_level: false,
        collect_calls: nested_class,
        instance_names: HashSet::new(),
    };
    let mut static_ctx = WalkCtx {
        class_name: Some(class_id.to_string()),
        call_enclosing: nested_enclosing,
        module_level: false,
        collect_calls: nested_class,
        instance_names: HashSet::new(),
    };
    for element in elements {
        match element {
            ClassElement::StaticBlock { body, .. } => {
                visit_stmt(source, body, &mut static_ctx, out);
            }
            ClassElement::Constructor {
                params, body, span, ..
            } => {
                let ctor_id = format!("{class_id}.constructor");
                out.constructors.push(named(source, ctor_id.clone(), *span));
                let mut inner = function_ctx_from(&HashSet::new(), Some(ctor_id), params, true);
                visit_stmt(source, body, &mut inner, out);
            }
            ClassElement::Accessor {
                kind,
                key,
                params,
                body,
                is_private,
                span,
                ..
            } => {
                if let Some(accessor_id) = accessor_id(class_id, *kind, key, *is_private) {
                    let mut item = named(source, accessor_id.clone(), *span);
                    item.accessor = Some(match kind {
                        AccessorKind::Get => "get",
                        AccessorKind::Set => "set",
                    });
                    out.accessors.push(item);
                    let mut inner =
                        function_ctx_from(&HashSet::new(), Some(accessor_id), params, true);
                    visit_stmt(source, body, &mut inner, out);
                } else {
                    let mut inner = function_ctx_from(
                        &HashSet::new(),
                        member_ctx.call_enclosing.clone(),
                        params,
                        member_ctx.collect_calls,
                    );
                    visit_stmt(source, body, &mut inner, out);
                }
            }
            ClassElement::Method {
                key,
                params,
                body,
                is_static,
                is_private,
                span,
                ..
            } => {
                if let Some(method_id) = method_member_id(class_id, key, *is_private) {
                    let mut item = named(source, method_id.clone(), *span);
                    if *is_static {
                        item.is_static = true;
                    }
                    out.methods.push(item);
                    if *is_static || *is_private {
                        let mut inner =
                            function_ctx_from(&HashSet::new(), Some(method_id), params, true);
                        visit_stmt(source, body, &mut inner, out);
                    } else {
                        let mut inner =
                            function_ctx_from(&class_instances, Some(method_id), params, true);
                        visit_stmt(source, body, &mut inner, out);
                    }
                } else {
                    let mut inner = function_ctx_from(
                        &HashSet::new(),
                        member_ctx.call_enclosing.clone(),
                        params,
                        member_ctx.collect_calls,
                    );
                    visit_stmt(source, body, &mut inner, out);
                }
            }
            ClassElement::Field {
                key,
                value,
                is_static,
                is_private,
                span,
            } => {
                if !push_function_field(
                    source,
                    class_id,
                    key,
                    value.as_ref(),
                    *is_static,
                    *is_private,
                    *span,
                    &member_ctx,
                    out,
                ) {
                    if nested_class {
                        if let Some(expr) = value {
                            collect_call_expr(source, expr, Some(class_id), &mut out.calls);
                        }
                    }
                    if !*is_static && !*is_private {
                        if let ObjectKey::Ident(id) = key {
                            apply_instance_binding(&id.name, value.as_ref(), &mut class_instances);
                        }
                    }
                }
            }
        }
    }
}

fn method_member_id(class_id: &str, key: &ObjectKey, is_private: bool) -> Option<String> {
    let name = match key {
        ObjectKey::Ident(id) if is_private => format!("#{}", id.name),
        ObjectKey::Ident(id) => id.name.clone(),
        ObjectKey::String(s) => s.value.to_string_lossy(),
        ObjectKey::Computed(_) => return None,
    };
    Some(format!("{class_id}.{name}"))
}

fn push_function_field(
    source: &str,
    class_id: &str,
    key: &ObjectKey,
    value: Option<&Expr>,
    is_static: bool,
    is_private: bool,
    span: Span,
    member_ctx: &WalkCtx,
    out: &mut ExtractV1,
) -> bool {
    if !is_static && !is_private {
        return false;
    }
    let Some(expr) = value else {
        return false;
    };
    if !matches!(
        unwrap_parens(expr),
        Expr::ArrowFunction { .. } | Expr::FunctionExpression { .. }
    ) {
        return false;
    }
    let Some(method_id) = method_member_id(class_id, key, is_private) else {
        return false;
    };
    let mut item = named(source, method_id.clone(), span);
    if is_static {
        item.is_static = true;
    }
    out.methods.push(item);
    let mut steal_ctx = member_ctx.clone();
    steal_ctx.call_enclosing = Some(method_id);
    steal_ctx.collect_calls = true;
    let _ = skip_assigned_callable(source, unwrap_parens(expr), &steal_ctx, out);
    true
}

fn accessor_id(
    class_id: &str,
    kind: AccessorKind,
    key: &ObjectKey,
    is_private: bool,
) -> Option<String> {
    let kind_name = match kind {
        AccessorKind::Get => "get",
        AccessorKind::Set => "set",
    };
    let name = match key {
        ObjectKey::Ident(id) if is_private => format!("#{}", id.name),
        ObjectKey::Ident(id) => id.name.clone(),
        ObjectKey::String(s) => s.value.to_string_lossy(),
        ObjectKey::Computed(_) => return None,
    };
    Some(format!("{class_id}.{kind_name}.{name}"))
}

fn unwrap_parens(expr: &Expr) -> &Expr {
    match expr {
        Expr::Paren { expr, .. } | Expr::As { expr, .. } => unwrap_parens(expr),
        other => other,
    }
}

fn function_ctx(
    parent: &WalkCtx,
    enclosing: Option<String>,
    params: &[Param],
    collect_calls: bool,
) -> WalkCtx {
    function_ctx_from(&parent.instance_names, enclosing, params, collect_calls)
}

fn function_ctx_from(
    instances: &HashSet<String>,
    enclosing: Option<String>,
    params: &[Param],
    collect_calls: bool,
) -> WalkCtx {
    let mut instance_names = instances.clone();
    delete_params(params, &mut instance_names);
    WalkCtx {
        class_name: None,
        call_enclosing: enclosing,
        module_level: false,
        collect_calls,
        instance_names,
    }
}

fn loop_ctx(parent: &WalkCtx) -> WalkCtx {
    let mut inner = parent.clone();
    inner.module_level = false;
    inner
}

fn delete_params(params: &[Param], names: &mut HashSet<String>) {
    for param in params {
        delete_bound_names(&param.binding, names);
    }
}

fn delete_bound_names(binding: &BindingPattern, names: &mut HashSet<String>) {
    binding.for_each_ident(&mut |id| {
        names.remove(&id.name);
    });
}

fn seed_import_locals(stmt: &Stmt, names: &mut HashSet<String>) {
    let Stmt::ImportDeclaration {
        specifiers,
        namespace,
        type_only,
        ..
    } = stmt
    else {
        return;
    };
    if *type_only {
        return;
    }
    for spec in specifiers {
        if spec.is_type {
            continue;
        }
        names.insert(spec.local.name.clone());
    }
    if let Some(ns) = namespace {
        names.insert(ns.name.clone());
    }
}

fn take_let_binding(binding: &BindingPattern, init: Option<&Expr>, names: &mut HashSet<String>) {
    match binding {
        BindingPattern::Ident(id) => apply_instance_binding(&id.name, init, names),
        other => delete_bound_names(other, names),
    }
}

fn apply_instance_binding(name: &str, value: Option<&Expr>, names: &mut HashSet<String>) {
    let Some(value) = value else {
        names.remove(name);
        return;
    };
    match unwrap_parens(value) {
        Expr::New { callee, .. } => {
            if ctor_unwraps_to_ident_or_member(callee) {
                names.insert(name.to_string());
            } else {
                names.remove(name);
            }
        }
        Expr::Ident(id)
            if names.contains(&id.name) => {
                names.insert(name.to_string());
            }
        _ => {
            names.remove(name);
        }
    }
}

fn ctor_unwraps_to_ident_or_member(callee: &Expr) -> bool {
    match unwrap_parens(callee) {
        Expr::Ident(_) => true,
        Expr::MemberExpression {
            computed, private, ..
        } if !*computed && !*private => true,
        _ => false,
    }
}

fn take_assign_target(target: &Expr, value: &Expr, names: &mut HashSet<String>) {
    match unwrap_parens(target) {
        Expr::Ident(id) => apply_instance_binding(&id.name, Some(value), names),
        Expr::ArrayPattern { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, .. }
                    | ArrayPatternElement::Rest(binding) => {
                        delete_bound_names(binding, names);
                    }
                }
            }
        }
        Expr::ObjectPattern { properties, .. } => {
            for prop in properties {
                match prop {
                    ObjectPatternProp::Prop { binding, .. } | ObjectPatternProp::Rest(binding) => {
                        delete_bound_names(binding, names);
                    }
                }
            }
        }
        _ => {}
    }
}

fn clear_for_binding(stmt: &Stmt, names: &mut HashSet<String>) {
    match stmt {
        Stmt::Let { binding, .. } => delete_bound_names(binding, names),
        Stmt::Expression { expr, .. } => match unwrap_parens(expr) {
            Expr::Ident(id) => {
                names.remove(&id.name);
            }
            Expr::ArrayPattern { elements, .. } => {
                for el in elements {
                    match el {
                        ArrayPatternElement::Elision => {}
                        ArrayPatternElement::Pattern { binding, .. }
                        | ArrayPatternElement::Rest(binding) => {
                            delete_bound_names(binding, names);
                        }
                    }
                }
            }
            Expr::ObjectPattern { properties, .. } => {
                for prop in properties {
                    match prop {
                        ObjectPatternProp::Prop { binding, .. }
                        | ObjectPatternProp::Rest(binding) => {
                            delete_bound_names(binding, names);
                        }
                    }
                }
            }
            _ => {}
        },
        _ => {}
    }
}

fn stamp_assigned_object_methods(
    source: &str,
    lhs: &str,
    expr: &Expr,
    ctx: &WalkCtx,
    out: &mut ExtractV1,
) -> bool {
    let Expr::ObjectExpression { properties, .. } = unwrap_parens(expr) else {
        return false;
    };
    for prop in properties {
        let ObjectProp::Property {
            key,
            value,
            shorthand,
            span,
        } = prop
        else {
            continue;
        };
        if *shorthand {
            continue;
        }
        let Some(key_name) = object_method_key_name(key) else {
            continue;
        };
        let core = unwrap_parens(value);
        if !matches!(
            core,
            Expr::ArrowFunction { .. } | Expr::FunctionExpression { .. }
        ) {
            continue;
        }
        let method_id = format!("{lhs}.{key_name}");
        out.methods.push(named(source, method_id.clone(), *span));
        let mut steal_ctx = ctx.clone();
        steal_ctx.call_enclosing = Some(method_id);
        steal_ctx.collect_calls = true;
        steal_object_method_body(source, core, &steal_ctx, out);
    }
    true
}

fn object_method_key_name(key: &ObjectKey) -> Option<String> {
    match key {
        ObjectKey::Ident(id) if !id.name.is_empty() => Some(id.name.clone()),
        ObjectKey::String(s) => {
            let name = s.value.to_string_lossy();
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        }
        ObjectKey::Computed(expr) => computed_string_name(expr),
        _ => None,
    }
}

fn steal_object_method_body(source: &str, expr: &Expr, ctx: &WalkCtx, out: &mut ExtractV1) {
    match expr {
        Expr::ArrowFunction { body, params, .. } => {
            let mut inner = function_ctx(ctx, ctx.call_enclosing.clone(), params, true);
            match body {
                ArrowBody::Block(stmt) => visit_stmt(source, stmt, &mut inner, out),
                ArrowBody::Expr(inner_expr) => visit_expr(source, inner_expr, &mut inner, out),
            }
        }
        Expr::FunctionExpression { params, body, .. } => {
            let mut inner = function_ctx(ctx, ctx.call_enclosing.clone(), params, true);
            visit_stmt(source, body, &mut inner, out);
        }
        _ => {}
    }
}

fn visit_assigned_value(
    source: &str,
    expr: &Expr,
    ctx: &mut WalkCtx,
    out: &mut ExtractV1,
    steal: bool,
) {
    if steal && skip_assigned_callable(source, unwrap_parens(expr), ctx, out) {
        return;
    }
    visit_expr(source, expr, ctx, out);
}

fn skip_assigned_callable(source: &str, expr: &Expr, ctx: &WalkCtx, out: &mut ExtractV1) -> bool {
    match expr {
        Expr::ArrowFunction { body, params, .. } => {
            let mut inner =
                function_ctx(ctx, ctx.call_enclosing.clone(), params, ctx.collect_calls);
            match body {
                ArrowBody::Block(stmt) => visit_stmt(source, stmt, &mut inner, out),
                ArrowBody::Expr(inner_expr) => visit_expr(source, inner_expr, &mut inner, out),
            }
            true
        }
        Expr::FunctionExpression {
            is_method: false,
            params,
            body,
            ..
        } => {
            let mut inner =
                function_ctx(ctx, ctx.call_enclosing.clone(), params, ctx.collect_calls);
            visit_stmt(source, body, &mut inner, out);
            true
        }
        _ => false,
    }
}

fn visit_expr(source: &str, expr: &Expr, ctx: &mut WalkCtx, out: &mut ExtractV1) {
    match expr {
        Expr::ArrowFunction {
            body, params, span, ..
        } => {
            let (start_line, _) = span_lines(source, *span);
            let name = format!("arrow:{start_line}");
            out.functions.push(named(source, name.clone(), *span));
            let mut inner = function_ctx(ctx, Some(name), params, true);
            match body {
                ArrowBody::Block(stmt) => visit_stmt(source, stmt, &mut inner, out),
                ArrowBody::Expr(inner_expr) => visit_expr(source, inner_expr, &mut inner, out),
            }
        }
        Expr::FunctionExpression {
            name,
            params,
            body,
            is_method,
            span,
            ..
        } => {
            if *is_method {
                return;
            }
            if name.is_some() {
                let mut inner =
                    function_ctx(ctx, ctx.call_enclosing.clone(), params, ctx.collect_calls);
                visit_stmt(source, body, &mut inner, out);
                return;
            }
            let (start_line, _) = span_lines(source, *span);
            let name = format!("function:{start_line}");
            out.functions.push(named(source, name.clone(), *span));
            let mut inner = function_ctx(ctx, Some(name), params, true);
            visit_stmt(source, body, &mut inner, out);
        }
        Expr::Paren { expr, .. } | Expr::Unary { arg: expr, .. } | Expr::As { expr, .. } => {
            visit_expr(source, expr, ctx, out);
        }
        Expr::Assign {
            target, op, value, ..
        } => {
            visit_expr(source, value, ctx, out);
            if *op == AssignOp::Eq {
                take_assign_target(target, value, &mut ctx.instance_names);
            }
        }
        Expr::Call {
            callee,
            args,
            optional,
            span,
            ..
        } => {
            if ctx.collect_calls {
                push_call(
                    source,
                    callee,
                    *optional,
                    *span,
                    ctx.call_enclosing.clone(),
                    &ctx.instance_names,
                    false,
                    &mut out.calls,
                );
            }
            visit_expr(source, callee, ctx, out);
            for arg in args {
                match arg {
                    Arg::Expr(inner) | Arg::Spread(inner) => {
                        visit_expr(source, inner, ctx, out);
                    }
                }
            }
        }
        Expr::TaggedTemplate {
            tag,
            expressions,
            span,
            ..
        } => {
            if ctx.collect_calls {
                push_call(
                    source,
                    tag,
                    false,
                    *span,
                    ctx.call_enclosing.clone(),
                    &ctx.instance_names,
                    true,
                    &mut out.calls,
                );
            }
            for inner in expressions {
                visit_expr(source, inner, ctx, out);
            }
        }
        Expr::New { callee, args, .. } => {
            visit_expr(source, callee, ctx, out);
            for arg in args {
                match arg {
                    Arg::Expr(inner) | Arg::Spread(inner) => {
                        visit_expr(source, inner, ctx, out);
                    }
                }
            }
        }
        Expr::ObjectExpression { properties, .. } => {
            for prop in properties {
                match prop {
                    ObjectProp::Property {
                        value, shorthand, ..
                    } => {
                        if *shorthand {
                            continue;
                        }
                        visit_expr(source, value, ctx, out);
                    }
                    ObjectProp::Spread { expr, .. } => {
                        visit_expr(source, expr, ctx, out);
                    }
                    ObjectProp::Accessor { .. } => {}
                }
            }
        }
        _ => {}
    }
}

fn call_name(
    callee: &Expr,
    optional: bool,
    instance_names: &HashSet<String>,
    tagged: bool,
) -> Option<(String, bool)> {
    match unwrap_parens(callee) {
        Expr::Ident(id) => {
            if !tagged && !optional && instance_names.contains(&id.name) {
                Some(("__call__".to_string(), true))
            } else {
                Some((id.name.clone(), false))
            }
        }
        Expr::Call { .. } | Expr::TaggedTemplate { .. } if !tagged => {
            Some(("__call__".to_string(), true))
        }
        Expr::MemberExpression {
            property,
            computed,
            private,
            ..
        } if !*private => {
            if *computed {
                if tagged {
                    None
                } else {
                    computed_string_name(property).map(|name| (name, true))
                }
            } else {
                match unwrap_parens(property) {
                    Expr::Ident(id) => Some((id.name.clone(), true)),
                    _ => None,
                }
            }
        }
        _ => None,
    }
}

fn computed_string_name(property: &Expr) -> Option<String> {
    match unwrap_parens(property) {
        Expr::String(s) => {
            let name = s.value.to_string_lossy();
            if name.is_empty() {
                None
            } else {
                Some(name)
            }
        }
        _ => None,
    }
}

fn push_call(
    source: &str,
    callee: &Expr,
    optional: bool,
    span: Span,
    enclosing: Option<String>,
    instance_names: &HashSet<String>,
    tagged: bool,
    out: &mut Vec<NamedSpan>,
) {
    let Some((name, member)) = call_name(callee, optional, instance_names, tagged) else {
        return;
    };
    let (start_line, end_line) = span_lines(source, span);
    out.push(NamedSpan {
        name,
        start_line,
        end_line,
        enclosing,
        abi: None,
        native: false,
        member,
        is_static: false,
        accessor: None,
    });
}

fn collect_call_expr(source: &str, expr: &Expr, enclosing: Option<&str>, out: &mut Vec<NamedSpan>) {
    match expr {
        Expr::Call {
            callee, args, span, ..
        } => {
            push_call(
                source,
                callee,
                false,
                *span,
                enclosing.map(str::to_string),
                &HashSet::new(),
                false,
                out,
            );
            for arg in args {
                match arg {
                    Arg::Expr(inner) | Arg::Spread(inner) => {
                        collect_call_expr(source, inner, enclosing, out);
                    }
                }
            }
        }
        Expr::Paren { expr, .. } => collect_call_expr(source, expr, enclosing, out),
        _ => {}
    }
}

fn span_lines(source: &str, span: Span) -> (u32, u32) {
    let start = line_at(source, span.start.0);
    let end_byte = span.end.0.saturating_sub(1).max(span.start.0);
    let end = line_at(source, end_byte);
    (start, end)
}

fn line_at(source: &str, byte: u32) -> u32 {
    let n = (byte as usize).min(source.len());
    source.as_bytes()[..n]
        .iter()
        .filter(|&&b| b == b'\n')
        .count() as u32
        + 1
}

fn emit_json(extract: &ExtractV1) -> String {
    let mut s = String::from("{\"version\":1,");
    s.push_str("\"functions\":");
    emit_array(&mut s, &extract.functions);
    s.push_str(",\"classes\":");
    emit_array(&mut s, &extract.classes);
    s.push_str(",\"typeAliases\":");
    emit_array(&mut s, &extract.type_aliases);
    s.push_str(",\"externFunctions\":");
    emit_array(&mut s, &extract.extern_functions);
    s.push_str(",\"methods\":");
    emit_array(&mut s, &extract.methods);
    s.push_str(",\"constructors\":");
    emit_array(&mut s, &extract.constructors);
    s.push_str(",\"accessors\":");
    emit_array(&mut s, &extract.accessors);
    s.push_str(",\"imports\":");
    emit_array(&mut s, &extract.imports);
    s.push_str(",\"exports\":");
    emit_array(&mut s, &extract.exports);
    s.push_str(",\"calls\":");
    emit_array(&mut s, &extract.calls);
    s.push('}');
    s
}

fn emit_array(s: &mut String, items: &[NamedSpan]) {
    s.push('[');
    for (i, item) in items.iter().enumerate() {
        if i > 0 {
            s.push(',');
        }
        s.push('{');
        s.push_str("\"name\":");
        push_json_string(s, &item.name);
        s.push_str(",\"startLine\":");
        s.push_str(&item.start_line.to_string());
        s.push_str(",\"endLine\":");
        s.push_str(&item.end_line.to_string());
        if let Some(enclosing) = &item.enclosing {
            s.push_str(",\"enclosing\":");
            push_json_string(s, enclosing);
        }
        if let Some(abi) = &item.abi {
            s.push_str(",\"abi\":");
            push_json_string(s, abi);
        }
        if item.native {
            s.push_str(",\"native\":true");
        }
        if item.member {
            s.push_str(",\"member\":true");
        }
        if item.is_static {
            s.push_str(",\"static\":true");
        }
        if let Some(accessor) = item.accessor {
            s.push_str(",\"accessor\":");
            push_json_string(s, accessor);
        }
        s.push('}');
    }
    s.push(']');
}

fn push_json_string(s: &mut String, value: &str) {
    s.push('"');
    for c in value.chars() {
        match c {
            '"' => s.push_str("\\\""),
            '\\' => s.push_str("\\\\"),
            '\n' => s.push_str("\\n"),
            '\r' => s.push_str("\\r"),
            '\t' => s.push_str("\\t"),
            c if (c as u32) < 0x20 => s.push_str(&format!("\\u{:04x}", c as u32)),
            c => s.push(c),
        }
    }
    s.push('"');
}
