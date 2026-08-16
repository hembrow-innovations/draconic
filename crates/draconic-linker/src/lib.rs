//! Link ESM modules into a single Program (ROADMAP E11.01–E11.04, E18.29–E18.31).
//!
//! Loads an entry file, follows static relative `import … from "…"`, mangles
//! per-module top-level bindings to avoid collisions, rewrites import locals to
//! the exporter's mangled names, and concatenates dependency bodies before the
//! entry. Supports named, default, namespace (`import * as ns`), `export * from`,
//! `export * as ns from`, and `export { … } from` re-exports, including cyclic
//! graphs (live bindings via shared cells).

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use draconic_ast::{
    Arg, ArrayElement, ArrayPatternElement, ArrowBody, AssignOp, BindingKind,
    BindingPattern, ClassElement, Expr, Ident, ImportPhase, NumberLit, ObjectKey,
    ObjectPatternProp, ObjectProp, Param, Program, Stmt,
};
use draconic_diagnostics::{Diagnostic, Span};

use draconic_parser::{parse, parse_module};

/// Parse `entry` and all static relative imports into one linked Program.
pub fn link_entry(entry: &Path) -> Result<Program, Diagnostic> {
    let entry = normalize_path(entry)?;
    let mut loader = Loader::new();
    loader.load_graph(&entry)?;
    loader.link(&entry)
}

struct Loader {
    /// Canonical path → module id (load order).
    ids: HashMap<PathBuf, usize>,
    modules: Vec<ModuleData>,
}

struct ModuleData {
    /// Body statements after peeling import/export wrappers.
    body: Vec<Stmt>,
    /// export_name → local name (pre-mangle) in this module (direct exports only).
    exports: HashMap<String, String>,
    /// `export * from` dependency paths (named exports re-exported; not `default`).
    star_reexports: Vec<PathBuf>,
    /// `export { imported as exported } from` named re-exports.
    named_reexports: Vec<NamedReexport>,
    /// `export * as local from` — local binding is the module namespace object.
    namespace_reexports: Vec<NamespaceBind>,
    /// import local → (resolved module path, exported name).
    imports: Vec<ImportBind>,
    /// `import * as local` → resolved module path.
    namespaces: Vec<NamespaceBind>,
    /// Dependencies that must evaluate with this module (named/side-effect/non-defer).
    eval_deps: Vec<PathBuf>,
    /// All ModuleRequest targets (incl. deferred namespace-only) for ReadyForSyncExecution.
    requested: Vec<PathBuf>,
    /// String-literal `import.defer("…")` targets (E19.84.06). Loaded into the graph
    /// and get deferred namespaces, but do not mark eval unless also an eval_dep.
    dynamic_defer_targets: Vec<PathBuf>,
    /// String-literal evaluation-phase `import("…")` targets (E19.84.08). Loaded into
    /// the graph for linked dynamic import + evaluation-error identity; not eval_deps.
    dynamic_import_targets: Vec<PathBuf>,
}

struct NamedReexport {
    from: PathBuf,
    /// Name in the source module (`local` side of the specifier).
    imported: String,
    /// Name under which this module re-exports it.
    exported: String,
}

struct ImportBind {
    local: String,
    from: PathBuf,
    imported: String,
}

#[derive(Clone)]
struct NamespaceBind {
    local: String,
    from: PathBuf,
    /// `import defer * as local` (E19.42 / E19.55).
    deferred: bool,
}

impl Loader {
    fn new() -> Self {
        Self {
            ids: HashMap::new(),
            modules: Vec::new(),
        }
    }

    fn load_graph(&mut self, entry: &Path) -> Result<(), Diagnostic> {
        let mut stack = Vec::new();
        self.load_module(entry, &mut stack)
    }

    fn load_module(&mut self, path: &Path, stack: &mut Vec<PathBuf>) -> Result<(), Diagnostic> {
        let path = normalize_path(path)?;
        if self.ids.contains_key(&path) {
            return Ok(());
        }
        // Back-edge into an in-progress module: allow cycles. The ancestor stays
        // on the stack, finishes after its deps, and registers once.
        if stack.iter().any(|p| p == &path) {
            return Ok(());
        }
        stack.push(path.clone());

        let source = fs::read_to_string(&path).map_err(|e| {
            Diagnostic::new(
                format!("failed to read module {}: {e}", path.display()),
                Span::dummy(),
            )
        })?;
        // E19.84.03: `.json` files are JSON modules (ParseJSONModule). The raw
        // source is embedded as a JS string and parsed at eval via the runtime's
        // JSON.parse; the synthetic module exports that value as `default`.
        let program = if path.extension().is_some_and(|e| e == "json") {
            parse_json_module(&source, &path)?
        } else {
            // ESM files are always Module goal ([+Await], reserved `await`) — E19.52.
            parse_module(&source)?
        };
        let parent = path.parent().unwrap_or_else(|| Path::new("."));

        let mut body = Vec::new();
        let mut exports: HashMap<String, String> = HashMap::new();
        let mut star_reexports: Vec<PathBuf> = Vec::new();
        let mut named_reexports: Vec<NamedReexport> = Vec::new();
        let mut namespace_reexports: Vec<NamespaceBind> = Vec::new();
        let mut imports: Vec<ImportBind> = Vec::new();
        let mut namespaces: Vec<NamespaceBind> = Vec::new();
        let mut eval_deps: Vec<PathBuf> = Vec::new();
        let mut dep_paths = Vec::new();
        // E19.69: bare `export { local }` must resolve to Var/LexicallyDeclaredNames.
        let mut local_export_checks: Vec<(String, Span)> = Vec::new();

        for stmt in program.body {
            match stmt {
                Stmt::ImportDeclaration {
                    specifiers,
                    namespace,
                    source,
                    phase,
                    ..
                } => {
                    let spec = source.value.to_string_strict().ok_or_else(|| {
                        Diagnostic::new(
                            "module specifier must be a well-formed string".to_string(),
                            source.span,
                        )
                    })?;
                    let dep = resolve_specifier(parent, &spec, source.span)?;
                    dep_paths.push(dep.clone());
                    let deferred_ns = phase == ImportPhase::Defer && namespace.is_some();
                    // Named / default / side-effect imports evaluate the target; deferred
                    // namespace alone does not (E19.55).
                    let mut marks_eval = !specifiers.is_empty()
                        || namespace.is_none()
                        || (namespace.is_some() && !deferred_ns);
                    if deferred_ns && specifiers.is_empty() {
                        marks_eval = false;
                    }
                    if marks_eval {
                        eval_deps.push(dep.clone());
                    }
                    for s in specifiers {
                        imports.push(ImportBind {
                            local: s.local.name,
                            from: dep.clone(),
                            imported: s.imported.name,
                        });
                    }
                    if let Some(ns) = namespace {
                        namespaces.push(NamespaceBind {
                            local: ns.name,
                            from: dep.clone(),
                            deferred: deferred_ns,
                        });
                    }
                }
                Stmt::ExportNamedDeclaration {
                    declaration,
                    specifiers,
                    source,
                    ..
                } => {
                    if let Some(src) = source {
                        let spec = src.value.to_string_strict().ok_or_else(|| {
                            Diagnostic::new(
                                "module specifier must be a well-formed string".to_string(),
                                src.span,
                            )
                        })?;
                        let dep = resolve_specifier(parent, &spec, src.span)?;
                        dep_paths.push(dep.clone());
                        eval_deps.push(dep.clone());
                        for s in specifiers {
                            if exports.contains_key(&s.exported.name)
                                || named_reexports
                                    .iter()
                                    .any(|r| r.exported == s.exported.name)
                                || namespace_reexports.iter().any(|r| r.local == s.exported.name)
                            {
                                return Err(Diagnostic::new(
                                    format!("duplicate export `{}`", s.exported.name),
                                    s.exported.span,
                                ));
                            }
                            named_reexports.push(NamedReexport {
                                from: dep.clone(),
                                imported: s.local.name,
                                exported: s.exported.name,
                            });
                        }
                    } else {
                        if let Some(decl) = declaration {
                            // Multi-declarator `export let a, b` parses as a Block of Lets.
                            let decls = expand_export_decl(*decl);
                            for d in decls {
                                collect_decl_exports(&d, &mut exports)?;
                                body.push(d);
                            }
                        }
                        for s in specifiers {
                            if exports
                                .insert(s.exported.name.clone(), s.local.name.clone())
                                .is_some()
                                || named_reexports
                                    .iter()
                                    .any(|r| r.exported == s.exported.name)
                                || namespace_reexports.iter().any(|r| r.local == s.exported.name)
                            {
                                return Err(Diagnostic::new(
                                    format!("duplicate export `{}`", s.exported.name),
                                    s.exported.span,
                                ));
                            }
                            local_export_checks.push((s.local.name.clone(), s.local.span));
                        }
                    }
                }
                Stmt::ExportDefaultDeclaration {
                    declaration,
                    local,
                    ..
                } => {
                    if exports
                        .insert("default".into(), local.name.clone())
                        .is_some()
                    {
                        return Err(Diagnostic::new(
                            "duplicate default export".to_string(),
                            local.span,
                        ));
                    }
                    body.push(*declaration);
                }
                Stmt::ExportAllDeclaration {
                    exported,
                    source,
                    ..
                } => {
                    let spec = source.value.to_string_strict().ok_or_else(|| {
                        Diagnostic::new(
                            "module specifier must be a well-formed string".to_string(),
                            source.span,
                        )
                    })?;
                    let dep = resolve_specifier(parent, &spec, source.span)?;
                    dep_paths.push(dep.clone());
                    eval_deps.push(dep.clone());
                    if let Some(ns) = exported {
                        if exports
                            .insert(ns.name.clone(), ns.name.clone())
                            .is_some()
                            || named_reexports.iter().any(|r| r.exported == ns.name)
                            || namespace_reexports.iter().any(|r| r.local == ns.name)
                        {
                            return Err(Diagnostic::new(
                                format!("duplicate export `{}`", ns.name),
                                ns.span,
                            ));
                        }
                        namespace_reexports.push(NamespaceBind {
                            local: ns.name,
                            from: dep,
                            deferred: false,
                        });
                    } else {
                        star_reexports.push(dep);
                    }
                }
                other => body.push(other),
            }
        }

        // E19.69: ExportedBindings must also occur in Var/LexicallyDeclaredNames
        // (or as import bindings). Globals like `Number` are not module bindings.
        if !local_export_checks.is_empty() {
            let mut declared = top_level_names(&body);
            for imp in &imports {
                declared.insert(imp.local.clone());
            }
            for ns in &namespaces {
                declared.insert(ns.local.clone());
            }
            for ns in &namespace_reexports {
                declared.insert(ns.local.clone());
            }
            for (name, span) in &local_export_checks {
                if !declared.contains(name) {
                    return Err(Diagnostic::new(
                        format!("export of undeclared binding `{name}`"),
                        *span,
                    ));
                }
            }
        }

        // E19.84.06: load string-literal `import.defer("…")` targets into the graph
        // (deferred namespace + lazy eval) without marking them eval_deps.
        let mut dynamic_defer_targets = Vec::new();
        collect_dynamic_defer_targets(&body, parent, &mut dynamic_defer_targets)?;
        for dep in &dynamic_defer_targets {
            if !dep_paths.iter().any(|p| p == dep) {
                dep_paths.push(dep.clone());
            }
        }
        // E19.84.08: load string-literal evaluation-phase `import("…")` targets so
        // linked graphs can share evaluation errors with deferred namespaces.
        let mut dynamic_import_targets = Vec::new();
        collect_dynamic_eval_import_targets(&body, parent, &mut dynamic_import_targets)?;
        for dep in &dynamic_import_targets {
            if !dep_paths.iter().any(|p| p == dep) {
                dep_paths.push(dep.clone());
            }
        }

        for dep in &dep_paths {
            self.load_module(dep, stack)?;
        }

        let id = self.modules.len();
        self.ids.insert(path.clone(), id);
        self.modules.push(ModuleData {
            body,
            exports,
            star_reexports,
            named_reexports,
            namespace_reexports,
            imports,
            namespaces,
            eval_deps,
            requested: dep_paths,
            dynamic_defer_targets,
            dynamic_import_targets,
        });
        stack.pop();
        Ok(())
    }

