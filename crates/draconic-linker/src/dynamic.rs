use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use draconic_ast::{
    Arg, ArrayElement, ArrayPatternElement, ArrowBody, BindingPattern, ClassElement, Expr, Ident,
    ImportPhase, ObjectKey, ObjectPatternProp, ObjectProp, Stmt,
};
use draconic_diagnostics::{codes, Diagnostic, Span};
use draconic_parser::parse;

use crate::load::Loader;
use crate::namespace::{
    deferred_eval_fn_name, deferred_namespace_binding_name, shared_namespace_binding_name,
};
use crate::path::lexical_normalize_path;
use crate::spans::{uniqueify_expr_spans, SyntheticSpans};

impl Loader {
    /// E19.84.02 / E19.84.08: rewrite dynamic `import.defer("…")` and evaluation-phase
    /// `import("…")` of linked modules. Defer → `Promise.resolve(__ns_defer{id})`.
    /// Evaluation → Promise that runs the module's once-eval thunk (if any), rethrows
    /// a cached `[[EvaluationError]]`, and fulfills with `__ns{id}`.
    pub(crate) fn rewrite_dynamic_deferred_imports(
        &self,
        body: &mut Vec<Stmt>,
        self_path: &Path,
        deferred_ns_targets: &HashSet<usize>,
        spans: &mut SyntheticSpans,
    ) -> Result<(), Diagnostic> {
        let mut ctx = RewriteCtx {
            importer_dir: self_path.parent().unwrap_or(Path::new("")).to_path_buf(),
            deferred_ns_targets,
            ids: &self.ids,
            spans,
        };
        for stmt in body.iter_mut() {
            rewrite_stmt_dynamic_imports(stmt, &mut ctx)?;
        }
        Ok(())
    }
}

struct RewriteCtx<'a> {
    importer_dir: PathBuf,
    deferred_ns_targets: &'a HashSet<usize>,
    ids: &'a HashMap<PathBuf, usize>,
    spans: &'a mut SyntheticSpans,
}

