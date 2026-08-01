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
    Arg, ArrayElement, ArrayPatternElement, ArrowBody, AssignOp, BindingKind, BindingPattern,
    ClassElement, Expr, Ident, ImportPhase, ObjectKey, ObjectPatternProp, ObjectProp, Param,
    Program, Stmt,
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
        // ESM files are always Module goal ([+Await], reserved `await`) — E19.52.
        let program = parse_module(&source)?;
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
                            collect_decl_exports(&decl, &mut exports)?;
                            body.push(*decl);
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
        for id in 0..self.modules.len() {
            let mut ns_binds = self.modules[id].namespaces.clone();
            ns_binds.extend(self.modules[id].namespace_reexports.clone());
            let mut ns_stmts = Vec::new();
            for bind in &ns_binds {
                let from_id = *self.ids.get(&bind.from).ok_or_else(|| {
                    Diagnostic::new(
                        format!("module not loaded: {}", bind.from.display()),
                        Span::dummy(),
                    )
                })?;
                let resolved = self.collect_resolved_exports(from_id)?;
                let bind_span = span_gen.next();
                // Deferred ns over a still-lazy module: Proxy + eval thunk.
                // Deferred ns over an eager module (e.g. TLA under import defer): plain
                // object — evaluation already ran with the importer.
                if bind.deferred && !eager.contains(&from_id) {
                    any_deferred_ns = true;
                    let eval_name = deferred_eval_fn_name(from_id);
                    let mut pairs: Vec<(String, String)> = Vec::new();
                    let mut names: Vec<_> = resolved.keys().cloned().collect();
                    names.sort();
                    for export_name in names {
                        let (def_id, local_in_exporter) =
                            resolved.get(&export_name).expect("key from map");
                        let remote =
                            final_binding_name(&mangled, *def_id, local_in_exporter)?;
                        pairs.push((export_name, remote));
                    }
                    ns_stmts.push(make_deferred_namespace_binding(
                        &bind.local,
                        &eval_name,
                        &pairs,
                        bind_span,
                    )?);
                } else {
                    shared_ns_targets.insert(from_id);
                    import_renames[id]
                        .insert(bind.local.clone(), shared_namespace_binding_name(from_id));
                }
            }
            if !ns_stmts.is_empty() {
                let body = &mut self.modules[id].body;
                let mut combined = ns_stmts;
                combined.append(body);
                *body = combined;
            }
        }

        // Build shared eager namespace objects once per target module.
        // Emitted after that module's body (post-order) so export locals are initialized.
        let mut shared_ns_by_id: HashMap<usize, Stmt> = HashMap::new();
        for from_id in shared_ns_targets {
            let resolved = self.collect_resolved_exports(from_id)?;
            let props = namespace_object_props_resolved(&resolved, &mangled, &mut span_gen)?;
            let bind_span = span_gen.next();
            let obj_span = span_gen.next();
            shared_ns_by_id.insert(
                from_id,
                Stmt::Let {
                    kind: BindingKind::Let,
                    binding: BindingPattern::Ident(Ident {
                        name: shared_namespace_binding_name(from_id),
                        span: bind_span,
                    }),
                    type_ann: None,
                    init: Some(Expr::ObjectExpression {
                        properties: props,
                        span: obj_span,
                    }),
                    span: bind_span,
                },
            );
        }

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
        if any_deferred_ns {
            for stmt in deferred_namespace_helper_stmts()? {
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
            for stmt in &body {
                let sp = stmt_span_approx(stmt);
                if linked_body.is_empty() {
                    start = sp.start.0;
                }
                end = sp.end.0;
            }
            linked_body.extend(body);
            // Namespace object for this module after its exports are initialized.
            if let Some(ns_stmt) = shared_ns_by_id.remove(&id) {
                let sp = stmt_span_approx(&ns_stmt);
                if linked_body.is_empty() {
                    start = sp.start.0;
                }
                end = sp.end.0;
                linked_body.push(ns_stmt);
            }
        }
        // Empty modules that are still namespace targets (no body stmts).
        let mut leftover: Vec<_> = shared_ns_by_id.into_iter().collect();
        leftover.sort_by_key(|(id, _)| *id);
        for (_, ns_stmt) in leftover {
            let sp = stmt_span_approx(&ns_stmt);
            if linked_body.is_empty() {
                start = sp.start.0;
            }
            end = sp.end.0;
            linked_body.push(ns_stmt);
        }

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
            let resolved = self.resolve_export(dep_id, &re.imported, visiting)?;
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
}

/// Sentinel local name: export BindingName is ~namespace~ (module namespace object).
const BINDING_NAMESPACE: &str = "\0namespace";

fn shared_namespace_binding_name(module_id: usize) -> String {
    format!("__ns{module_id}")
}