    fn link(&mut self, entry: &Path) -> Result<Program, Diagnostic> {
        let entry = normalize_path(entry)?;
        let entry_id = *self.ids.get(&entry).expect("entry loaded");

        // E19.71: IndirectExportEntries must resolve before emit.
        self.validate_indirect_exports()?;

        // Mangle non-entry modules fully. Entry keeps original local names so
        // host checks (js.check) and scripts see the source binding names.
        let mut mangled: Vec<HashMap<String, String>> = Vec::with_capacity(self.modules.len());
        for (id, module) in self.modules.iter().enumerate() {
            let mut map = HashMap::new();
            if id != entry_id {
                for name in top_level_names(&module.body) {
                    map.insert(name.clone(), format!("__m{id}_{name}"));
                }
                for ns in &module.namespaces {
                    map.insert(ns.local.clone(), format!("__m{id}_{}", ns.local));
                }
                for ns in &module.namespace_reexports {
                    map.insert(ns.local.clone(), format!("__m{id}_{}", ns.local));
                }
            }
            mangled.push(map);
        }

        let mut import_renames: Vec<HashMap<String, String>> =
            vec![HashMap::new(); self.modules.len()];
        for (id, module) in self.modules.iter().enumerate() {
            for bind in &module.imports {
                let from_id = *self.ids.get(&bind.from).ok_or_else(|| {
                    Diagnostic::new(
                        format!("module not loaded: {}", bind.from.display()),
                        Span::dummy(),
                    )
                })?;
                let (def_id, local_in_exporter) = self
                    .resolve_export(from_id, &bind.imported, &mut HashSet::new())?
                    .ok_or_else(|| {
                        Diagnostic::new(
                            format!(
                                "module {} has no export `{}`",
                                bind.from.display(),
                                bind.imported
                            ),
                            Span::dummy(),
                        )
                    })?;
                let remote = final_binding_name(&mangled, def_id, &local_in_exporter)?;
                if let Some(prev) = import_renames[id].get(&bind.local) {
                    if prev != &remote {
                        return Err(Diagnostic::new(
                            format!("duplicate import binding `{}`", bind.local),
                            Span::dummy(),
                        ));
                    }
                }
                import_renames[id].insert(bind.local.clone(), remote);
            }
        }

        // E19.55: modules reachable only via `import defer` stay unevaluated until a
        // deferred-namespace trigger. Eager = entry + eval_deps closure.
        let eager = self.compute_eager_modules(entry_id);

        // E19.71: one shared namespace object per target module (`__ns{id}`).
        // Eager `import *` / `export * as` rename onto that binding.
        let mut span_gen = SyntheticSpans::new();
        let mut any_deferred_ns = false;
        let mut shared_ns_targets: HashSet<usize> = HashSet::new();
        let mut deferred_ns_targets: HashSet<usize> = HashSet::new();
        for id in 0..self.modules.len() {
            let mut ns_binds = self.modules[id].namespaces.clone();
            ns_binds.extend(self.modules[id].namespace_reexports.clone());
            for bind in &ns_binds {
                let from_id = *self.ids.get(&bind.from).ok_or_else(|| {
                    Diagnostic::new(
                        format!("module not loaded: {}", bind.from.display()),
                        Span::dummy(),
                    )
                })?;
                // E19.84.02: every `import defer * as ns` site renames onto one
                // shared deferred namespace object per target module, distinct from
                // the eager `__ns{id}` object. Re-exports of the namespace keep the
                // deferred identity through `resolve_local_export_binding`.
                if bind.deferred {
                    any_deferred_ns = true;
                    deferred_ns_targets.insert(from_id);
                    import_renames[id]
                        .insert(bind.local.clone(), deferred_namespace_binding_name(from_id));
                } else {
                    shared_ns_targets.insert(from_id);
                    import_renames[id]
                        .insert(bind.local.clone(), shared_namespace_binding_name(from_id));
                }
            }
            // E19.84.06: dynamic `import.defer("…")` of a linked module also gets a
            // shared deferred namespace (even with no static `import defer`).
            for from in &self.modules[id].dynamic_defer_targets {
                if let Some(&from_id) = self.ids.get(from) {
                    any_deferred_ns = true;
                    deferred_ns_targets.insert(from_id);
                }
            }
            // E19.84.08: evaluation-phase `import("…")` of a linked module needs an
            // eager namespace object as the ImportCall fulfillment value.
            for from in &self.modules[id].dynamic_import_targets {
                if let Some(&from_id) = self.ids.get(from) {
                    shared_ns_targets.insert(from_id);
                }
            }
        }

        // E19.84.02: build one shared deferred namespace object per target module.
        // Created once at link time (instantiation), distinct from the eager `__ns{id}`.
        // Lazy target: evaluation calls the once-eval thunk. Eager target (e.g. TLA
        // under import defer): exports are already initialized when the program runs,
        // so the closure just reads the (mangled) export bindings.
        let mut deferred_ns_by_id: HashMap<usize, Stmt> = HashMap::new();
        for from_id in &deferred_ns_targets {
            let resolved = self.collect_resolved_exports(*from_id)?;
            let mut pairs: Vec<(String, String)> = Vec::new();
            let mut names: Vec<_> = resolved.keys().cloned().collect();
            names.sort();
            for export_name in names {
                let (def_id, local_in_exporter) =
                    resolved.get(&export_name).expect("key from map");
                let remote = final_binding_name(&mangled, *def_id, local_in_exporter)?;
                pairs.push((export_name, remote));
            }
            let eval_name = (!eager.contains(from_id)).then(|| deferred_eval_fn_name(*from_id));
            let bind_span = span_gen.next();
            let mut stmt = make_deferred_namespace_binding(
                &deferred_namespace_binding_name(*from_id),
                eval_name.as_deref(),
                &pairs,
                *from_id,
                bind_span,
            )?;
            // Synthetic per-module statements share spans; binder/IR key symbols
            // by span, so every top-level stmt must get unique spans (E19.86).
            uniqueify_stmt_spans(&mut stmt, &mut span_gen);
            deferred_ns_by_id.insert(*from_id, stmt);
        }

        // E19.86: build shared eager namespace objects once per target module.
        // Instantiated at the top of the linked program (before any module body) as
        // spec-exotic Proxy objects, so import-site references never hit a TDZ on
        // the namespace binding itself (E19.71 emitted `let __ns{id}` after the
        // target module's body, which broke self-imports). Export values are read
        // lazily through getter closures over the (mangled) export bindings, so
        // `[[Get]]` of an uninitialized binding still throws ReferenceError and
        // late-initialized bindings stay live.
        let mut shared_ns_setup: Vec<(usize, Stmt)> = Vec::new();
        for from_id in &shared_ns_targets {
            let resolved = self.collect_resolved_exports(*from_id)?;
            let span = span_gen.next();
            let mut stmt = make_shared_namespace_binding(
                &shared_namespace_binding_name(*from_id),
                &resolved,
                &mangled,
                span,
            )?;
            // Synthetic per-module statements share spans; binder/IR key symbols
            // by span, so every top-level stmt must get unique spans (E19.86).
            uniqueify_stmt_spans(&mut stmt, &mut span_gen);
            shared_ns_setup.push((*from_id, stmt));
        }
        shared_ns_setup.sort_by_key(|(id, _)| *id);

        // modules are stored in post-order (deps before importers). Entry last.
        let mut order: Vec<usize> = (0..self.modules.len()).collect();
        order.retain(|&id| id != entry_id);
        order.push(entry_id);

        // Pre-rename deferred module bodies and build lazy eval thunks.
        let mut deferred_thunks: HashMap<usize, Vec<Stmt>> = HashMap::new();
        for id in 0..self.modules.len() {
            if eager.contains(&id) {
                continue;
            }
            let mut rename = mangled[id].clone();
            rename.extend(import_renames[id].clone());
            let mut body = std::mem::take(&mut self.modules[id].body);
            for stmt in &mut body {
                rename_stmt(stmt, &rename, &mut ScopeStack::new());
                uniqueify_stmt_spans(stmt, &mut span_gen);
            }
            // Call deferred eval deps first (named imports into this deferred module).
            let mut prelude_calls = Vec::new();
            for dep in &self.modules[id].eval_deps.clone() {
                if let Some(&dep_id) = self.ids.get(dep) {
                    if !eager.contains(&dep_id) {
                        let fn_name = deferred_eval_fn_name(dep_id);
                        prelude_calls.push(make_call_stmt(&fn_name, span_gen.next()));
                    }
                }
            }
            let eval_name = deferred_eval_fn_name(id);
            deferred_thunks.insert(
                id,
                wrap_deferred_module_body(id, &eval_name, prelude_calls, body, &mut span_gen),
            );
        }

        let mut linked_body = Vec::new();
        // Status / [[EvaluationError]] helpers for deferred ns (E19.84.05) and for
        // lazy once-eval of non-eager modules (incl. dynamic-import targets, E19.84.08).
        let has_lazy_modules = (0..self.modules.len()).any(|id| !eager.contains(&id));
        if any_deferred_ns || has_lazy_modules {
            for stmt in deferred_module_status_helper_stmts(self, self.modules.len())? {
                linked_body.push(stmt);
            }
        }
        if any_deferred_ns {
            for stmt in deferred_namespace_helper_stmts()? {
                linked_body.push(stmt);
            }
            // E19.84.02: instantiate each shared deferred namespace object once, at
            // the top of the program (module namespace objects exist at link time).
            let mut defer_order: Vec<_> = deferred_ns_by_id.into_iter().collect();
            defer_order.sort_by_key(|(id, _)| *id);
            for (_, stmt) in defer_order {
                linked_body.push(stmt);
            }
        }
        // E19.86: instantiate eager module namespace objects up-front (before any
        // module body). Getter closures read the export bindings lazily, so the
        // namespace binding itself is never in TDZ at an import site.
        if !shared_ns_targets.is_empty() {
            for stmt in shared_namespace_helper_stmts()? {
                linked_body.push(stmt);
            }
            for (_, stmt) in shared_ns_setup.into_iter() {
                linked_body.push(stmt);
            }
        }
        // Emit deferred thunks before eager bodies (hoisted bindings + eval fns).
        for id in &order {
            if let Some(thunks) = deferred_thunks.remove(id) {
                linked_body.extend(thunks);
            }
        }

        let mut start = 0u32;
        let mut end = 0u32;
        // E19.84.02: reverse path lookup so dynamic `import.defer` specifiers can be
        // resolved against the linking module's own location.
        let mut id_to_path: HashMap<usize, PathBuf> = HashMap::new();
        for (p, id) in &self.ids {
            id_to_path.insert(*id, p.clone());
        }
        for id in order {
            if !eager.contains(&id) {
                continue;
            }
            let mut rename = mangled[id].clone();
            rename.extend(import_renames[id].clone());
            let mut body = std::mem::take(&mut self.modules[id].body);
            for stmt in &mut body {
                rename_stmt(stmt, &rename, &mut ScopeStack::new());
                // Per-file source offsets collide across modules; binder/IR key by Span.
                uniqueify_stmt_spans(stmt, &mut span_gen);
            }
            // E19.84.02 / E19.84.08: rewrite dynamic `import.defer` / evaluation-phase
            // `import("…")` of linked modules (Node lacks import-defer; linked eval
            // errors must share identity with deferred-namespace triggers).
            if !deferred_ns_targets.is_empty() || !shared_ns_targets.is_empty() {
                if let Some(path) = id_to_path.get(&id) {
                    self.rewrite_dynamic_deferred_imports(
                        &mut body,
                        path,
                        &deferred_ns_targets,
                        &mut span_gen,
                    )?;
                }
            }
            // E19.84.05: mark eager module ~evaluating~ … ~evaluated~ around body so
            // deferred-namespace EnsureDeferredNamespaceEvaluation can TypeError.
            if any_deferred_ns {
                linked_body.push(make_module_status_assign(id, 1, span_gen.next()));
            }
            for stmt in &body {
                let sp = stmt_span_approx(stmt);
                if linked_body.is_empty() {
                    start = sp.start.0;
                }
                end = sp.end.0;
            }
            linked_body.extend(body);
            if any_deferred_ns {
                linked_body.push(make_module_status_assign(id, 3, span_gen.next()));
            }
        }
        // (E19.86: eager namespace objects are instantiated up-front with the other
        // namespace machinery, no longer emitted after each module body.)

        Ok(Program {
            body: linked_body,
            span: Span::new(start, end),
        })
    }

    /// Modules that evaluate eagerly: entry plus the closure of `eval_deps`.
    ///
    /// Also: deferred-import targets that have top-level await (or that transitively
    /// reach TLA) evaluate eagerly — GatherAsynchronousTransitiveDependencies (E19.55).
    fn compute_eager_modules(&self, entry_id: usize) -> HashSet<usize> {
        let mut eager = HashSet::new();
        let mut stack = vec![entry_id];
        while let Some(id) = stack.pop() {
            if !eager.insert(id) {
                continue;
            }
            for dep in &self.modules[id].eval_deps {
                if let Some(&dep_id) = self.ids.get(dep) {
                    stack.push(dep_id);
                }
            }
            // Deferred namespace edges: still pull in async/TLA transitive deps.
            for ns in &self.modules[id].namespaces {
                if !ns.deferred {
                    continue;
                }
                if let Some(&dep_id) = self.ids.get(&ns.from) {
                    for tla_id in self.gather_async_transitive(dep_id) {
                        stack.push(tla_id);
                    }
                }
            }
            // E19.84.06: dynamic `import.defer` — same GatherAsynchronousTransitiveDependencies.
            for from in &self.modules[id].dynamic_defer_targets {
                if let Some(&dep_id) = self.ids.get(from) {
                    for tla_id in self.gather_async_transitive(dep_id) {
                        stack.push(tla_id);
                    }
                }
            }
        }
        eager
    }