fn rewrite_stmt_dynamic_imports(
    stmt: &mut Stmt,
    ctx: &mut RewriteCtx<'_>,
) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Expression { expr, .. } => rewrite_expr_dynamic_imports(expr, ctx)?,
        Stmt::Let {
            binding,
            init,
            span,
            ..
        } => {
            rewrite_binding_dynamic_imports(binding, ctx)?;
            if let Some(init) = init {
                rewrite_expr_dynamic_imports(init, ctx)?;
            }
            *span = Span::dummy(); // rewritten imports carry their own spans
        }
        Stmt::Empty { .. } => {}
        Stmt::Block { body, .. } => {
            for s in body {
                rewrite_stmt_dynamic_imports(s, ctx)?;
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            rewrite_expr_dynamic_imports(test, ctx)?;
            rewrite_stmt_dynamic_imports(consequent, ctx)?;
            if let Some(alt) = alternate {
                rewrite_stmt_dynamic_imports(alt, ctx)?;
            }
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            rewrite_expr_dynamic_imports(test, ctx)?;
            rewrite_stmt_dynamic_imports(body, ctx)?;
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            if let Some(init) = init {
                rewrite_stmt_dynamic_imports(init, ctx)?;
            }
            if let Some(test) = test {
                rewrite_expr_dynamic_imports(test, ctx)?;
            }
            if let Some(update) = update {
                rewrite_expr_dynamic_imports(update, ctx)?;
            }
            rewrite_stmt_dynamic_imports(body, ctx)?;
        }
        Stmt::ForIn {
            left, right, body, ..
        }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            rewrite_stmt_dynamic_imports(left, ctx)?;
            rewrite_expr_dynamic_imports(right, ctx)?;
            rewrite_stmt_dynamic_imports(body, ctx)?;
        }
        Stmt::Labeled { body, .. } => rewrite_stmt_dynamic_imports(body, ctx)?,
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            rewrite_expr_dynamic_imports(discriminant, ctx)?;
            for c in cases {
                if let Some(test) = &mut c.test {
                    rewrite_expr_dynamic_imports(test, ctx)?;
                }
                for s in &mut c.body {
                    rewrite_stmt_dynamic_imports(s, ctx)?;
                }
            }
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            rewrite_stmt_dynamic_imports(block, ctx)?;
            if let Some(handler) = handler {
                rewrite_stmt_dynamic_imports(handler, ctx)?;
            }
            if let Some(finalizer) = finalizer {
                rewrite_stmt_dynamic_imports(finalizer, ctx)?;
            }
        }
        Stmt::With { object, body, .. } => {
            rewrite_expr_dynamic_imports(object, ctx)?;
            rewrite_stmt_dynamic_imports(body, ctx)?;
        }
        Stmt::FunctionDeclaration { body, params, .. } => {
            for p in params {
                rewrite_binding_dynamic_imports(&mut p.binding, ctx)?;
                if let Some(default) = &mut p.default {
                    rewrite_expr_dynamic_imports(default, ctx)?;
                }
            }
            rewrite_stmt_dynamic_imports(body, ctx)?;
        }
        Stmt::ClassDeclaration {
            super_class, body, ..
        } => {
            if let Some(super_class) = super_class {
                rewrite_expr_dynamic_imports(super_class, ctx)?;
            }
            rewrite_class_elements_dynamic_imports(body, ctx)?;
        }
        Stmt::Return { argument, .. } => {
            if let Some(argument) = argument {
                rewrite_expr_dynamic_imports(argument, ctx)?;
            }
        }
        Stmt::Throw { argument, .. } => rewrite_expr_dynamic_imports(argument, ctx)?,
        Stmt::Break { .. } | Stmt::Continue { .. } => {}
        Stmt::ImportDeclaration { .. }
        | Stmt::ExportNamedDeclaration { .. }
        | Stmt::ExportDefaultDeclaration { .. }
        | Stmt::ExportAllDeclaration { .. }
        | Stmt::TypeAlias { .. }
        | Stmt::ExternFunctionDeclaration { .. } => {}
    }
    Ok(())
}

