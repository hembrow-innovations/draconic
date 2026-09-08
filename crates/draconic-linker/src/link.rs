use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use draconic_ast::{Program, Stmt};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_parser::parse;

use crate::eval::{
    make_module_status_assign, module_body_has_tla, wrap_async_eager_module_body,
    wrap_deferred_module_body,
};
use crate::load::{top_level_names, Loader};
use crate::namespace::{
    deferred_eval_fn_name, deferred_module_status_helper_stmts, deferred_namespace_binding_name,
    deferred_namespace_helper_stmts, final_binding_name, make_call_stmt,
    make_deferred_namespace_binding, make_shared_namespace_binding, shared_namespace_binding_name,
    shared_namespace_helper_stmts,
};
use crate::path::normalize_path;
use crate::rename::{rename_stmt, ScopeStack};
use crate::spans::{stmt_span_approx, uniqueify_stmt_spans, SyntheticSpans};

impl Loader {
    pub(crate) fn link(&mut self, entry: &Path) -> Result<Program, Diagnostic> {
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
                let (def_id, local_in_exporter) = resolved.get(&export_name).expect("key from map");
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

        // Thunk emission order: load ids then entry last (stable, independent of eval).
        let mut thunk_order: Vec<usize> = (0..self.modules.len()).collect();
        thunk_order.retain(|&id| id != entry_id);
        thunk_order.push(entry_id);

        // Snapshot HasTLA before any body is taken (deferred thunks / eager emit).
        let has_tla: Vec<bool> = self
            .modules
            .iter()
            .map(|m| module_body_has_tla(&m.body))
            .collect();

        // E19.84.09: eager body order follows InnerModuleEvaluation (ModuleRequests
        // evaluation list), not DFS load post-order.
        let eval_order = self.compute_inner_module_eval_order(entry_id, &eager, &has_tla);
        let async_mods = self.compute_async_eval_modules(&eager, &has_tla);
        // Precompute async deps per module while TLA snapshot is valid (before take).
        let async_deps_by_id: Vec<Vec<usize>> = (0..self.modules.len())
            .map(|id| {
                self.evaluation_list(id, &has_tla)
                    .into_iter()
                    .filter(|d| async_mods.contains(d))
                    .collect()
            })
            .collect();

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
        let track_status = any_deferred_ns || has_lazy_modules;
        if track_status {
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
        for id in &thunk_order {
            if let Some(thunks) = deferred_thunks.remove(id) {
                linked_body.extend(thunks);
            }
        }

        // E19.84.09: promise slots for async module evaluation (TLA interleaving).
        let needs_async_eval = !async_mods.is_empty();
        if needs_async_eval {
            let mp_src = "let __draconic_mp = [];\n";
            for mut stmt in parse(mp_src)?.body {
                uniqueify_stmt_spans(&mut stmt, &mut span_gen);
                linked_body.push(stmt);
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
        for &id in &eval_order {
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
            for stmt in &body {
                let sp = stmt_span_approx(stmt);
                if linked_body.is_empty() {
                    start = sp.start.0;
                }
                end = sp.end.0;
            }
            let is_async = async_mods.contains(&id);
            if is_async {
                let async_deps = &async_deps_by_id[id];
                let is_entry = id == entry_id;
                let wrapped = wrap_async_eager_module_body(
                    id,
                    body,
                    async_deps,
                    track_status,
                    is_entry,
                    &mut span_gen,
                )?;
                linked_body.extend(wrapped);
            } else {
                // E19.84.05: mark eager module ~evaluating~ … ~evaluated~ around body so
                // deferred-namespace EnsureDeferredNamespaceEvaluation can TypeError.
                if track_status {
                    linked_body.push(make_module_status_assign(id, 1, span_gen.next()));
                }
                linked_body.extend(body);
                if track_status {
                    linked_body.push(make_module_status_assign(id, 3, span_gen.next()));
                }
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
                    for tla_id in self.gather_async_transitive(dep_id, None) {
                        stack.push(tla_id);
                    }
                }
            }
            // E19.84.06: dynamic `import.defer` — same GatherAsynchronousTransitiveDependencies.
            for from in &self.modules[id].dynamic_defer_targets {
                if let Some(&dep_id) = self.ids.get(from) {
                    for tla_id in self.gather_async_transitive(dep_id, None) {
                        stack.push(tla_id);
                    }
                }
            }
        }
        eager
    }

    /// InnerModuleEvaluation evaluationList for `id` (E19.84.09).
    ///
    /// For each ModuleRequest in source order: ~defer~ → GatherAsynchronousTransitive
    /// Dependencies; else append the required module if not already listed.
    fn evaluation_list(&self, id: usize, has_tla: &[bool]) -> Vec<usize> {
        let mut list = Vec::new();
        for req in &self.modules[id].module_requests {
            let Some(&req_id) = self.ids.get(&req.path) else {
                continue;
            };
            if req.deferred {
                for async_id in self.gather_async_transitive(req_id, Some(has_tla)) {
                    if !list.contains(&async_id) {
                        list.push(async_id);
                    }
                }
            } else if !list.contains(&req_id) {
                list.push(req_id);
            }
        }
        list
    }

    /// DFS InnerModuleEvaluation order over eager modules (deps before importers).
    fn compute_inner_module_eval_order(
        &self,
        entry_id: usize,
        eager: &HashSet<usize>,
        has_tla: &[bool],
    ) -> Vec<usize> {
        let mut order = Vec::new();
        let mut visiting = HashSet::new();
        let mut done = HashSet::new();
        self.inner_module_eval_order_rec(
            entry_id,
            eager,
            has_tla,
            &mut visiting,
            &mut done,
            &mut order,
        );
        // Any eager module not reached from entry (should be rare) — append stably.
        let mut rest: Vec<_> = eager
            .iter()
            .copied()
            .filter(|id| !done.contains(id))
            .collect();
        rest.sort();
        for id in rest {
            self.inner_module_eval_order_rec(
                id,
                eager,
                has_tla,
                &mut visiting,
                &mut done,
                &mut order,
            );
        }
        order
    }

    fn inner_module_eval_order_rec(
        &self,
        id: usize,
        eager: &HashSet<usize>,
        has_tla: &[bool],
        visiting: &mut HashSet<usize>,
        done: &mut HashSet<usize>,
        order: &mut Vec<usize>,
    ) {
        if done.contains(&id) || !eager.contains(&id) {
            return;
        }
        if !visiting.insert(id) {
            return; // cycle: break
        }
        for dep in self.evaluation_list(id, has_tla) {
            self.inner_module_eval_order_rec(dep, eager, has_tla, visiting, done, order);
        }
        visiting.remove(&id);
        done.insert(id);
        order.push(id);
    }

    /// Modules whose evaluation is async: HasTLA or any evaluationList dep is async.
    fn compute_async_eval_modules(
        &self,
        eager: &HashSet<usize>,
        has_tla: &[bool],
    ) -> HashSet<usize> {
        let mut async_mods = HashSet::new();
        let mut changed = true;
        while changed {
            changed = false;
            for &id in eager {
                if async_mods.contains(&id) {
                    continue;
                }
                if has_tla.get(id).copied().unwrap_or(false) {
                    async_mods.insert(id);
                    changed = true;
                    continue;
                }
                if self
                    .evaluation_list(id, has_tla)
                    .into_iter()
                    .any(|d| async_mods.contains(&d))
                {
                    async_mods.insert(id);
                    changed = true;
                }
            }
        }
        async_mods
    }

    /// Modules with TLA (or that reach them) under a deferred import subgraph.
    ///
    /// `has_tla_snapshot` avoids reading bodies after they were moved into thunks.
    fn gather_async_transitive(
        &self,
        start: usize,
        has_tla_snapshot: Option<&[bool]>,
    ) -> Vec<usize> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        self.gather_async_transitive_rec(start, has_tla_snapshot, &mut seen, &mut out);
        out
    }

    fn gather_async_transitive_rec(
        &self,
        id: usize,
        has_tla_snapshot: Option<&[bool]>,
        seen: &mut HashSet<usize>,
        out: &mut Vec<usize>,
    ) {
        if !seen.insert(id) {
            return;
        }
        let has_tla = match has_tla_snapshot {
            Some(flags) => flags.get(id).copied().unwrap_or(false),
            None => module_body_has_tla(&self.modules[id].body),
        };
        if has_tla {
            out.push(id);
            return;
        }
        // Spec: walk all RequestedModules (both phases).
        for req in &self.modules[id].module_requests {
            if let Some(&dep_id) = self.ids.get(&req.path) {
                self.gather_async_transitive_rec(dep_id, has_tla_snapshot, seen, out);
            }
        }
    }
}