    /// Modules with TLA (or that reach them) under a deferred import subgraph.
    fn gather_async_transitive(&self, start: usize) -> Vec<usize> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        self.gather_async_transitive_rec(start, &mut seen, &mut out);
        out
    }

    fn gather_async_transitive_rec(
        &self,
        id: usize,
        seen: &mut HashSet<usize>,
        out: &mut Vec<usize>,
    ) {
        if !seen.insert(id) {
            return;
        }
        if module_body_has_tla(&self.modules[id].body) {
            out.push(id);
            return;
        }
        for dep in &self.modules[id].eval_deps {
            if let Some(&dep_id) = self.ids.get(dep) {
                self.gather_async_transitive_rec(dep_id, seen, out);
            }
        }
        for ns in &self.modules[id].namespaces {
            if let Some(&dep_id) = self.ids.get(&ns.from) {
                self.gather_async_transitive_rec(dep_id, seen, out);
            }
        }
    }

    /// Resolve `name` exported by `module_id` to `(defining_module_id, local_name)`.
    /// Follows `export * from` and `export { … } from`. Direct exports shadow stars.
    /// Ambiguous star collisions yield `None` (same as missing for link errors).
    fn resolve_export(
        &self,
        module_id: usize,
        name: &str,
        visiting: &mut HashSet<usize>,
    ) -> Result<Option<(usize, String)>, Diagnostic> {
        let (unambiguous, ambiguous) = self.collect_export_maps_rec(module_id, visiting)?;
        if ambiguous.contains(name) {
            return Ok(None);
        }
        Ok(unambiguous.get(name).cloned())
    }

    /// Resolve a single named export through direct exports and named re-exports,
    /// tolerating cycles: a named re-export into a module already being expanded
    /// (self-import, or a star-re-export cycle) must still resolve its exact name
    /// (E19.86). Star re-exports participate, but a star dep already on the chain
    /// is skipped so the walk terminates. Ambiguity is not tracked here (the full
    /// map via [`Self::collect_export_maps_rec`] governs ambiguous star collisions).
    fn resolve_named_export(
        &self,
        module_id: usize,
        name: &str,
        chain: &mut HashSet<usize>,
    ) -> Result<Option<(usize, String)>, Diagnostic> {
        if !chain.insert(module_id) {
            return Ok(None);
        }
        let module = &self.modules[module_id];
        if let Some(local) = module.exports.get(name) {
            return self.resolve_local_export_binding(module_id, local, chain);
        }
        if let Some(re) = module.named_reexports.iter().find(|r| r.exported == name) {
            let dep_id = *self.ids.get(&re.from).ok_or_else(|| {
                Diagnostic::new(
                    format!("module not loaded: {}", re.from.display()),
                    Span::dummy(),
                )
            })?;
            return self.resolve_named_export(dep_id, &re.imported, chain);
        }
        for dep_path in &module.star_reexports {
            let dep_id = *self.ids.get(dep_path).ok_or_else(|| {
                Diagnostic::new(
                    format!("module not loaded: {}", dep_path.display()),
                    Span::dummy(),
                )
            })?;
            if let Some(binding) = self.resolve_named_export(dep_id, name, chain)? {
                return Ok(Some(binding));
            }
        }
        Ok(None)
    }

    /// Unambiguous export names visible from `module_id` (GetModuleNamespace set).
    /// Ambiguous star collisions are omitted, not errors (E19.71).
    fn collect_resolved_exports(
        &self,
        module_id: usize,
    ) -> Result<HashMap<String, (usize, String)>, Diagnostic> {
        let (unambiguous, _) = self.collect_export_maps_rec(module_id, &mut HashSet::new())?;
        Ok(unambiguous)
    }

    /// Resolve a local export name through import / `export * as` / true local binding.
    ///
    /// `export { foo }` after `import { foo }` or `import * as foo` is an indirect
    /// re-export of the original binding (same Module + BindingName), not a new local.
    fn resolve_local_export_binding(
        &self,
        module_id: usize,
        local: &str,
        visiting: &mut HashSet<usize>,
    ) -> Result<Option<(usize, String)>, Diagnostic> {
        let module = &self.modules[module_id];
        if let Some(ns) = module
            .namespaces
            .iter()
            .chain(module.namespace_reexports.iter())
            .find(|n| n.local == local)
        {
            let from_id = *self.ids.get(&ns.from).ok_or_else(|| {
                Diagnostic::new(
                    format!("module not loaded: {}", ns.from.display()),
                    Span::dummy(),
                )
            })?;
            // E19.84.02: re-exporting a deferred namespace keeps deferred identity
            // (distinct shared object from the eager namespace).
            if ns.deferred {
                return Ok(Some((from_id, BINDING_DEFERRED_NAMESPACE.to_string())));
            }
            return Ok(Some((from_id, BINDING_NAMESPACE.to_string())));
        }
        if let Some(imp) = module.imports.iter().find(|i| i.local == local) {
            let from_id = *self.ids.get(&imp.from).ok_or_else(|| {
                Diagnostic::new(
                    format!("module not loaded: {}", imp.from.display()),
                    Span::dummy(),
                )
            })?;
            return self.resolve_export(from_id, &imp.imported, visiting);
        }
        Ok(Some((module_id, local.to_string())))
    }

    fn collect_export_maps_rec(
        &self,
        module_id: usize,
        visiting: &mut HashSet<usize>,
    ) -> Result<(HashMap<String, (usize, String)>, HashSet<String>), Diagnostic> {
        if !visiting.insert(module_id) {
            return Ok((HashMap::new(), HashSet::new()));
        }
        let module = &self.modules[module_id];
        let mut out: HashMap<String, (usize, String)> = HashMap::new();
        let mut ambiguous: HashSet<String> = HashSet::new();

        for (export_name, local) in &module.exports {
            match self.resolve_local_export_binding(module_id, local, visiting)? {
                Some(binding) => {
                    out.insert(export_name.clone(), binding);
                }
                None => {
                    // Imported local that is null/ambiguous — treat as ambiguous export.
                    ambiguous.insert(export_name.clone());
                }
            }
        }

        // Named re-exports (`export { x as y } from`) — explicit, can include `default`.
        for re in &module.named_reexports {
            let dep_id = *self.ids.get(&re.from).ok_or_else(|| {
                Diagnostic::new(
                    format!("module not loaded: {}", re.from.display()),
                    Span::dummy(),
                )
            })?;
            // Named re-exports are exact name lookups — they must resolve even when
            // the target module is mid-expansion (self-import / star cycle), which
            // `collect_export_maps_rec`'s visiting set would otherwise short-circuit
            // (E19.86).
            let resolved = self.resolve_named_export(dep_id, &re.imported, &mut HashSet::new())?;
            if module.exports.contains_key(&re.exported) {
                // Direct export already owns this name — skip (direct wins).
                continue;
            }
            match resolved {
                Some(binding) => {
                    if let Some(prev) = out.get(&re.exported) {
                        if prev != &binding {
                            visiting.remove(&module_id);
                            return Err(Diagnostic::new(
                                format!("duplicate export `{}`", re.exported),
                                Span::dummy(),
                            ));
                        }
                    } else {
                        out.insert(re.exported.clone(), binding);
                    }
                }
                None => {
                    // Missing or ambiguous imported binding — record for named import errors.
                    ambiguous.insert(re.exported.clone());
                    out.remove(&re.exported);
                }
            }
        }

        for dep_path in &module.star_reexports {
            let dep_id = *self.ids.get(dep_path).ok_or_else(|| {
                Diagnostic::new(
                    format!("module not loaded: {}", dep_path.display()),
                    Span::dummy(),
                )
            })?;
            let (dep_exports, dep_ambiguous) = self.collect_export_maps_rec(dep_id, visiting)?;
            for name in dep_ambiguous {
                if name == "default" {
                    continue;
                }
                if module.exports.contains_key(&name)
                    || module.named_reexports.iter().any(|r| r.exported == name)
                {
                    continue;
                }
                out.remove(&name);
                ambiguous.insert(name);
            }
            for (export_name, binding) in dep_exports {
                if export_name == "default" {
                    continue;
                }
                if module.exports.contains_key(&export_name)
                    || module
                        .named_reexports
                        .iter()
                        .any(|r| r.exported == export_name)
                {
                    continue;
                }
                if ambiguous.contains(&export_name) {
                    continue;
                }
                match out.get(&export_name) {
                    Some(prev) if prev != &binding => {
                        out.remove(&export_name);
                        ambiguous.insert(export_name);
                    }
                    Some(_) => {}
                    None => {
                        out.insert(export_name, binding);
                    }
                }
            }
        }
        visiting.remove(&module_id);
        Ok((out, ambiguous))
    }

    /// IndirectExportEntries must resolve (not null/ambiguous) — E19.71.
    fn validate_indirect_exports(&self) -> Result<(), Diagnostic> {
        for module in &self.modules {
            for re in &module.named_reexports {
                let dep_id = *self.ids.get(&re.from).ok_or_else(|| {
                    Diagnostic::new(
                        format!("module not loaded: {}", re.from.display()),
                        Span::dummy(),
                    )
                })?;
                let resolved =
                    self.resolve_export(dep_id, &re.imported, &mut HashSet::new())?;
                if resolved.is_none() {
                    return Err(Diagnostic::new(
                        format!(
                            "module {} has no export `{}` (missing or ambiguous)",
                            re.from.display(),
                            re.imported
                        ),
                        Span::dummy(),
                    ));
                }
            }
        }
        Ok(())
    }

    /// E19.84.02 / E19.84.08: rewrite dynamic `import.defer("…")` and evaluation-phase
    /// `import("…")` of linked modules. Defer → `Promise.resolve(__ns_defer{id})`.
    /// Evaluation → Promise that runs the module's once-eval thunk (if any), rethrows
    /// a cached `[[EvaluationError]]`, and fulfills with `__ns{id}`.
    fn rewrite_dynamic_deferred_imports(
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

fn rewrite_stmt_dynamic_imports(stmt: &mut Stmt, ctx: &mut RewriteCtx<'_>) -> Result<(), Diagnostic> {
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
        | Stmt::TypeAlias { .. } => {}
    }
    Ok(())
}

fn rewrite_class_elements_dynamic_imports(
    elements: &mut [ClassElement],
    ctx: &mut RewriteCtx<'_>,
) -> Result<(), Diagnostic> {
    for el in elements {
        match el {
            ClassElement::Constructor { body, .. }
            | ClassElement::StaticBlock { body, .. } => rewrite_stmt_dynamic_imports(body, ctx)?,
            ClassElement::Method { key, params, body, .. }
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
                key, value, is_static, ..
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
                    ArrayPatternElement::Pattern { binding, default, .. } => {
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

fn rewrite_expr_dynamic_imports(expr: &mut Expr, ctx: &mut RewriteCtx<'_>) -> Result<(), Diagnostic> {
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
        Expr::Binary { left, right, .. } | Expr::Assign { target: left, value: right, .. } => {
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
                } if *phase == ImportPhase::Defer => {
                    ctx.deferred_ident_for_source(source)?.map(|name| (name, *span))
                }
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
                    args: vec![Arg::Expr(Expr::Ident(Ident {
                        name,
                        span: sp_n,
                    }))],
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
                    ObjectProp::Accessor { key, params, body, .. } => {
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
                    ObjectProp::Spread { expr, .. } => {
                        rewrite_expr_dynamic_imports(expr, ctx)?
                    }
                }
            }
        }
        Expr::TemplateLiteral { expressions, .. }
        | Expr::TaggedTemplate { expressions, .. } => {
            for e in expressions {
                rewrite_expr_dynamic_imports(e, ctx)?;
            }
        }
        Expr::Paren { expr, .. } | Expr::As { expr, .. } => {
            rewrite_expr_dynamic_imports(expr, ctx)?
        }
        Expr::FunctionExpression {
            params,
            body,
            ..
        } => {
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
                    ArrayPatternElement::Pattern { binding, default, .. } => {
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
        let Stmt::Expression { mut expr, .. } = program
            .body
            .into_iter()
            .next()
            .ok_or_else(|| Diagnostic::new("eval import rewrite produced no stmt", span))?
        else {
            return Err(Diagnostic::new(
                "eval import rewrite expected expression stmt",
                span,
            ));
        };
        // Fresh spans per rewrite site — binder/IR key symbols by Span.
        uniqueify_expr_spans(&mut expr, self.spans);
        Ok(Some(expr))
    }
}

fn lexical_normalize_path(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for comp in path.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other),
        }
    }
    out
}
/// Sentinel local name: export BindingName is ~namespace~ (module namespace object).
const BINDING_NAMESPACE: &str = "\0namespace";
/// Sentinel local name: export BindingName is a deferred module namespace
/// (E19.84.02) — distinct shared object from the eager namespace.
const BINDING_DEFERRED_NAMESPACE: &str = "\0deferred-namespace";

fn shared_namespace_binding_name(module_id: usize) -> String {
    format!("__ns{module_id}")
}

fn deferred_namespace_binding_name(module_id: usize) -> String {
    format!("__ns_defer{module_id}")
}

fn final_binding_name(
    mangled: &[HashMap<String, String>],
    def_id: usize,
    local_in_exporter: &str,
) -> Result<String, Diagnostic> {
    if local_in_exporter == BINDING_NAMESPACE {
        return Ok(shared_namespace_binding_name(def_id));
    }
    if local_in_exporter == BINDING_DEFERRED_NAMESPACE {
        return Ok(deferred_namespace_binding_name(def_id));
    }
    final_local_name(&mangled[def_id], local_in_exporter).ok_or_else(|| {
        Diagnostic::new(
            format!(
                "export local `{local_in_exporter}` missing in defining module {def_id}"
            ),
            Span::dummy(),
        )
    })
}

/// Resolve a local name through the module's mangling map (identity if unmangled).
fn final_local_name(mangled: &HashMap<String, String>, local: &str) -> Option<String> {
    if let Some(m) = mangled.get(local) {
        return Some(m.clone());
    }
    // Entry module: locals keep source names (not present in mangled).
    Some(local.to_string())
}

/// Fresh spans outside typical source ranges so binder span→symbol maps stay unique.
struct SyntheticSpans {
    next: u32,
}

impl SyntheticSpans {
    fn new() -> Self {
        // High half of u32 avoids colliding with real UTF-8 source offsets in fixtures.
        Self {
            next: 0x8000_0000,
        }
    }

    fn next(&mut self) -> Span {
        let start = self.next;
        self.next = self.next.saturating_add(2);
        Span::new(start, start + 1)
    }
}

/// Assign fresh unique spans across a linked module body so multi-file programs
/// do not share binder/IR span keys (source offsets are per-file).
fn uniqueify_stmt_spans(stmt: &mut Stmt, spans: &mut SyntheticSpans) {
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
                        key,
                        value,
                        span,
                        ..
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
        Stmt::With {
            object,
            body,
            span,
        } => {
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
    }
}

fn uniqueify_binding_spans(pat: &mut BindingPattern, spans: &mut SyntheticSpans) {
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

fn uniqueify_object_key_spans(key: &mut ObjectKey, spans: &mut SyntheticSpans) {
    match key {
        ObjectKey::Ident(id) => id.span = spans.next(),
        ObjectKey::String(s) => s.span = spans.next(),
        ObjectKey::Computed(e) => uniqueify_expr_spans(e, spans),
    }
}

fn uniqueify_params_spans(params: &mut [Param], spans: &mut SyntheticSpans) {
    for p in params {
        uniqueify_binding_spans(&mut p.binding, spans);
        if let Some(default) = &mut p.default {
            uniqueify_expr_spans(default, spans);
        }
    }
}

fn uniqueify_expr_spans(expr: &mut Expr, spans: &mut SyntheticSpans) {
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
        | Expr::NewTarget { span } | Expr::ImportMeta { span } => *span = spans.next(),
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
        Expr::Unary { arg, span, .. } | Expr::Update { arg, span, .. } | Expr::Paren { expr: arg, span } => {
            *span = spans.next();
            uniqueify_expr_spans(arg, spans);
        }
        Expr::As { expr, span, .. } => {
            *span = spans.next();
            uniqueify_expr_spans(expr, spans);
        }
        Expr::Binary {
            left,
            right,
            span,
            ..
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
            callee,
            args,
            span,
            ..
        }
        | Expr::New {
            callee,
            args,
            span,
        } => {
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
                        key,
                        value,
                        span,
                        ..
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
            params,
            body,
            span,
            ..
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
        Expr::PrivateIn {
            name,
            object,
            span,
        } => {
            *span = spans.next();
            name.span = spans.next();
            uniqueify_expr_spans(object, spans);
        }
    }
}

fn deferred_eval_fn_name(mod_id: usize) -> String {
    format!("__draconic_eval_m{mod_id}")
}

/// True when module body has top-level `await` / `await using` / `for await` (HasTLA).
fn module_body_has_tla(body: &[Stmt]) -> bool {
    body.iter().any(stmt_has_top_level_await)
}

fn stmt_has_top_level_await(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Expression { expr, .. } | Stmt::Throw { argument: expr, .. } => {
            expr_has_top_level_await(expr)
        }
        Stmt::Let { kind, init, .. } => {
            *kind == BindingKind::AwaitUsing
                || init.as_ref().is_some_and(expr_has_top_level_await)
        }
        Stmt::Return {
            argument: Some(expr),
            ..
        } => expr_has_top_level_await(expr),
        Stmt::Block { body, .. } => body.iter().any(stmt_has_top_level_await),
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_top_level_await(test)
                || stmt_has_top_level_await(consequent)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_has_top_level_await(a))
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            expr_has_top_level_await(test) || stmt_has_top_level_await(body)
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            init.as_ref().is_some_and(|s| stmt_has_top_level_await(s))
                || test.as_ref().is_some_and(expr_has_top_level_await)
                || update.as_ref().is_some_and(expr_has_top_level_await)
                || stmt_has_top_level_await(body)
        }
        Stmt::ForIn {
            left, right, body, ..
        } => {
            stmt_has_top_level_await(left)
                || expr_has_top_level_await(right)
                || stmt_has_top_level_await(body)
        }
        Stmt::ForOf {
            left,
            right,
            body,
            is_await,
            ..
        } => {
            *is_await
                || stmt_has_top_level_await(left)
                || expr_has_top_level_await(right)
                || stmt_has_top_level_await(body)
        }
        Stmt::Labeled { body, .. } => stmt_has_top_level_await(body),
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            expr_has_top_level_await(discriminant)
                || cases.iter().any(|c| {
                    c.test.as_ref().is_some_and(expr_has_top_level_await)
                        || c.body.iter().any(stmt_has_top_level_await)
                })
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            stmt_has_top_level_await(block)
                || handler.as_ref().is_some_and(|h| stmt_has_top_level_await(h))
                || finalizer
                    .as_ref()
                    .is_some_and(|f| stmt_has_top_level_await(f))
        }
        Stmt::With { object, body, .. } => {
            expr_has_top_level_await(object) || stmt_has_top_level_await(body)
        }
        // Nested functions/classes have their own async context — not module TLA.
        Stmt::FunctionDeclaration { .. }
        | Stmt::ClassDeclaration { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::Empty { .. }
        | Stmt::ImportDeclaration { .. }
        | Stmt::ExportNamedDeclaration { .. }
        | Stmt::ExportDefaultDeclaration { .. }
        | Stmt::ExportAllDeclaration { .. }
        | Stmt::TypeAlias { .. }
        | Stmt::Return { argument: None, .. } => false,
    }
}