fn rewrite_class_elements_dynamic_imports(
    elements: &mut [ClassElement],
    ctx: &mut RewriteCtx<'_>,
) -> Result<(), Diagnostic> {
    for el in elements {
        match el {
            ClassElement::Constructor { body, .. } | ClassElement::StaticBlock { body, .. } => {
                rewrite_stmt_dynamic_imports(body, ctx)?
            }
            ClassElement::Method {
                key, params, body, ..
            }
            | ClassElement::Accessor {
                key, params, body, ..
            } => {
                if let ObjectKey::Computed(key) = key {
                    rewrite_expr_dynamic_imports(key, ctx)?;
                }
                for p in params {
                    rewrite_binding_dynamic_imports(&mut p.binding, ctx)?;
                    if let Some(default) = &mut p.default {
                        rewrite_expr_dynamic_imports(default, ctx)?;
                    }
                }
                rewrite_stmt_dynamic_imports(body, ctx)?;
            }
            ClassElement::Field {
                key,
                value,
                is_static,
                ..
            } => {
                if *is_static {
                    if let ObjectKey::Computed(key) = key {
                        rewrite_expr_dynamic_imports(key, ctx)?;
                    }
                    if let Some(value) = value {
                        rewrite_expr_dynamic_imports(value, ctx)?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn rewrite_binding_dynamic_imports(
    pat: &mut BindingPattern,
    ctx: &mut RewriteCtx<'_>,
) -> Result<(), Diagnostic> {
    match pat {
        BindingPattern::Ident(_) | BindingPattern::Member(_) => {}
        BindingPattern::Array { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern {
                        binding, default, ..
                    } => {
                        rewrite_binding_dynamic_imports(binding, ctx)?;
                        if let Some(default) = default {
                            rewrite_expr_dynamic_imports(default, ctx)?;
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        rewrite_binding_dynamic_imports(binding, ctx)?
                    }
                }
            }
        }
        BindingPattern::Object { properties, .. } => {
            for prop in properties {
                match prop {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        default,
                        ..
                    } => {
                        if let ObjectKey::Computed(key) = key {
                            rewrite_expr_dynamic_imports(key, ctx)?;
                        }
                        rewrite_binding_dynamic_imports(binding, ctx)?;
                        if let Some(default) = default {
                            rewrite_expr_dynamic_imports(default, ctx)?;
                        }
                    }
                    ObjectPatternProp::Rest(binding) => {
                        rewrite_binding_dynamic_imports(binding, ctx)?
                    }
                }
            }
        }
    }
    Ok(())
}

fn rewrite_expr_dynamic_imports(
    expr: &mut Expr,
    ctx: &mut RewriteCtx<'_>,
) -> Result<(), Diagnostic> {
    match expr {
        Expr::Ident(_)
        | Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::String(_)
        | Expr::RegExp { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::Super { .. }
        | Expr::NewTarget { .. }
        | Expr::ImportMeta { .. } => {}
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } => {
            rewrite_expr_dynamic_imports(arg, ctx)?
        }
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => {
            rewrite_expr_dynamic_imports(left, ctx)?;
            rewrite_expr_dynamic_imports(right, ctx)?;
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            rewrite_expr_dynamic_imports(test, ctx)?;
            rewrite_expr_dynamic_imports(consequent, ctx)?;
            rewrite_expr_dynamic_imports(alternate, ctx)?;
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            rewrite_expr_dynamic_imports(callee, ctx)?;
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => rewrite_expr_dynamic_imports(e, ctx)?,
                }
            }
        }
        Expr::ImportCall { .. } => {
            let defer_replacement = match expr {
                Expr::ImportCall {
                    phase,
                    source,
                    span,
                    ..
                } if *phase == ImportPhase::Defer => ctx
                    .deferred_ident_for_source(source)?
                    .map(|name| (name, *span)),
                _ => None,
            };
            if let Some((name, span)) = defer_replacement {
                // Spec: ImportCall with ~defer~ returns a Promise of the deferred ns.
                // `await import.defer(…)` and `.then(…)` both need a thenable.
                // Distinct spans: binder/IR key symbols by Span (shared span collapses names).
                let sp_p = Span::new(span.start.0.saturating_add(1), span.end.0);
                let sp_r = Span::new(span.start.0.saturating_add(2), span.end.0);
                let sp_n = Span::new(span.start.0.saturating_add(3), span.end.0);
                let sp_c = Span::new(span.start.0.saturating_add(4), span.end.0);
                *expr = Expr::Call {
                    callee: Box::new(Expr::MemberExpression {
                        object: Box::new(Expr::Ident(Ident {
                            name: "Promise".into(),
                            span: sp_p,
                        })),
                        property: Box::new(Expr::Ident(Ident {
                            name: "resolve".into(),
                            span: sp_r,
                        })),
                        computed: false,
                        optional: false,
                        private: false,
                        span: sp_c,
                    }),
                    args: vec![Arg::Expr(Expr::Ident(Ident { name, span: sp_n }))],
                    optional: false,
                    span: sp_c,
                };
                return Ok(());
            }
            // E19.84.08: evaluation-phase `import("linked")` → evaluate + eager ns.
            let eval_replacement = match expr {
                Expr::ImportCall {
                    phase,
                    source,
                    span,
                    ..
                } if *phase == ImportPhase::Evaluation => {
                    ctx.eval_import_rewrite_for_source(source, *span)?
                }
                _ => None,
            };
            if let Some(rewritten) = eval_replacement {
                *expr = rewritten;
                return Ok(());
            }
            if let Expr::ImportCall {
                phase,
                source,
                options,
                ..
            } = expr
            {
                let _ = phase;
                rewrite_expr_dynamic_imports(source, ctx)?;
                if let Some(options) = options {
                    rewrite_expr_dynamic_imports(options, ctx)?;
                }
            }
        }
        Expr::MemberExpression {
            object, property, ..
        } => {
            rewrite_expr_dynamic_imports(object, ctx)?;
            rewrite_expr_dynamic_imports(property, ctx)?;
        }
        Expr::PrivateIn { object, .. } => rewrite_expr_dynamic_imports(object, ctx)?,
        Expr::ArrayExpression { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        rewrite_expr_dynamic_imports(e, ctx)?
                    }
                    ArrayElement::Elision => {}
                }
            }
        }
        Expr::ObjectExpression { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property { key, value, .. } => {
                        if let ObjectKey::Computed(key) = key {
                            rewrite_expr_dynamic_imports(key, ctx)?;
                        }
                        rewrite_expr_dynamic_imports(value, ctx)?;
                    }
                    ObjectProp::Accessor {
                        key, params, body, ..
                    } => {
                        if let ObjectKey::Computed(key) = key {
                            rewrite_expr_dynamic_imports(key, ctx)?;
                        }
                        for p in params {
                            rewrite_binding_dynamic_imports(&mut p.binding, ctx)?;
                            if let Some(default) = &mut p.default {
                                rewrite_expr_dynamic_imports(default, ctx)?;
                            }
                        }
                        rewrite_stmt_dynamic_imports(body, ctx)?;
                    }
                    ObjectProp::Spread { expr, .. } => rewrite_expr_dynamic_imports(expr, ctx)?,
                }
            }
        }
        Expr::TemplateLiteral { expressions, .. } | Expr::TaggedTemplate { expressions, .. } => {
            for e in expressions {
                rewrite_expr_dynamic_imports(e, ctx)?;
            }
        }
        Expr::Paren { expr, .. } | Expr::As { expr, .. } => {
            rewrite_expr_dynamic_imports(expr, ctx)?
        }
        Expr::FunctionExpression { params, body, .. } => {
            for p in params {
                rewrite_binding_dynamic_imports(&mut p.binding, ctx)?;
                if let Some(default) = &mut p.default {
                    rewrite_expr_dynamic_imports(default, ctx)?;
                }
            }
            rewrite_stmt_dynamic_imports(body, ctx)?;
        }
        Expr::ClassExpression {
            super_class, body, ..
        } => {
            if let Some(super_class) = super_class {
                rewrite_expr_dynamic_imports(super_class, ctx)?;
            }
            rewrite_class_elements_dynamic_imports(body, ctx)?;
        }
        Expr::ArrowFunction { params, body, .. } => {
            for p in params {
                rewrite_binding_dynamic_imports(&mut p.binding, ctx)?;
                if let Some(default) = &mut p.default {
                    rewrite_expr_dynamic_imports(default, ctx)?;
                }
            }
            match body {
                ArrowBody::Expr(expr) => rewrite_expr_dynamic_imports(expr, ctx)?,
                ArrowBody::Block(block) => rewrite_stmt_dynamic_imports(block, ctx)?,
            }
        }
        Expr::ArrayPattern { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern {
                        binding, default, ..
                    } => {
                        rewrite_binding_dynamic_imports(binding, ctx)?;
                        if let Some(default) = default {
                            rewrite_expr_dynamic_imports(default, ctx)?;
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        rewrite_binding_dynamic_imports(binding, ctx)?
                    }
                }
            }
        }
        Expr::ObjectPattern { properties, .. } => {
            for prop in properties {
                match prop {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        default,
                        ..
                    } => {
                        if let ObjectKey::Computed(key) = key {
                            rewrite_expr_dynamic_imports(key, ctx)?;
                        }
                        rewrite_binding_dynamic_imports(binding, ctx)?;
                        if let Some(default) = default {
                            rewrite_expr_dynamic_imports(default, ctx)?;
                        }
                    }
                    ObjectPatternProp::Rest(binding) => {
                        rewrite_binding_dynamic_imports(binding, ctx)?
                    }
                }
            }
        }
    }
    Ok(())
}