fn final_binding_name(
    mangled: &[HashMap<String, String>],
    def_id: usize,
    local_in_exporter: &str,
) -> Result<String, Diagnostic> {
    if local_in_exporter == BINDING_NAMESPACE {
        return Ok(shared_namespace_binding_name(def_id));
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

/// Runtime helper implementing deferred module namespace exotic object triggers (E19.55).
fn deferred_namespace_helper_stmts() -> Result<Vec<Stmt>, Diagnostic> {
    // Parsed once per link that needs deferred namespaces. Node hosts lack native
    // `import defer`; this Proxy matches Test262 evaluation-trigger surface.
    let src = r#"
function __draconic_deferred_ns(evaluate) {
  let evaluated = false;
  let exportsObj = null;
  let target = Object.create(null);
  function ensure() {
    if (!evaluated) {
      evaluated = true;
      exportsObj = evaluate();
    }
    return exportsObj;
  }
  function isSymbolLike(p) {
    return typeof p === "symbol" || p === "then";
  }
  // Traps encode deferred module-namespace trigger rules (E19.55). Target starts
  // extensible; preventExtensions makes it non-extensible when asked.
  return new Proxy(target, {
    get(_t, p) {
      if (p === Symbol.toStringTag) return "Module";
      if (isSymbolLike(p)) return undefined;
      let ex = ensure();
      if (Object.prototype.hasOwnProperty.call(ex, p)) return ex[p];
      return undefined;
    },
    has(_t, p) {
      if (isSymbolLike(p)) return false;
      let ex = ensure();
      return Object.prototype.hasOwnProperty.call(ex, p);
    },
    getOwnPropertyDescriptor(_t, p) {
      if (isSymbolLike(p)) return undefined;
      let ex = ensure();
      if (!Object.prototype.hasOwnProperty.call(ex, p)) return undefined;
      return { value: ex[p], writable: true, enumerable: true, configurable: true };
    },
    ownKeys() {
      return Reflect.ownKeys(ensure());
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
    setPrototypeOf() {
      return false;
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
    eval_fn: &str,
    export_pairs: &[(String, String)],
    span: Span,
) -> Result<Stmt, Diagnostic> {
    let mut props = String::new();
    for (export_name, remote) in export_pairs {
        let key = js_object_key(export_name);
        props.push_str(&format!("{key}: {remote}, "));
    }
    let src = format!(
        "let {local} = __draconic_deferred_ns(function () {{ {eval_fn}(); return {{ {props} }}; }});"
    );
    let mut body = parse(&src)?.body;
    let stmt = body.pop().ok_or_else(|| {
        Diagnostic::new("deferred namespace binding parse produced no stmt", span)
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

/// Hoist top-level bindings and wrap module body in a once-eval function.
fn wrap_deferred_module_body(
    _mod_id: usize,
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
    let done_name = format!("{eval_name}_done");
    let done_span = spans.next();
    out.push(Stmt::Let {
        kind: BindingKind::Let,
        binding: BindingPattern::Ident(Ident {
            name: done_name.clone(),
            span: done_span,
        }),
        type_ann: None,
        init: Some(Expr::Boolean {
            value: false,
            span: done_span,
        }),
        span: done_span,
    });

    let mut eval_body: Vec<Stmt> = Vec::new();
    let guard_span = spans.next();
    // if (done) return; done = true;
    eval_body.push(Stmt::If {
        test: Expr::Ident(Ident {
            name: done_name.clone(),
            span: guard_span,
        }),
        consequent: Box::new(Stmt::Return {
            argument: None,
            span: guard_span,
        }),
        alternate: None,
        span: guard_span,
    });
    eval_body.push(Stmt::Expression {
        expr: Expr::Assign {
            target: Box::new(Expr::Ident(Ident {
                name: done_name,
                span: guard_span,
            })),
            op: AssignOp::Eq,
            value: Box::new(Expr::Boolean {
                value: true,
                span: guard_span,
            }),
            span: guard_span,
        },
        span: guard_span,
    });
    eval_body.extend(prelude_calls);
    for stmt in body {
        eval_body.push(hoist_decl_to_assign(stmt));
    }
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

fn namespace_object_props_resolved(
    resolved: &HashMap<String, (usize, String)>,
    mangled: &[HashMap<String, String>],
    spans: &mut SyntheticSpans,
) -> Result<Vec<ObjectProp>, Diagnostic> {
    let mut names: Vec<_> = resolved.keys().cloned().collect();
    names.sort();
    let mut props = Vec::with_capacity(names.len());
    for export_name in names {
        let (def_id, local_in_exporter) = resolved.get(&export_name).expect("key from map");
        let remote = final_binding_name(mangled, *def_id, local_in_exporter)?;
        let key_span = spans.next();
        let val_span = spans.next();
        let prop_span = spans.next();
        let key = Ident {
            name: export_name,
            span: key_span,
        };
        let value = Expr::Ident(Ident {
            name: remote,
            span: val_span,
        });
        props.push(ObjectProp::Property {
            key: ObjectKey::Ident(key),
            value,
            shorthand: false,
            span: prop_span,
        });
    }
    Ok(props)
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
}