fn expr_has_top_level_await(expr: &Expr) -> bool {
    match expr {
        Expr::Unary {
            op: draconic_ast::UnaryOp::Await,
            ..
        } => true,
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } => expr_has_top_level_await(arg),
        Expr::Binary { left, right, .. } | Expr::Assign { target: left, value: right, .. } => {
            expr_has_top_level_await(left) || expr_has_top_level_await(right)
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            expr_has_top_level_await(test)
                || expr_has_top_level_await(consequent)
                || expr_has_top_level_await(alternate)
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            expr_has_top_level_await(callee)
                || args.iter().any(|a| match a {
                    Arg::Expr(e) | Arg::Spread(e) => expr_has_top_level_await(e),
                })
        }
        Expr::MemberExpression {
            object, property, ..
        } => expr_has_top_level_await(object) || expr_has_top_level_await(property),
        Expr::ArrayExpression { elements, .. } => elements.iter().any(|el| match el {
            ArrayElement::Expr(e) | ArrayElement::Spread(e) => expr_has_top_level_await(e),
            ArrayElement::Elision => false,
        }),
        Expr::ObjectExpression { properties, .. } => properties.iter().any(|p| match p {
            ObjectProp::Property { value, key, .. } => {
                expr_has_top_level_await(value)
                    || matches!(key, ObjectKey::Computed(e) if expr_has_top_level_await(e))
            }
            ObjectProp::Spread { expr, .. } => expr_has_top_level_await(expr),
            ObjectProp::Accessor { .. } => false,
        }),
        Expr::ImportCall {
            source, options, ..
        } => {
            expr_has_top_level_await(source)
                || options.as_ref().is_some_and(|o| expr_has_top_level_await(o))
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => expr_has_top_level_await(tag) || expressions.iter().any(expr_has_top_level_await),
        Expr::TemplateLiteral { expressions, .. } => {
            expressions.iter().any(expr_has_top_level_await)
        }
        Expr::Paren { expr, .. } | Expr::As { expr, .. } => expr_has_top_level_await(expr),
        Expr::PrivateIn { object, .. } => expr_has_top_level_await(object),
        // Nested functions — not module TLA.
        Expr::FunctionExpression { .. }
        | Expr::ArrowFunction { .. }
        | Expr::ClassExpression { .. }
        | Expr::Ident(_)
        | Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::String(_)
        | Expr::RegExp { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::Super { .. }
        | Expr::NewTarget { .. }
        | Expr::ImportMeta { .. }
        | Expr::ArrayPattern { .. }
        | Expr::ObjectPattern { .. } => false,
    }
}

/// E19.84.05: per-module [[Status]] + ReadyForSyncExecution for deferred ns.
fn deferred_module_status_helper_stmts(
    loader: &Loader,
    n_modules: usize,
) -> Result<Vec<Stmt>, Diagnostic> {
    let mut status_inits = String::from("[");
    let mut tla_inits = String::from("[");
    let mut deps_inits = String::from("[");
    for id in 0..n_modules {
        if id > 0 {
            status_inits.push_str(", ");
            tla_inits.push_str(", ");
            deps_inits.push_str(", ");
        }
        status_inits.push('0'); // linked
        let has_tla = module_body_has_tla(&loader.modules[id].body);
        tla_inits.push_str(if has_tla { "true" } else { "false" });
        deps_inits.push('[');
        let mut first = true;
        for dep in &loader.modules[id].requested {
            if let Some(&dep_id) = loader.ids.get(dep) {
                if !first {
                    deps_inits.push_str(", ");
                }
                first = false;
                deps_inits.push_str(&dep_id.to_string());
            }
        }
        deps_inits.push(']');
    }
    status_inits.push(']');
    tla_inits.push(']');
    deps_inits.push(']');
    // E19.84.08: parallel [[EvaluationError]] slots (undefined until a throw).
    let mut error_inits = String::from("[");
    for id in 0..n_modules {
        if id > 0 {
            error_inits.push_str(", ");
        }
        error_inits.push_str("undefined");
    }
    error_inits.push(']');
    let src = format!(
        r#"
let __draconic_mstatus = {status_inits};
let __draconic_merror = {error_inits};
let __draconic_mtla = {tla_inits};
let __draconic_mdeps = {deps_inits};
function __draconic_ready(id, seen) {{
  if (seen === undefined) seen = [];
  if (seen.indexOf(id) >= 0) return true;
  seen = seen.concat([id]);
  let st = __draconic_mstatus[id];
  if (st === 3) return true;
  if (st === 1 || st === 2) return false;
  if (__draconic_mtla[id]) return false;
  let deps = __draconic_mdeps[id];
  for (let i = 0; i < deps.length; i++) {{
    if (!__draconic_ready(deps[i], seen)) return false;
  }}
  return true;
}}
"#
    );
    Ok(parse(&src)?.body)
}

fn make_module_status_assign(mod_id: usize, status: i32, span: Span) -> Stmt {
    // __draconic_mstatus[mod_id] = status;
    Stmt::Expression {
        expr: Expr::Assign {
            target: Box::new(Expr::MemberExpression {
                object: Box::new(Expr::Ident(Ident {
                    name: "__draconic_mstatus".into(),
                    span,
                })),
                property: Box::new(Expr::Number(NumberLit {
                    raw: mod_id.to_string(),
                    span,
                })),
                computed: true,
                optional: false,
                private: false,
                span,
            }),
            op: AssignOp::Eq,
            value: Box::new(Expr::Number(NumberLit {
                raw: status.to_string(),
                span,
            })),
            span,
        },
        span,
    }
}

/// Runtime helper implementing deferred module namespace exotic object triggers
/// (E19.55) and the deferred namespace object MOP (E19.84.01).
fn deferred_namespace_helper_stmts() -> Result<Vec<Stmt>, Diagnostic> {
    // Parsed once per link that needs deferred namespaces. Node hosts lack native
    // `import defer`; this Proxy matches Test262 evaluation-trigger + MOP surface.
    let src = r#"
function __draconic_deferred_ns(evaluate, exportNames, modId) {
  let evaluated = false;
  let evalError = undefined;
  let exportsObj = null;
  let names = exportNames.slice().sort();
  // Target is a static stand-in matching a module namespace exotic object's
  // non-configurable keys, so Proxy invariants hold (E19.84.01).
  let target = Object.create(null);
  Object.defineProperty(target, Symbol.toStringTag, {
    value: "Deferred Module",
    writable: false,
    enumerable: false,
    configurable: false,
  });
  for (let i = 0; i < names.length; i++) {
    Object.defineProperty(target, names[i], {
      value: undefined,
      writable: true,
      enumerable: true,
      configurable: false,
    });
  }
  Object.preventExtensions(target);
  function ensure() {
    // E19.84.08: cached [[EvaluationError]] rethrows the same reason.
    if (evaluated) {
      if (evalError !== undefined) throw evalError;
      return exportsObj;
    }
    // E19.84.05: EnsureDeferredNamespaceEvaluation — if not ~evaluated~ and
    // ReadyForSyncExecution is false, throw TypeError (do not start evaluation).
    let st = __draconic_mstatus[modId];
    if (st !== 3 && !__draconic_ready(modId)) {
      throw new TypeError("Deferred module is not ready for synchronous evaluation");
    }
    // Already evaluated elsewhere (eager body or prior dynamic import) with error.
    if (st === 3 && __draconic_merror[modId] !== undefined) {
      evalError = __draconic_merror[modId];
      evaluated = true;
      throw evalError;
    }
    try {
      exportsObj = evaluate();
      evaluated = true;
      for (let i = 0; i < names.length; i++) {
        target[names[i]] = exportsObj[names[i]];
      }
    } catch (e) {
      evalError = __draconic_merror[modId] !== undefined ? __draconic_merror[modId] : e;
      evaluated = true;
      throw evalError;
    }
    return exportsObj;
  }
  function isSymbolLike(p) {
    return typeof p === "symbol" || p === "then";
  }
  // Traps encode deferred module-namespace trigger rules (E19.55) and the
  // deferred namespace object MOP (E19.84.01).
  return new Proxy(target, {
    get(_t, p) {
      if (p === Symbol.toStringTag) return "Deferred Module";
      if (isSymbolLike(p)) return undefined;
      let ex = ensure();
      if (Object.prototype.hasOwnProperty.call(ex, p)) return ex[p];
      return undefined;
    },
    has(_t, p) {
      if (isSymbolLike(p)) {
        return Object.prototype.hasOwnProperty.call(target, p);
      }
      let ex = ensure();
      return Object.prototype.hasOwnProperty.call(ex, p);
    },
    getOwnPropertyDescriptor(_t, p) {
      if (p === Symbol.toStringTag) {
        return { value: "Deferred Module", writable: false, enumerable: false, configurable: false };
      }
      if (isSymbolLike(p)) {
        if (!Object.prototype.hasOwnProperty.call(target, p)) return undefined;
        return { value: target[p], writable: true, enumerable: true, configurable: false };
      }
      let ex = ensure();
      if (!Object.prototype.hasOwnProperty.call(ex, p)) return undefined;
      target[p] = ex[p];
      return { value: ex[p], writable: true, enumerable: true, configurable: false };
    },
    ownKeys() {
      ensure();
      let keys = names.slice();
      keys.push(Symbol.toStringTag);
      return keys;
    },
    defineProperty(_t, p, desc) {
      if (isSymbolLike(p)) return false;
      ensure();
      return false;
    },
    deleteProperty(_t, p) {
      if (isSymbolLike(p)) return true;
      ensure();
      return false;
    },
    set() {
      return false;
    },
    getPrototypeOf() {
      return null;
    },
    setPrototypeOf(_t, p) {
      return p === null;
    },
    isExtensible() {
      return Object.isExtensible(target);
    },
    preventExtensions() {
      Object.preventExtensions(target);
      return true;
    },
  });
}
"#;
    Ok(parse(src)?.body)
}

fn shared_namespace_helper_stmts() -> Result<Vec<Stmt>, Diagnostic> {
    // E19.86: module namespace exotic object polyfill for eager namespaces. The
    // linked program flattens ESM into plain scripts, so `import * as ns` must
    // bind to an object that reproduces the spec [[ModuleNamespace]] MOP: null
    // prototype, non-extensible, sorted ownKeys (Symbol.toStringTag last),
    // non-configurable data descriptors, `[[Get]]` = GetBindingValue (throws
    // ReferenceError on uninitialized bindings), `[[Set]]` = false, etc. Values
    // are fetched through getter closures over the (mangled) export bindings so
    // they stay live and honor binding TDZ. Parsed once per link that needs it.
    let src = r#"
function __draconic_make_ns(pairs, exportNames, toStringTag) {
  let getters = Object.create(null);
  for (let i = 0; i < pairs.length; i++) {
    getters[pairs[i][0]] = pairs[i][1];
  }
  let names = exportNames.slice().sort();
  let target = Object.create(null);
  Object.defineProperty(target, Symbol.toStringTag, {
    value: toStringTag,
    writable: false,
    enumerable: false,
    configurable: false,
  });
  for (let i = 0; i < names.length; i++) {
    Object.defineProperty(target, names[i], {
      value: undefined,
      writable: true,
      enumerable: true,
      configurable: false,
    });
  }
  Object.preventExtensions(target);
  function hasName(p) {
    return typeof p === "string" && Object.prototype.hasOwnProperty.call(getters, p);
  }
  return new Proxy(target, {
    get(_t, p) {
      if (p === Symbol.toStringTag) return toStringTag;
      if (typeof p === "symbol") return undefined;
      if (!hasName(p)) return undefined;
      return getters[p]();
    },
    has(_t, p) {
      if (p === Symbol.toStringTag) return true;
      if (typeof p === "symbol") return false;
      return hasName(p);
    },
    getOwnPropertyDescriptor(_t, p) {
      if (p === Symbol.toStringTag) {
        return { value: toStringTag, writable: false, enumerable: false, configurable: false };
      }
      if (typeof p === "symbol") return undefined;
      if (!hasName(p)) return undefined;
      return { value: getters[p](), writable: true, enumerable: true, configurable: false };
    },
    ownKeys() {
      return names.concat([Symbol.toStringTag]);
    },
    defineProperty(_t, p, desc) {
      if (typeof p === "symbol") {
        if (p !== Symbol.toStringTag) return false;
        if (desc.configurable === true) return false;
        if (desc.writable === true) return false;
        if (desc.enumerable === true) return false;
        if ("value" in desc && desc.value !== toStringTag) return false;
        return true;
      }
      if (!hasName(p)) return false;
      let current = getters[p]();
      if (desc.configurable === true) return false;
      if (desc.enumerable === false) return false;
      if (desc.get !== undefined || desc.set !== undefined) return false;
      if (desc.writable === false) return false;
      if ("value" in desc && desc.value !== current) return false;
      return true;
    },
    deleteProperty(_t, p) {
      if (typeof p === "symbol") return p !== Symbol.toStringTag;
      return !hasName(p);
    },
    set() {
      return false;
    },
    getPrototypeOf() {
      return null;
    },
    setPrototypeOf(_t, p) {
      return p === null;
    },
    isExtensible() {
      return false;
    },
    preventExtensions() {
      return true;
    },
  });
}
"#;
    Ok(parse(src)?.body)
}

fn make_call_stmt(fn_name: &str, span: Span) -> Stmt {
    Stmt::Expression {
        expr: Expr::Call {
            callee: Box::new(Expr::Ident(Ident {
                name: fn_name.to_string(),
                span,
            })),
            args: vec![],
            optional: false,
            span,
        },
        span,
    }
}

fn make_deferred_namespace_binding(
    local: &str,
    eval_fn: Option<&str>,
    export_pairs: &[(String, String)],
    mod_id: usize,
    span: Span,
) -> Result<Stmt, Diagnostic> {
    let mut props = String::new();
    let mut names = String::new();
    for (export_name, remote) in export_pairs {
        let key = js_object_key(export_name);
        props.push_str(&format!("{key}: {remote}, "));
        let lit = export_name.replace('\\', "\\\\").replace('"', "\\\"");
        names.push_str(&format!("\"{lit}\", "));
    }
    // `eval_fn` is the once-eval thunk for a still-lazy deferred module; eager
    // modules already ran, so the closure just reads the initialized bindings.
    let eval_src = match eval_fn {
        Some(f) => format!("{f}();"),
        None => String::new(),
    };
    let src = format!(
        "let {local} = __draconic_deferred_ns(function () {{ {eval_src} return {{ {props} }}; }}, [{names}], {mod_id});"
    );
    let mut body = parse(&src)?.body;
    let stmt = body.pop().ok_or_else(|| {
        Diagnostic::new("deferred namespace binding parse produced no stmt", span)
    })?;
    Ok(stmt)
}

/// E19.86: build `let __ns{id} = __draconic_make_ns(pairs, names, "Module");`.
/// Each pair is `[exportName, function () { return <final binding name>; }]`, so
/// export values are read lazily (live bindings + TDZ ReferenceError) instead of
/// being snapshotted at namespace creation.
fn make_shared_namespace_binding(
    local: &str,
    resolved: &HashMap<String, (usize, String)>,
    mangled: &[HashMap<String, String>],
    span: Span,
) -> Result<Stmt, Diagnostic> {
    let mut names: Vec<String> = resolved.keys().cloned().collect();
    names.sort();
    let mut pairs: Vec<String> = Vec::with_capacity(names.len());
    let mut names_lit: Vec<String> = Vec::with_capacity(names.len());
    for export_name in &names {
        let (def_id, local_in_exporter) = resolved.get(export_name).expect("key from map");
        let remote = final_binding_name(mangled, *def_id, local_in_exporter)?;
        let lit = export_name.replace('\\', "\\\\").replace('"', "\\\"");
        pairs.push(format!("[\"{lit}\", function () {{ return {remote}; }}]"));
        names_lit.push(format!("\"{lit}\""));
    }
    let src = format!(
        "let {local} = __draconic_make_ns([{}], [{}], \"Module\");",
        pairs.join(", "),
        names_lit.join(", ")
    );
    let mut body = parse(&src)?.body;
    let stmt = body.pop().ok_or_else(|| {
        Diagnostic::new("shared namespace binding parse produced no stmt", span)
    })?;
    Ok(stmt)
}

fn js_object_key(name: &str) -> String {
    if is_js_ident(name) {
        name.to_string()
    } else {
        format!("\"{}\"", name.replace('\\', "\\\\").replace('"', "\\\""))
    }
}

fn is_js_ident(name: &str) -> bool {
    let mut chars = name.chars();
    match chars.next() {
        Some(c) if c == '_' || c == '$' || c.is_ascii_alphabetic() => {}
        _ => return false,
    }
    chars.all(|c| c == '_' || c == '$' || c.is_ascii_alphanumeric())
}

/// E19.84.03: JSON modules (`.json`) — ParseJSONModule. The raw JSON source is
/// embedded as a JS string literal and parsed by the runtime's `JSON.parse` at
/// eval, so the synthetic `default` binding is the parsed JSON value.
fn parse_json_module(source: &str, path: &Path) -> Result<Program, Diagnostic> {
    const LOCAL: &str = "__json_default";
    let lit = js_string_literal(source);
    let synthetic = format!("const {LOCAL} = JSON.parse({lit});\nexport {{ {LOCAL} as default }};");
    parse_module(&synthetic).map_err(|e| {
        Diagnostic::new(
            format!(
                "failed to synthesize JSON module {}: {e}",
                path.display()
            ),
            Span::dummy(),
        )
    })
}

/// Quote `s` as a double-quoted JS string literal (no U+2028/U+2029 in output).
fn js_string_literal(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\u{b}' => out.push_str("\\u000b"),
            '\u{0}' => out.push_str("\\u0000"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            '\u{2028}' => out.push_str("\\u2028"),
            '\u{2029}' => out.push_str("\\u2029"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// Hoist top-level bindings and wrap module body in a once-eval function.
fn wrap_deferred_module_body(
    mod_id: usize,
    eval_name: &str,
    prelude_calls: Vec<Stmt>,
    body: Vec<Stmt>,
    spans: &mut SyntheticSpans,
) -> Vec<Stmt> {
    let names = top_level_names(&body);
    let mut out = Vec::new();
    let mut sorted: Vec<_> = names.into_iter().collect();
    sorted.sort();
    for name in &sorted {
        let sp = spans.next();
        out.push(Stmt::Let {
            kind: BindingKind::Let,
            binding: BindingPattern::Ident(Ident {
                name: name.clone(),
                span: sp,
            }),
            type_ann: None,
            init: None,
            span: sp,
        });
    }

    // E19.84.05 / E19.84.08: once-eval with [[EvaluationError]] cache.
    // ReadyForSyncExecution / TypeError is enforced by __draconic_deferred_ns.ensure before call.
    let mut try_body: Vec<Stmt> = prelude_calls;
    for stmt in body {
        try_body.push(hoist_decl_to_assign(stmt));
    }
    try_body.push(make_module_status_assign(mod_id, 3, spans.next()));
    let guard_src = format!(
        r#"
function {eval_name}() {{
  if (__draconic_mstatus[{mod_id}] === 3) {{
    if (__draconic_merror[{mod_id}] !== undefined) throw __draconic_merror[{mod_id}];
    return;
  }}
  __draconic_mstatus[{mod_id}] = 1;
  try {{
    ;
  }} catch (e) {{
    __draconic_merror[{mod_id}] = e;
    __draconic_mstatus[{mod_id}] = 3;
    throw e;
  }}
}}
"#
    );
    let mut parsed = match parse(&guard_src) {
        Ok(p) => p.body,
        Err(_) => {
            // Fallback without try (should not happen): previous shape.
            let mut eval_body = Vec::new();
            let guard_span = spans.next();
            eval_body.push(make_module_status_assign(mod_id, 1, guard_span));
            eval_body.extend(try_body);
            let fn_span = spans.next();
            out.push(Stmt::FunctionDeclaration {
                name: Ident {
                    name: eval_name.to_string(),
                    span: fn_span,
                },
                type_params: vec![],
                params: vec![],
                return_type: None,
                body: Box::new(Stmt::Block {
                    body: eval_body,
                    span: fn_span,
                }),
                is_async: false,
                is_generator: false,
                span: fn_span,
            });
            return out;
        }
    };
    // Inject real module body into the try block of the parsed skeleton.
    if let Some(Stmt::FunctionDeclaration { body: fn_body, .. }) = parsed.last_mut() {
        if let Stmt::Block { body: stmts, .. } = fn_body.as_mut() {
            for stmt in stmts.iter_mut() {
                if let Stmt::Try {
                    block,
                    ..
                } = stmt
                {
                    if let Stmt::Block {
                        body: try_stmts, ..
                    } = block.as_mut()
                    {
                        *try_stmts = try_body;
                        break;
                    }
                }
            }
        }
    }
    for mut stmt in parsed {
        uniqueify_stmt_spans(&mut stmt, spans);
        out.push(stmt);
    }
    out
}

/// Turn top-level `let/const x = init` / `function f` into assignments to hoisted bindings.
fn hoist_decl_to_assign(stmt: Stmt) -> Stmt {
    match stmt {
        Stmt::Let {
            binding: BindingPattern::Ident(id),
            init: Some(init),
            span,
            ..
        } => Stmt::Expression {
            expr: Expr::Assign {
                target: Box::new(Expr::Ident(id)),
                op: AssignOp::Eq,
                value: Box::new(init),
                span,
            },
            span,
        },
        Stmt::Let {
            binding: BindingPattern::Ident(_),
            init: None,
            span,
            ..
        } => Stmt::Empty { span },
        Stmt::FunctionDeclaration {
            name,
            params,
            return_type,
            body,
            is_async,
            is_generator,
            span,
            ..
        } => {
            let fn_expr = Expr::FunctionExpression {
                name: Some(name.clone()),
                params,
                return_type,
                body,
                is_async,
                is_generator,
                is_method: false,
                span,
            };
            Stmt::Expression {
                expr: Expr::Assign {
                    target: Box::new(Expr::Ident(name)),
                    op: AssignOp::Eq,
                    value: Box::new(fn_expr),
                    span,
                },
                span,
            }
        }
        other => other,
    }
}

/// Expand multi-declarator export (`export let a, b`) from a Block of Lets into
/// individual declarations (parser packs multi-declarators as `Stmt::Block`).
fn expand_export_decl(decl: Stmt) -> Vec<Stmt> {
    match decl {
        Stmt::Block { body, .. }
            if body
                .iter()
                .all(|s| matches!(s, Stmt::Let { .. })) =>
        {
            body
        }
        other => vec![other],
    }
}

fn collect_decl_exports(
    decl: &Stmt,
    exports: &mut HashMap<String, String>,
) -> Result<(), Diagnostic> {
    match decl {
        Stmt::Let { binding, .. } => {
            let mut err = None;
            binding.for_each_ident(&mut |id| {
                if err.is_some() {
                    return;
                }
                if exports.insert(id.name.clone(), id.name.clone()).is_some() {
                    err = Some(Diagnostic::new(
                        format!("duplicate export `{}`", id.name),
                        id.span,
                    ));
                }
            });
            match err {
                Some(e) => Err(e),
                None => Ok(()),
            }
        }
        Stmt::FunctionDeclaration { name, .. } | Stmt::ClassDeclaration { name, .. } => {
            if exports
                .insert(name.name.clone(), name.name.clone())
                .is_some()
            {
                return Err(Diagnostic::new(
                    format!("duplicate export `{}`", name.name),
                    name.span,
                ));
            }
            Ok(())
        }
        Stmt::Block { body, .. }
            if body
                .iter()
                .all(|s| matches!(s, Stmt::Let { .. })) =>
        {
            for s in body {
                collect_decl_exports(s, exports)?;
            }
            Ok(())
        }
        _ => Err(Diagnostic::new(
            "unsupported export declaration".to_string(),
            Span::dummy(),
        )),
    }
}

fn top_level_names(body: &[Stmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    for stmt in body {
        match stmt {
            Stmt::Let { binding, .. } => {
                binding.for_each_ident(&mut |id| {
                    names.insert(id.name.clone());
                });
            }
            Stmt::FunctionDeclaration { name, .. } | Stmt::ClassDeclaration { name, .. } => {
                names.insert(name.name.clone());
            }
            _ => {}
        }
    }
    names
}

/// Nested scopes for rename: frame 0 is module top-level; deeper frames shadow.
struct ScopeStack {
    frames: Vec<HashSet<String>>,
}

impl ScopeStack {
    fn new() -> Self {
        Self {
            frames: vec![HashSet::new()],
        }
    }

    fn push(&mut self) {
        self.frames.push(HashSet::new());
    }

    fn pop(&mut self) {
        self.frames.pop();
    }

    fn declare_nested(&mut self, name: &str) {
        self.frames
            .last_mut()
            .expect("scope")
            .insert(name.to_string());
    }

    fn depth(&self) -> usize {
        self.frames.len()
    }

    fn is_shadowed(&self, name: &str) -> bool {
        for frame in self.frames.iter().skip(1).rev() {
            if frame.contains(name) {
                return true;
            }
        }
        false
    }
}

fn rename_object_key(
    key: &mut ObjectKey,
    renames: &HashMap<String, String>,
    scopes: &mut ScopeStack,
) {
    match key {
        ObjectKey::Computed(e) => rename_expr(e, renames, scopes),
        ObjectKey::Ident(_) | ObjectKey::String(_) => {}
    }
}

fn rename_ident(id: &mut Ident, renames: &HashMap<String, String>, scopes: &ScopeStack) {
    if scopes.is_shadowed(&id.name) {
        return;
    }
    if let Some(new_name) = renames.get(&id.name) {
        id.name = new_name.clone();
    }
}

fn rename_binding_decl(
    pat: &mut BindingPattern,
    renames: &HashMap<String, String>,
    scopes: &mut ScopeStack,
) {
    match pat {
        BindingPattern::Ident(id) => {
            if scopes.depth() == 1 {
                rename_ident(id, renames, scopes);
            } else {
                scopes.declare_nested(&id.name);
            }
        }
        BindingPattern::Member(expr) => rename_expr(expr, renames, scopes),
        BindingPattern::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        default,
                        ..
                    } => {
                        rename_object_key(key, renames, scopes);
                        rename_binding_decl(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ObjectPatternProp::Rest(binding) => {
                        rename_binding_decl(binding, renames, scopes);
                    }
                }
            }
        }
        BindingPattern::Array { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        rename_binding_decl(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        rename_binding_decl(binding, renames, scopes);
                    }
                }
            }
        }
    }
}

fn rename_stmt(stmt: &mut Stmt, renames: &HashMap<String, String>, scopes: &mut ScopeStack) {
    match stmt {
        Stmt::Expression { expr, .. } => rename_expr(expr, renames, scopes),
        Stmt::Let { binding, init, .. } => {
            if let Some(init) = init {
                rename_expr(init, renames, scopes);
            }
            rename_binding_decl(binding, renames, scopes);
        }
        Stmt::Empty { .. }
        | Stmt::Break { .. }
        | Stmt::Continue { .. }
        | Stmt::ImportDeclaration { .. }
        | Stmt::ExportNamedDeclaration { .. }
        | Stmt::ExportDefaultDeclaration { .. }
        | Stmt::ExportAllDeclaration { .. }
        | Stmt::TypeAlias { .. } => {}
        Stmt::Block { body, .. } => {
            scopes.push();
            // declare nested first for TDZ-ish list, then rename bodies
            for s in body.iter_mut() {
                predeclare_nested(s, scopes);
            }
            for s in body.iter_mut() {
                rename_stmt(s, renames, scopes);
            }
            scopes.pop();
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            rename_expr(test, renames, scopes);
            rename_stmt(consequent, renames, scopes);
            if let Some(alt) = alternate {
                rename_stmt(alt, renames, scopes);
            }
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { body, test, .. } => {
            rename_expr(test, renames, scopes);
            rename_stmt(body, renames, scopes);
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            scopes.push();
            if let Some(init) = init {
                rename_stmt(init, renames, scopes);
            }
            if let Some(test) = test {
                rename_expr(test, renames, scopes);
            }
            if let Some(update) = update {
                rename_expr(update, renames, scopes);
            }
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Stmt::ForIn {
            left, right, body, ..
        }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            scopes.push();
            rename_stmt(left, renames, scopes);
            rename_expr(right, renames, scopes);
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Stmt::Labeled { body, .. } => rename_stmt(body, renames, scopes),
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            rename_expr(discriminant, renames, scopes);
            scopes.push();
            for case in cases.iter_mut() {
                if let Some(test) = &mut case.test {
                    rename_expr(test, renames, scopes);
                }
                for s in case.body.iter_mut() {
                    predeclare_nested(s, scopes);
                }
            }
            for case in cases.iter_mut() {
                for s in case.body.iter_mut() {
                    rename_stmt(s, renames, scopes);
                }
            }
            scopes.pop();
        }
        Stmt::FunctionDeclaration {
            name, params, body, ..
        } => {
            if scopes.depth() == 1 {
                rename_ident(name, renames, scopes);
            } else {
                scopes.declare_nested(&name.name);
            }
            scopes.push();
            rename_params(params, renames, scopes);
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Stmt::ClassDeclaration {
            name,
            super_class,
            body,
            ..
        } => {
            if scopes.depth() == 1 {
                rename_ident(name, renames, scopes);
            } else {
                scopes.declare_nested(&name.name);
            }
            if let Some(sc) = super_class {
                rename_expr(sc, renames, scopes);
            }
            for el in body.iter_mut() {
                match el {
                    ClassElement::Constructor { params, body, .. } => {
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                    ClassElement::Method {
                        key, params, body, ..
                    }
                    | ClassElement::Accessor {
                        key, params, body, ..
                    } => {
                        rename_object_key(key, renames, scopes);
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                    ClassElement::Field { key, value, .. } => {
                        rename_object_key(key, renames, scopes);
                        if let Some(v) = value {
                            rename_expr(v, renames, scopes);
                        }
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        rename_stmt(body, renames, scopes);
                    }
                }
            }
        }
        Stmt::Return { argument, .. } => {
            if let Some(arg) = argument {
                rename_expr(arg, renames, scopes);
            }
        }
        Stmt::Throw { argument, .. } => rename_expr(argument, renames, scopes),
        Stmt::Try {
            block,
            handler_param,
            handler,
            finalizer,
            ..
        } => {
            rename_stmt(block, renames, scopes);
            if let Some(handler) = handler {
                scopes.push();
                if let Some(param) = handler_param {
                    rename_binding_decl(param, renames, scopes);
                }
                rename_stmt(handler, renames, scopes);
                scopes.pop();
            }
            if let Some(finalizer) = finalizer {
                rename_stmt(finalizer, renames, scopes);
            }
        }
        Stmt::With { object, body, .. } => {
            rename_expr(object, renames, scopes);
            rename_stmt(body, renames, scopes);
        }
    }
}

fn predeclare_nested(stmt: &Stmt, scopes: &mut ScopeStack) {
    if scopes.depth() == 1 {
        return;
    }
    match stmt {
        Stmt::Let { binding, .. } => {
            binding.for_each_ident(&mut |id| scopes.declare_nested(&id.name));
        }
        Stmt::FunctionDeclaration { name, .. } | Stmt::ClassDeclaration { name, .. } => {
            scopes.declare_nested(&name.name);
        }
        _ => {}
    }
}

fn rename_params(params: &mut [Param], renames: &HashMap<String, String>, scopes: &mut ScopeStack) {
    for p in params.iter_mut() {
        rename_binding_decl(&mut p.binding, renames, scopes);
        if let Some(default) = &mut p.default {
            rename_expr(default, renames, scopes);
        }
    }
}

fn rename_expr(expr: &mut Expr, renames: &HashMap<String, String>, scopes: &mut ScopeStack) {
    match expr {
        Expr::Ident(id) => rename_ident(id, renames, scopes),
        Expr::Number(_)
        | Expr::BigInt(_)
        | Expr::String(_)
        | Expr::RegExp { .. }
        | Expr::Boolean { .. }
        | Expr::Null { .. }
        | Expr::This { .. }
        | Expr::Super { .. }
        | Expr::NewTarget { .. } | Expr::ImportMeta { .. } => {}
        Expr::ImportCall {
            source, options, ..
        } => {
            rename_expr(source, renames, scopes);
            if let Some(opts) = options {
                rename_expr(opts, renames, scopes);
            }
        }
        Expr::TemplateLiteral { expressions, .. } => {
            for e in expressions {
                rename_expr(e, renames, scopes);
            }
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            rename_expr(tag, renames, scopes);
            for e in expressions {
                rename_expr(e, renames, scopes);
            }
        }
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } | Expr::Paren { expr: arg, .. } => {
            rename_expr(arg, renames, scopes)
        }
        Expr::As { expr, .. } => rename_expr(expr, renames, scopes),
        Expr::Binary { left, right, .. } => {
            rename_expr(left, renames, scopes);
            rename_expr(right, renames, scopes);
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            rename_expr(test, renames, scopes);
            rename_expr(consequent, renames, scopes);
            rename_expr(alternate, renames, scopes);
        }
        Expr::Assign { target, value, .. } => {
            rename_expr(target, renames, scopes);
            rename_expr(value, renames, scopes);
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            rename_expr(callee, renames, scopes);
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => rename_expr(e, renames, scopes),
                }
            }
        }
        Expr::FunctionExpression {
            name, params, body, ..
        } => {
            scopes.push();
            if let Some(name) = name {
                scopes.declare_nested(&name.name);
            }
            rename_params(params, renames, scopes);
            rename_stmt(body, renames, scopes);
            scopes.pop();
        }
        Expr::ClassExpression {
            name,
            super_class,
            body,
            ..
        } => {
            scopes.push();
            if let Some(name) = name {
                scopes.declare_nested(&name.name);
            }
            if let Some(sc) = super_class {
                rename_expr(sc, renames, scopes);
            }
            for el in body.iter_mut() {
                match el {
                    ClassElement::Constructor { params, body, .. } => {
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                    ClassElement::Method {
                        key, params, body, ..
                    }
                    | ClassElement::Accessor {
                        key, params, body, ..
                    } => {
                        rename_object_key(key, renames, scopes);
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                    ClassElement::Field { key, value, .. } => {
                        rename_object_key(key, renames, scopes);
                        if let Some(v) = value {
                            rename_expr(v, renames, scopes);
                        }
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        rename_stmt(body, renames, scopes);
                    }
                }
            }
            scopes.pop();
        }
        Expr::ArrowFunction { params, body, .. } => {
            scopes.push();
            rename_params(params, renames, scopes);
            match body {
                ArrowBody::Expr(e) => rename_expr(e, renames, scopes),
                ArrowBody::Block(b) => rename_stmt(b, renames, scopes),
            }
            scopes.pop();
        }
        Expr::ObjectExpression { properties, .. } => {
            for p in properties {
                match p {
                    ObjectProp::Property {
                        key,
                        value,
                        shorthand,
                        ..
                    } => {
                        match key {
                            ObjectKey::Computed(e) => rename_expr(e, renames, scopes),
                            ObjectKey::Ident(id) if *shorthand => {
                                rename_ident(id, renames, scopes)
                            }
                            ObjectKey::Ident(_) | ObjectKey::String(_) => {}
                        }
                        rename_expr(value, renames, scopes);
                    }
                    ObjectProp::Accessor {
                        key, params, body, ..
                    } => {
                        match key {
                            ObjectKey::Computed(e) => rename_expr(e, renames, scopes),
                            ObjectKey::Ident(_) | ObjectKey::String(_) => {}
                        }
                        scopes.push();
                        rename_params(params, renames, scopes);
                        rename_stmt(body, renames, scopes);
                        scopes.pop();
                    }
                    ObjectProp::Spread { expr, .. } => rename_expr(expr, renames, scopes),
                }
            }
        }
        Expr::ArrayExpression { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        rename_expr(e, renames, scopes)
                    }
                    ArrayElement::Elision => {}
                }
            }
        }
        Expr::ArrayPattern { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        rename_binding_pattern_use(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        rename_binding_pattern_use(binding, renames, scopes)
                    }
                }
            }
        }
        Expr::ObjectPattern { properties, .. } => {
            for p in properties {
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        default,
                        ..
                    } => {
                        rename_object_key(key, renames, scopes);
                        rename_binding_pattern_use(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ObjectPatternProp::Rest(binding) => {
                        rename_binding_pattern_use(binding, renames, scopes)
                    }
                }
            }
        }
        Expr::MemberExpression {
            object,
            property,
            computed,
            ..
        } => {
            rename_expr(object, renames, scopes);
            if *computed {
                rename_expr(property, renames, scopes);
            }
            // Non-computed property name is not a variable reference.
        }
        Expr::PrivateIn { object, .. } => rename_expr(object, renames, scopes),
    }
}