impl RewriteCtx<'_> {
    /// Resolve a string-literal module specifier against `importer_dir` to a linked id.
    fn linked_id_for_source(&self, source: &Expr) -> Option<usize> {
        let Expr::String(lit) = source else {
            return None;
        };
        let spec = lit.value.to_string_lossy();
        let spec_path = Path::new(&spec);
        if spec_path.is_absolute() || spec_path.starts_with("http") {
            return None;
        }
        let mut resolved = self.importer_dir.clone();
        for comp in spec_path.components() {
            resolved.push(comp);
        }
        let norm = lexical_normalize_path(&resolved);
        self.ids.get(&norm).copied()
    }

    /// If `source` is a static string referring to a linked module, return the
    /// shared deferred namespace binding name for that module. Returns `None` for
    /// unlinkable specifiers (external URLs, unloaded modules, dynamic sources).
    fn deferred_ident_for_source(&self, source: &Expr) -> Result<Option<String>, Diagnostic> {
        let Some(id) = self.linked_id_for_source(source) else {
            return Ok(None);
        };
        if self.deferred_ns_targets.contains(&id) {
            Ok(Some(deferred_namespace_binding_name(id)))
        } else {
            Ok(None)
        }
    }

    /// E19.84.08: evaluation-phase `import("linked")` → Promise that evaluates the
    /// module (once), rethrows a cached evaluation error, and fulfills with `__ns{id}`.
    fn eval_import_rewrite_for_source(
        &mut self,
        source: &Expr,
        span: Span,
    ) -> Result<Option<Expr>, Diagnostic> {
        let Some(id) = self.linked_id_for_source(source) else {
            return Ok(None);
        };
        let ns = shared_namespace_binding_name(id);
        let eval_fn = deferred_eval_fn_name(id);
        let src = format!(
            r#"Promise.resolve().then(function () {{
  if (typeof {eval_fn} === "function") {{
    {eval_fn}();
  }}
  if (typeof __draconic_merror !== "undefined" && __draconic_merror[{id}] !== undefined) {{
    throw __draconic_merror[{id}];
  }}
  return {ns};
}})"#
        );
        let program = parse(&src)?;
        let Stmt::Expression { mut expr, .. } =
            program.body.into_iter().next().ok_or_else(|| {
                Diagnostic::new("eval import rewrite produced no stmt", span)
                    .with_code(codes::LINKER_INTERNAL)
            })?
        else {
            return Err(
                Diagnostic::new("eval import rewrite expected expression stmt", span)
                    .with_code(codes::LINKER_INTERNAL),
            );
        };
        // Fresh spans per rewrite site — binder/IR key symbols by Span.
        uniqueify_expr_spans(&mut expr, self.spans);
        Ok(Some(expr))
    }
}

#[cfg(test)]
mod tests {
    use crate::{link_entry, temp_link_dir};
    use std::fs;

    #[test]
    fn link_dynamic_import_defer_sync_lazy() {
        // E19.84.06: `import.defer("./dep")` loads dep into the graph as deferred
        // and rewrites to Promise.resolve(__ns_defer…); body stays unevaluated.
        let dir = temp_link_dir("dynamic-import-defer-sync");
        let dep = dir.join("dep.drac");
        let main = dir.join("main.drac");
        fs::write(
            &dep,
            "globalThis.side = (globalThis.side || 0) + 1;\nexport let x = 1;\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import.defer(\"./dep.drac\").then(function (ns) { let v = ns.x; });\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("dynamic import.defer link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("__draconic_deferred_ns") || dump.contains("draconic_deferred"),
            "expected deferred ns helper:\n{dump}"
        );
        assert!(
            dump.contains("Promise") && dump.contains("resolve"),
            "expected Promise.resolve rewrite:\n{dump}"
        );
        assert!(
            dump.contains("__draconic_eval_m") || dump.contains("FunctionDeclaration"),
            "expected deferred eval thunk:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