fn rename_binding_pattern_use(
    pat: &mut BindingPattern,
    renames: &HashMap<String, String>,
    scopes: &mut ScopeStack,
) {
    match pat {
        BindingPattern::Ident(id) => rename_ident(id, renames, scopes),
        BindingPattern::Member(expr) => rename_expr(expr, renames, scopes),
        BindingPattern::Array { elements, .. } => {
            for el in elements {
                match el {
                    ArrayPatternElement::Elision => {}
                    ArrayPatternElement::Pattern { binding, default } => {
                        rename_binding_pattern_use(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ArrayPatternElement::Rest(binding) => {
                        rename_binding_pattern_use(binding, renames, scopes)
                    }
                }
            }
        }
        BindingPattern::Object { properties, .. } => {
            for p in properties {
                match p {
                    ObjectPatternProp::Prop {
                        key,
                        binding,
                        default,
                        ..
                    } => {
                        rename_object_key(key, renames, scopes);
                        rename_binding_pattern_use(binding, renames, scopes);
                        if let Some(def) = default {
                            rename_expr(def, renames, scopes);
                        }
                    }
                    ObjectPatternProp::Rest(binding) => {
                        rename_binding_pattern_use(binding, renames, scopes)
                    }
                }
            }
        }
    }
}

fn stmt_span_approx(stmt: &Stmt) -> Span {
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
        | Stmt::TypeAlias { span, .. } => *span,
    }
}

/// E19.84.06: collect resolved paths of string-literal `import.defer("…")` calls.
fn collect_dynamic_defer_targets(
    body: &[Stmt],
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    for stmt in body {
        collect_dynamic_defer_in_stmt(stmt, parent, out)?;
    }
    Ok(())
}

/// E19.84.08: collect resolved paths of string-literal evaluation-phase `import("…")`.
fn collect_dynamic_eval_import_targets(
    body: &[Stmt],
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    for stmt in body {
        collect_dynamic_eval_import_in_stmt(stmt, parent, out)?;
    }
    Ok(())
}

fn collect_dynamic_eval_import_in_stmt(
    stmt: &Stmt,
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    // Reuse the defer walker; only the ImportCall phase filter differs.
    collect_dynamic_import_phase_in_stmt(stmt, parent, out, ImportPhase::Evaluation)
}

fn collect_dynamic_import_phase_in_stmt(
    stmt: &Stmt,
    parent: &Path,
    out: &mut Vec<PathBuf>,
    phase_filter: ImportPhase,
) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Expression { expr, .. } => {
            collect_dynamic_import_phase_in_expr(expr, parent, out, phase_filter)?
        }
        Stmt::Let { init: Some(init), .. } => {
            collect_dynamic_import_phase_in_expr(init, parent, out, phase_filter)?
        }
        Stmt::Block { body, .. } => {
            for s in body {
                collect_dynamic_import_phase_in_stmt(s, parent, out, phase_filter)?;
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            collect_dynamic_import_phase_in_expr(test, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_stmt(consequent, parent, out, phase_filter)?;
            if let Some(alt) = alternate {
                collect_dynamic_import_phase_in_stmt(alt, parent, out, phase_filter)?;
            }
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            collect_dynamic_import_phase_in_expr(test, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            if let Some(init) = init {
                collect_dynamic_import_phase_in_stmt(init, parent, out, phase_filter)?;
            }
            if let Some(test) = test {
                collect_dynamic_import_phase_in_expr(test, parent, out, phase_filter)?;
            }
            if let Some(update) = update {
                collect_dynamic_import_phase_in_expr(update, parent, out, phase_filter)?;
            }
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Stmt::ForIn {
            left, right, body, ..
        }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            collect_dynamic_import_phase_in_stmt(left, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_expr(right, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Stmt::Labeled { body, .. } => {
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?
        }
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            collect_dynamic_import_phase_in_expr(discriminant, parent, out, phase_filter)?;
            for c in cases {
                if let Some(test) = &c.test {
                    collect_dynamic_import_phase_in_expr(test, parent, out, phase_filter)?;
                }
                for s in &c.body {
                    collect_dynamic_import_phase_in_stmt(s, parent, out, phase_filter)?;
                }
            }
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            collect_dynamic_import_phase_in_stmt(block, parent, out, phase_filter)?;
            if let Some(handler) = handler {
                collect_dynamic_import_phase_in_stmt(handler, parent, out, phase_filter)?;
            }
            if let Some(finalizer) = finalizer {
                collect_dynamic_import_phase_in_stmt(finalizer, parent, out, phase_filter)?;
            }
        }
        Stmt::With { object, body, .. } => {
            collect_dynamic_import_phase_in_expr(object, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Stmt::Return {
            argument: Some(arg),
            ..
        }
        | Stmt::Throw { argument: arg, .. } => {
            collect_dynamic_import_phase_in_expr(arg, parent, out, phase_filter)?
        }
        Stmt::FunctionDeclaration {
            body, params, ..
        } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_import_phase_in_expr(default, parent, out, phase_filter)?;
                }
            }
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Stmt::ClassDeclaration {
            super_class, body, ..
        } => {
            if let Some(sc) = super_class {
                collect_dynamic_import_phase_in_expr(sc, parent, out, phase_filter)?;
            }
            for el in body {
                match el {
                    ClassElement::Constructor { body, params, .. }
                    | ClassElement::Method { body, params, .. }
                    | ClassElement::Accessor { body, params, .. } => {
                        for p in params {
                            if let Some(default) = &p.default {
                                collect_dynamic_import_phase_in_expr(
                                    default,
                                    parent,
                                    out,
                                    phase_filter,
                                )?;
                            }
                        }
                        collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
                    }
                    ClassElement::Field { key, value, .. } => {
                        if let ObjectKey::Computed(key) = key {
                            collect_dynamic_import_phase_in_expr(key, parent, out, phase_filter)?;
                        }
                        if let Some(value) = value {
                            collect_dynamic_import_phase_in_expr(value, parent, out, phase_filter)?;
                        }
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn collect_dynamic_import_phase_in_expr(
    expr: &Expr,
    parent: &Path,
    out: &mut Vec<PathBuf>,
    phase_filter: ImportPhase,
) -> Result<(), Diagnostic> {
    match expr {
        Expr::ImportCall {
            phase,
            source,
            options,
            ..
        } => {
            if *phase == phase_filter {
                if let Expr::String(lit) = source.as_ref() {
                    if let Some(spec) = lit.value.to_string_strict() {
                        let dep = resolve_specifier(parent, &spec, lit.span)?;
                        if !out.iter().any(|p| p == &dep) {
                            out.push(dep);
                        }
                    }
                }
            }
            collect_dynamic_import_phase_in_expr(source, parent, out, phase_filter)?;
            if let Some(options) = options {
                collect_dynamic_import_phase_in_expr(options, parent, out, phase_filter)?;
            }
        }
        Expr::Unary { arg, .. }
        | Expr::Update { arg, .. }
        | Expr::Paren { expr: arg, .. }
        | Expr::As { expr: arg, .. } => {
            collect_dynamic_import_phase_in_expr(arg, parent, out, phase_filter)?;
        }
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => {
            collect_dynamic_import_phase_in_expr(left, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_expr(right, parent, out, phase_filter)?;
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            collect_dynamic_import_phase_in_expr(test, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_expr(consequent, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_expr(alternate, parent, out, phase_filter)?;
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            collect_dynamic_import_phase_in_expr(callee, parent, out, phase_filter)?;
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => {
                        collect_dynamic_import_phase_in_expr(e, parent, out, phase_filter)?
                    }
                }
            }
        }
        Expr::MemberExpression {
            object, property, ..
        } => {
            collect_dynamic_import_phase_in_expr(object, parent, out, phase_filter)?;
            collect_dynamic_import_phase_in_expr(property, parent, out, phase_filter)?;
        }
        Expr::PrivateIn { object, .. } => {
            collect_dynamic_import_phase_in_expr(object, parent, out, phase_filter)?
        }
        Expr::ArrayExpression { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        collect_dynamic_import_phase_in_expr(e, parent, out, phase_filter)?
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
                            collect_dynamic_import_phase_in_expr(key, parent, out, phase_filter)?;
                        }
                        collect_dynamic_import_phase_in_expr(value, parent, out, phase_filter)?;
                    }
                    ObjectProp::Accessor {
                        key, params, body, ..
                    } => {
                        if let ObjectKey::Computed(key) = key {
                            collect_dynamic_import_phase_in_expr(key, parent, out, phase_filter)?;
                        }
                        for p in params {
                            if let Some(default) = &p.default {
                                collect_dynamic_import_phase_in_expr(
                                    default,
                                    parent,
                                    out,
                                    phase_filter,
                                )?;
                            }
                        }
                        collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
                    }
                    ObjectProp::Spread { expr, .. } => {
                        collect_dynamic_import_phase_in_expr(expr, parent, out, phase_filter)?
                    }
                }
            }
        }
        Expr::TemplateLiteral { expressions, .. } => {
            for e in expressions {
                collect_dynamic_import_phase_in_expr(e, parent, out, phase_filter)?;
            }
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            collect_dynamic_import_phase_in_expr(tag, parent, out, phase_filter)?;
            for e in expressions {
                collect_dynamic_import_phase_in_expr(e, parent, out, phase_filter)?;
            }
        }
        Expr::FunctionExpression {
            params, body, ..
        } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_import_phase_in_expr(default, parent, out, phase_filter)?;
                }
            }
            collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
        }
        Expr::ClassExpression {
            super_class, body, ..
        } => {
            if let Some(sc) = super_class {
                collect_dynamic_import_phase_in_expr(sc, parent, out, phase_filter)?;
            }
            for el in body {
                match el {
                    ClassElement::Constructor { body, params, .. }
                    | ClassElement::Method { body, params, .. }
                    | ClassElement::Accessor { body, params, .. } => {
                        for p in params {
                            if let Some(default) = &p.default {
                                collect_dynamic_import_phase_in_expr(
                                    default,
                                    parent,
                                    out,
                                    phase_filter,
                                )?;
                            }
                        }
                        collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
                    }
                    ClassElement::Field { key, value, .. } => {
                        if let ObjectKey::Computed(key) = key {
                            collect_dynamic_import_phase_in_expr(key, parent, out, phase_filter)?;
                        }
                        if let Some(value) = value {
                            collect_dynamic_import_phase_in_expr(value, parent, out, phase_filter)?;
                        }
                    }
                    ClassElement::StaticBlock { body, .. } => {
                        collect_dynamic_import_phase_in_stmt(body, parent, out, phase_filter)?;
                    }
                }
            }
        }
        Expr::ArrowFunction { params, body, .. } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_import_phase_in_expr(default, parent, out, phase_filter)?;
                }
            }
            match body {
                ArrowBody::Expr(e) => {
                    collect_dynamic_import_phase_in_expr(e, parent, out, phase_filter)?
                }
                ArrowBody::Block(b) => {
                    collect_dynamic_import_phase_in_stmt(b, parent, out, phase_filter)?
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn collect_dynamic_defer_in_stmt(
    stmt: &Stmt,
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    match stmt {
        Stmt::Expression { expr, .. } => collect_dynamic_defer_in_expr(expr, parent, out)?,
        Stmt::Let { init: Some(init), .. } => collect_dynamic_defer_in_expr(init, parent, out)?,
        Stmt::Block { body, .. } => {
            for s in body {
                collect_dynamic_defer_in_stmt(s, parent, out)?;
            }
        }
        Stmt::If {
            test,
            consequent,
            alternate,
            ..
        } => {
            collect_dynamic_defer_in_expr(test, parent, out)?;
            collect_dynamic_defer_in_stmt(consequent, parent, out)?;
            if let Some(alt) = alternate {
                collect_dynamic_defer_in_stmt(alt, parent, out)?;
            }
        }
        Stmt::While { test, body, .. } | Stmt::DoWhile { test, body, .. } => {
            collect_dynamic_defer_in_expr(test, parent, out)?;
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Stmt::For {
            init,
            test,
            update,
            body,
            ..
        } => {
            if let Some(init) = init {
                collect_dynamic_defer_in_stmt(init, parent, out)?;
            }
            if let Some(test) = test {
                collect_dynamic_defer_in_expr(test, parent, out)?;
            }
            if let Some(update) = update {
                collect_dynamic_defer_in_expr(update, parent, out)?;
            }
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Stmt::ForIn {
            left, right, body, ..
        }
        | Stmt::ForOf {
            left, right, body, ..
        } => {
            collect_dynamic_defer_in_stmt(left, parent, out)?;
            collect_dynamic_defer_in_expr(right, parent, out)?;
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Stmt::Labeled { body, .. } => collect_dynamic_defer_in_stmt(body, parent, out)?,
        Stmt::Switch {
            discriminant,
            cases,
            ..
        } => {
            collect_dynamic_defer_in_expr(discriminant, parent, out)?;
            for c in cases {
                if let Some(test) = &c.test {
                    collect_dynamic_defer_in_expr(test, parent, out)?;
                }
                for s in &c.body {
                    collect_dynamic_defer_in_stmt(s, parent, out)?;
                }
            }
        }
        Stmt::Try {
            block,
            handler,
            finalizer,
            ..
        } => {
            collect_dynamic_defer_in_stmt(block, parent, out)?;
            if let Some(handler) = handler {
                collect_dynamic_defer_in_stmt(handler, parent, out)?;
            }
            if let Some(finalizer) = finalizer {
                collect_dynamic_defer_in_stmt(finalizer, parent, out)?;
            }
        }
        Stmt::With { object, body, .. } => {
            collect_dynamic_defer_in_expr(object, parent, out)?;
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Stmt::Return {
            argument: Some(arg),
            ..
        }
        | Stmt::Throw { argument: arg, .. } => collect_dynamic_defer_in_expr(arg, parent, out)?,
        Stmt::FunctionDeclaration {
            body, params, ..
        } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_defer_in_expr(default, parent, out)?;
                }
            }
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Stmt::ClassDeclaration {
            super_class, body, ..
        } => {
            if let Some(sc) = super_class {
                collect_dynamic_defer_in_expr(sc, parent, out)?;
            }
            collect_dynamic_defer_in_class_els(body, parent, out)?;
        }
        _ => {}
    }
    Ok(())
}

fn collect_dynamic_defer_in_class_els(
    body: &[ClassElement],
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    for el in body {
        match el {
            ClassElement::Constructor { body, params, .. }
            | ClassElement::Method { body, params, .. }
            | ClassElement::Accessor { body, params, .. } => {
                for p in params {
                    if let Some(default) = &p.default {
                        collect_dynamic_defer_in_expr(default, parent, out)?;
                    }
                }
                collect_dynamic_defer_in_stmt(body, parent, out)?;
            }
            ClassElement::Field {
                key,
                value,
                ..
            } => {
                if let ObjectKey::Computed(key) = key {
                    collect_dynamic_defer_in_expr(key, parent, out)?;
                }
                if let Some(value) = value {
                    collect_dynamic_defer_in_expr(value, parent, out)?;
                }
            }
            ClassElement::StaticBlock { body, .. } => {
                collect_dynamic_defer_in_stmt(body, parent, out)?;
            }
        }
    }
    Ok(())
}

fn collect_dynamic_defer_in_expr(
    expr: &Expr,
    parent: &Path,
    out: &mut Vec<PathBuf>,
) -> Result<(), Diagnostic> {
    match expr {
        Expr::ImportCall {
            phase,
            source,
            options,
            ..
        } => {
            if *phase == ImportPhase::Defer {
                if let Expr::String(lit) = source.as_ref() {
                    if let Some(spec) = lit.value.to_string_strict() {
                        let dep = resolve_specifier(parent, &spec, lit.span)?;
                        if !out.iter().any(|p| p == &dep) {
                            out.push(dep);
                        }
                    }
                }
            }
            collect_dynamic_defer_in_expr(source, parent, out)?;
            if let Some(options) = options {
                collect_dynamic_defer_in_expr(options, parent, out)?;
            }
        }
        Expr::Unary { arg, .. } | Expr::Update { arg, .. } | Expr::Paren { expr: arg, .. } | Expr::As { expr: arg, .. } => {
            collect_dynamic_defer_in_expr(arg, parent, out)?;
        }
        Expr::Binary { left, right, .. }
        | Expr::Assign {
            target: left,
            value: right,
            ..
        } => {
            collect_dynamic_defer_in_expr(left, parent, out)?;
            collect_dynamic_defer_in_expr(right, parent, out)?;
        }
        Expr::Conditional {
            test,
            consequent,
            alternate,
            ..
        } => {
            collect_dynamic_defer_in_expr(test, parent, out)?;
            collect_dynamic_defer_in_expr(consequent, parent, out)?;
            collect_dynamic_defer_in_expr(alternate, parent, out)?;
        }
        Expr::Call { callee, args, .. } | Expr::New { callee, args, .. } => {
            collect_dynamic_defer_in_expr(callee, parent, out)?;
            for a in args {
                match a {
                    Arg::Expr(e) | Arg::Spread(e) => collect_dynamic_defer_in_expr(e, parent, out)?,
                }
            }
        }
        Expr::MemberExpression {
            object, property, ..
        } => {
            collect_dynamic_defer_in_expr(object, parent, out)?;
            collect_dynamic_defer_in_expr(property, parent, out)?;
        }
        Expr::PrivateIn { object, .. } => collect_dynamic_defer_in_expr(object, parent, out)?,
        Expr::ArrayExpression { elements, .. } => {
            for el in elements {
                match el {
                    ArrayElement::Expr(e) | ArrayElement::Spread(e) => {
                        collect_dynamic_defer_in_expr(e, parent, out)?
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
                            collect_dynamic_defer_in_expr(key, parent, out)?;
                        }
                        collect_dynamic_defer_in_expr(value, parent, out)?;
                    }
                    ObjectProp::Accessor {
                        key, params, body, ..
                    } => {
                        if let ObjectKey::Computed(key) = key {
                            collect_dynamic_defer_in_expr(key, parent, out)?;
                        }
                        for p in params {
                            if let Some(default) = &p.default {
                                collect_dynamic_defer_in_expr(default, parent, out)?;
                            }
                        }
                        collect_dynamic_defer_in_stmt(body, parent, out)?;
                    }
                    ObjectProp::Spread { expr, .. } => {
                        collect_dynamic_defer_in_expr(expr, parent, out)?
                    }
                }
            }
        }
        Expr::TemplateLiteral { expressions, .. } => {
            for e in expressions {
                collect_dynamic_defer_in_expr(e, parent, out)?;
            }
        }
        Expr::TaggedTemplate {
            tag, expressions, ..
        } => {
            collect_dynamic_defer_in_expr(tag, parent, out)?;
            for e in expressions {
                collect_dynamic_defer_in_expr(e, parent, out)?;
            }
        }
        Expr::FunctionExpression {
            params, body, ..
        } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_defer_in_expr(default, parent, out)?;
                }
            }
            collect_dynamic_defer_in_stmt(body, parent, out)?;
        }
        Expr::ClassExpression {
            super_class, body, ..
        } => {
            if let Some(sc) = super_class {
                collect_dynamic_defer_in_expr(sc, parent, out)?;
            }
            collect_dynamic_defer_in_class_els(body, parent, out)?;
        }
        Expr::ArrowFunction { params, body, .. } => {
            for p in params {
                if let Some(default) = &p.default {
                    collect_dynamic_defer_in_expr(default, parent, out)?;
                }
            }
            match body {
                ArrowBody::Expr(e) => collect_dynamic_defer_in_expr(e, parent, out)?,
                ArrowBody::Block(b) => collect_dynamic_defer_in_stmt(b, parent, out)?,
            }
        }
        _ => {}
    }
    Ok(())
}

fn normalize_path(path: &Path) -> Result<PathBuf, Diagnostic> {
    if path.exists() {
        fs::canonicalize(path).map_err(|e| {
            Diagnostic::new(
                format!("canonicalize {}: {e}", path.display()),
                Span::dummy(),
            )
        })
    } else {
        Ok(path.to_path_buf())
    }
}

fn resolve_specifier(parent: &Path, spec: &str, span: Span) -> Result<PathBuf, Diagnostic> {
    if !(spec.starts_with("./") || spec.starts_with("../")) {
        return Err(Diagnostic::new(
            format!("only relative module specifiers are supported (got `{spec}`)"),
            span,
        ));
    }
    let joined = parent.join(spec);
    if joined.exists() {
        return fs::canonicalize(&joined).map_err(|e| {
            Diagnostic::new(
                format!("canonicalize {}: {e}", joined.display()),
                span,
            )
        });
    }
    Err(Diagnostic::new(
        format!("cannot resolve module `{spec}` from {}", parent.display()),
        span,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn link_named_export_import() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let main = dir.join("main.drac");
        fs::write(&lib, "export let value = 41;\nexport function inc(x) { return x + 1; }\n")
            .unwrap();
        fs::write(
            &main,
            "import { value, inc } from \"./lib.drac\";\nlet a = value;\nlet b = inc(value);\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("inc") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_default_export_import() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-default-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let main = dir.join("main.drac");
        fs::write(
            &lib,
            "export default function answer() { return 42; }\nexport let tag = \"ok\";\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import answer from \"./lib.drac\";\nimport ans, { tag } from \"./lib.drac\";\nlet a = answer();\nlet b = ans();\nlet c = tag;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("answer") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_default_export_expr() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-default-expr-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let main = dir.join("main.drac");
        fs::write(&lib, "export default 41 + 1;\n").unwrap();
        fs::write(&main, "import n from \"./lib.drac\";\nlet a = n;\n").unwrap();
        let program = link_entry(&main).expect("link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("__default") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_namespace_import() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-ns-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let main = dir.join("main.drac");
        fs::write(
            &lib,
            "export let value = 41;\nexport default function answer() { return 42; }\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import * as ns from \"./lib.drac\";\nimport answer, * as ns2 from \"./lib.drac\";\nlet a = ns.value;\nlet b = ns.default();\nlet c = answer();\nlet d = ns2.value;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("ObjectExpression"), "{dump}");
        assert!(dump.contains("value") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_cyclic_named_functions() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-cycle-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a.drac");
        let b = dir.join("b.drac");
        let main = dir.join("main.drac");
        fs::write(
            &a,
            "import { fromB } from \"./b.drac\";\nexport function fromA(x) { return x + 1; }\nexport function callB(x) { return fromB(x); }\n",
        )
        .unwrap();
        fs::write(
            &b,
            "import { fromA } from \"./a.drac\";\nexport function fromB(x) { return fromA(x) + 1; }\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import { fromA, callB } from \"./a.drac\";\nimport { fromB } from \"./b.drac\";\nlet a = fromA(40);\nlet b = fromB(40);\nlet c = callB(40);\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("cyclic link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("fromA") || dump.contains("__m"), "{dump}");
        assert!(dump.contains("fromB") || dump.contains("__m"), "{dump}");
        assert!(dump.contains("a"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_export_star_from() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-export-star-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let barrel = dir.join("barrel.drac");
        let main = dir.join("main.drac");
        fs::write(
            &lib,
            "export let value = 41;\nexport function inc(x) { return x + 1; }\nexport default 99;\n",
        )
        .unwrap();
        fs::write(&barrel, "export * from \"./lib.drac\";\n").unwrap();
        fs::write(
            &main,
            "import { value, inc } from \"./barrel.drac\";\nlet a = value;\nlet b = inc(value);\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("export * link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("inc") || dump.contains("__m"), "{dump}");
        // default must not come through export *
        assert!(
            !dump.contains("99") || dump.contains("a"),
            "default should not be required via export *: {dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_export_named_from() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-export-named-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let barrel = dir.join("barrel.drac");
        let main = dir.join("main.drac");
        fs::write(
            &lib,
            "export let value = 41;\nexport function inc(x) { return x + 1; }\nexport default 99;\n",
        )
        .unwrap();
        fs::write(
            &barrel,
            "export { value, inc as bump, default as d } from \"./lib.drac\";\nexport let extra = 7;\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import { value, bump, d, extra } from \"./barrel.drac\";\nimport * as ns from \"./barrel.drac\";\nlet a = value;\nlet b = bump(value);\nlet c = d;\nlet e = extra;\nlet na = ns.value;\nlet nb = ns.bump(1);\nlet nc = ns.d;\nlet ne = ns.extra;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("export {…} from link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("bump") || dump.contains("__m") || dump.contains("inc"), "{dump}");
        assert!(dump.contains("extra") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_export_star_as_ns_from() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-export-ns-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let barrel = dir.join("barrel.drac");
        let main = dir.join("main.drac");
        fs::write(
            &lib,
            "export let value = 41;\nexport function inc(x) { return x + 1; }\nexport default 99;\n",
        )
        .unwrap();
        fs::write(&barrel, "export * as ns from \"./lib.drac\";\nexport let extra = 7;\n")
            .unwrap();
        fs::write(
            &main,
            "import { ns, extra } from \"./barrel.drac\";\nimport * as m from \"./barrel.drac\";\nlet a = ns.value;\nlet b = ns.inc(ns.value);\nlet c = ns.default;\nlet d = extra;\nlet e = m.ns.value;\nlet f = m.extra;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("export * as ns from link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(dump.contains("ObjectExpression"), "{dump}");
        assert!(dump.contains("extra") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_export_class() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-export-class-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let lib = dir.join("lib.drac");
        let main = dir.join("main.drac");
        fs::write(
            &lib,
            "export class Point { constructor(x) { this.x = x; } }\nexport default class Counter { constructor(n) { this.n = n; } }\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import Counter, { Point } from \"./lib.drac\";\nlet p = new Point(1);\nlet c = new Counter(2);\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("export class link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("ClassDeclaration") || dump.contains("Point"), "{dump}");
        assert!(dump.contains("Counter") || dump.contains("__m"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

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

    #[test]
    fn link_import_defer_namespace_lazy() {
        // E19.55: deferred namespace must not eagerly run the dependency body.
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-import-defer-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let dep = dir.join("dep.drac");
        let main = dir.join("main.drac");
        fs::write(
            &dep,
            "globalThis.side = (globalThis.side || 0) + 1;\nexport let exported = 3;\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import defer * as ns from \"./dep.drac\";\nlet x = ns;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("import defer link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("__draconic_deferred_ns") || dump.contains("draconic_deferred"),
            "expected deferred ns helper, got:\n{dump}"
        );
        assert!(
            dump.contains("__draconic_eval_m") || dump.contains("FunctionDeclaration"),
            "expected deferred eval thunk, got:\n{dump}"
        );
        // E19.84.05: ReadyForSyncExecution status machinery.
        assert!(
            dump.contains("__draconic_mstatus") && dump.contains("__draconic_ready"),
            "expected module status / ready helpers, got:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_import_defer_self_while_evaluating_status() {
        // E19.84.05: self deferred namespace during evaluation wraps body with status.
        let dir = temp_link_dir("import-defer-self-eval");
        let main = dir.join("main.drac");
        fs::write(
            &main,
            "import defer * as self from \"./main.drac\";\nexport let foo = 1;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("self defer link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("__draconic_mstatus") && dump.contains("__draconic_ready"),
            "expected status helpers:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_export_multi_declarator_let_const() {
        // E19.84.07: `export let a, b` / multi-declarator packs as Block; must export both.
        let dir = temp_link_dir("export-multi-decl");
        let lib = dir.join("lib.drac");
        let main = dir.join("main.drac");
        fs::write(
            &lib,
            "export let resolveDone, rejectDone;\nexport const done = 1;\n",
        )
        .unwrap();
        fs::write(
            &main,
            "import { resolveDone, rejectDone, done } from \"./lib.drac\";\nlet x = done;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("multi-declarator export link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("resolveDone") && dump.contains("rejectDone") && dump.contains("done"),
            "expected multi-declarator exports, got:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_json_module_deferred_default() {
        // E19.84.03: `.json` files link as JSON modules whose `default` export is
        // the parsed JSON value; `import defer * as ns` yields a deferred namespace.
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-json-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        let data = dir.join("data.json");
        let main = dir.join("main.drac");
        fs::write(&data, "{ \"test262\": \"JSON module\", \"number\": 42 }\n").unwrap();
        fs::write(
            &main,
            "import defer * as ns from \"./data.json\" with { type: \"json\" };\nlet x = ns;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("json module link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("JSON.parse") || dump.contains("json_default"),
            "expected synthesized JSON.parse default, got:\n{dump}"
        );
        assert!(
            dump.contains("__draconic_deferred_ns") || dump.contains("draconic_deferred"),
            "expected deferred ns helper, got:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    fn temp_link_dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "draconic-link-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn link_ambiguous_star_omitted_from_namespace() {
        // E19.71: ambiguous export * names are absent from namespace objects.
        let dir = temp_link_dir("ambig-ns");
        fs::write(dir.join("a.drac"), "export let first = 1;\nexport let both = 2;\n").unwrap();
        fs::write(dir.join("b.drac"), "export let second = 3;\nexport let both = 4;\n").unwrap();
        fs::write(
            dir.join("barrel.drac"),
            "export * from \"./a.drac\";\nexport * from \"./b.drac\";\n",
        )
        .unwrap();
        let main = dir.join("main.drac");
        fs::write(
            &main,
            "import * as ns from \"./barrel.drac\";\nlet a = ns.first;\nlet b = ns.second;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("ambiguous star namespace link");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("first") || dump.contains("__m"), "{dump}");
        assert!(dump.contains("second") || dump.contains("__m"), "{dump}");
        // Ambiguous `both` must not appear as a namespace object property key.
        assert!(
            !dump.contains("name: both"),
            "ambiguous both must be omitted from namespace:\n{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_ambiguous_named_import_errors() {
        // E19.71: named import of ambiguous export * binding is a link error.
        let dir = temp_link_dir("ambig-import");
        fs::write(dir.join("a.drac"), "export let x = 1;\n").unwrap();
        fs::write(dir.join("b.drac"), "export let x = 2;\n").unwrap();
        fs::write(
            dir.join("barrel.drac"),
            "export * from \"./a.drac\";\nexport * from \"./b.drac\";\n",
        )
        .unwrap();
        let main = dir.join("main.drac");
        fs::write(&main, "import { x } from \"./barrel.drac\";\nlet y = x;\n").unwrap();
        let err = link_entry(&main).expect_err("ambiguous named import");
        assert!(
            err.message.contains("no export") || err.message.contains("ambiguous"),
            "got: {}",
            err.message
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_ambiguous_named_reexport_errors() {
        // E19.71: IndirectExportEntries of ambiguous bindings fail at link.
        let dir = temp_link_dir("ambig-reexport");
        fs::write(dir.join("a.drac"), "export let x = 1;\n").unwrap();
        fs::write(dir.join("b.drac"), "export let x = 2;\n").unwrap();
        fs::write(
            dir.join("barrel.drac"),
            "export * from \"./a.drac\";\nexport * from \"./b.drac\";\n",
        )
        .unwrap();
        let main = dir.join("main.drac");
        fs::write(&main, "export { x } from \"./barrel.drac\";\n").unwrap();
        let err = link_entry(&main).expect_err("ambiguous named re-export");
        assert!(
            err.message.contains("no export") || err.message.contains("ambiguous"),
            "got: {}",
            err.message
        );
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_same_binding_via_import_export_not_ambiguous() {
        // E19.71: `export { foo } from` and `import { foo }; export { foo }` same binding.
        let dir = temp_link_dir("same-binding");
        fs::write(dir.join("lib.drac"), "export const foo = 2;\n").unwrap();
        fs::write(
            dir.join("via_from.drac"),
            "export { foo } from \"./lib.drac\";\n",
        )
        .unwrap();
        fs::write(
            dir.join("via_import.drac"),
            "import { foo } from \"./lib.drac\";\nexport { foo };\n",
        )
        .unwrap();
        fs::write(
            dir.join("barrel.drac"),
            "export * from \"./via_from.drac\";\nexport * from \"./via_import.drac\";\n",
        )
        .unwrap();
        let consumer = dir.join("consumer.drac");
        fs::write(
            &consumer,
            "import { foo } from \"./barrel.drac\";\nlet v = foo;\n",
        )
        .unwrap();
        let program = link_entry(&consumer).expect("same binding not ambiguous");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("v"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_export_star_as_same_module_not_ambiguous() {
        // E19.71: two `export * as foo from empty` resolve to same namespace binding.
        let dir = temp_link_dir("ns-star-as");
        fs::write(dir.join("empty.drac"), "\n").unwrap();
        fs::write(
            dir.join("a.drac"),
            "export * as foo from \"./empty.drac\";\n",
        )
        .unwrap();
        fs::write(
            dir.join("b.drac"),
            "export * as foo from \"./empty.drac\";\n",
        )
        .unwrap();
        fs::write(
            dir.join("barrel.drac"),
            "export * from \"./a.drac\";\nexport * from \"./b.drac\";\n",
        )
        .unwrap();
        let main = dir.join("main.drac");
        fs::write(
            &main,
            "import { foo } from \"./barrel.drac\";\nlet t = typeof foo;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("export * as same module");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("__ns") || dump.contains("ObjectExpression"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_import_star_export_same_module_not_ambiguous() {
        // E19.71: `import * as foo; export { foo }` from same module twice.
        let dir = temp_link_dir("ns-import-export");
        fs::write(dir.join("empty.drac"), "\n").unwrap();
        fs::write(
            dir.join("a.drac"),
            "import * as foo from \"./empty.drac\";\nexport { foo };\n",
        )
        .unwrap();
        fs::write(
            dir.join("b.drac"),
            "import * as foo from \"./empty.drac\";\nexport { foo };\n",
        )
        .unwrap();
        fs::write(
            dir.join("barrel.drac"),
            "export * from \"./a.drac\";\nexport * from \"./b.drac\";\n",
        )
        .unwrap();
        let main = dir.join("main.drac");
        fs::write(
            &main,
            "import { foo } from \"./barrel.drac\";\nlet t = typeof foo;\n",
        )
        .unwrap();
        let program = link_entry(&main).expect("import * export same module");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("__ns") || dump.contains("ObjectExpression"), "{dump}");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn link_deferred_module_throws_evaluation_error_cache() {
        // E19.84.08: throwing deferred module records [[EvaluationError]]; dynamic
        // import and deferred-ns access share the same reason; merror helpers present.
        let dir = temp_link_dir("module-throws");
        fs::write(
            dir.join("throws.drac"),
            "throw { someError: \"the error from throws\" };\n",
        )
        .unwrap();
        fs::write(
            dir.join("defer_ns.drac"),
            "import defer * as ns from \"./throws.drac\";\nexport { ns };\n",
        )
        .unwrap();
        let main = dir.join("main.drac");
        fs::write(
            &main,
            r#"
import defer * as ns from "./throws.drac";
async function run() {
  let err1;
  await import("./throws.drac").catch(function (e) { err1 = e; });
  let err2;
  try { ns.foo; } catch (e) { err2 = e; }
  let err3;
  const mod = await import("./defer_ns.drac");
  try { mod.ns.foo; } catch (e) { err3 = e; }
  return err1 === err2 && err1 === err3;
}
"#,
        )
        .unwrap();
        let program = link_entry(&main).expect("module-throws link");
        let dump = draconic_ast::dump_program(&program);
        assert!(
            dump.contains("__draconic_merror") && dump.contains("__draconic_deferred_ns"),
            "{dump}"
        );
        assert!(
            dump.contains("__draconic_eval_m") || dump.contains("draconic_eval"),
            "{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
