use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use draconic_ast::{ImportPhase, Stmt};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_parser::parse_module;

use crate::dynamic_collect::{collect_dynamic_defer_targets, collect_dynamic_eval_import_targets};
use crate::json::parse_json_module;
use crate::path::{normalize_path, resolve_specifier};
use crate::{PackageLinkContext, ACTIVE_PACKAGES};

pub(crate) struct Loader {
    /// Canonical path → module id (load order).
    pub(crate) ids: HashMap<PathBuf, usize>,
    pub(crate) modules: Vec<ModuleData>,
    /// Optional lock + cache for module-path imports (K06.01).
    pub(crate) packages: Option<Arc<PackageLinkContext>>,
}

pub(crate) struct ModuleData {
    /// Body statements after peeling import/export wrappers.
    pub(crate) body: Vec<Stmt>,
    /// export_name → local name (pre-mangle) in this module (direct exports only).
    pub(crate) exports: HashMap<String, String>,
    /// `export * from` dependency paths (named exports re-exported; not `default`).
    pub(crate) star_reexports: Vec<PathBuf>,
    /// `export { imported as exported } from` named re-exports.
    pub(crate) named_reexports: Vec<NamedReexport>,
    /// `export * as local from` — local binding is the module namespace object.
    pub(crate) namespace_reexports: Vec<NamespaceBind>,
    /// import local → (resolved module path, exported name).
    pub(crate) imports: Vec<ImportBind>,
    /// `import * as local` → resolved module path.
    pub(crate) namespaces: Vec<NamespaceBind>,
    /// Dependencies that must evaluate with this module (named/side-effect/non-defer).
    pub(crate) eval_deps: Vec<PathBuf>,
    /// All ModuleRequest targets (incl. deferred namespace-only) for ReadyForSyncExecution.
    pub(crate) requested: Vec<PathBuf>,
    /// Static ModuleRequests in source order with phase (E19.84.09 evaluation list).
    /// Deduped by (path, deferred) per ModuleRequests static semantics.
    pub(crate) module_requests: Vec<ModuleRequest>,
    /// String-literal `import.defer("…")` targets (E19.84.06). Loaded into the graph
    /// and get deferred namespaces, but do not mark eval unless also an eval_dep.
    pub(crate) dynamic_defer_targets: Vec<PathBuf>,
    /// String-literal evaluation-phase `import("…")` targets (E19.84.08). Loaded into
    /// the graph for linked dynamic import + evaluation-error identity; not eval_deps.
    pub(crate) dynamic_import_targets: Vec<PathBuf>,
}

/// One static ModuleRequest (specifier + phase) for InnerModuleEvaluation.
pub(crate) struct ModuleRequest {
    pub(crate) path: PathBuf,
    /// `import defer` / deferred phase; otherwise evaluation phase.
    pub(crate) deferred: bool,
}

pub(crate) struct NamedReexport {
    pub(crate) from: PathBuf,
    /// Name in the source module (`local` side of the specifier).
    pub(crate) imported: String,
    /// Name under which this module re-exports it.
    pub(crate) exported: String,
}

pub(crate) struct ImportBind {
    pub(crate) local: String,
    pub(crate) from: PathBuf,
    pub(crate) imported: String,
}

#[derive(Clone)]
pub(crate) struct NamespaceBind {
    pub(crate) local: String,
    pub(crate) from: PathBuf,
    /// `import defer * as local` (E19.42 / E19.55).
    pub(crate) deferred: bool,
}

impl Loader {
    pub(crate) fn new(packages: Option<Arc<PackageLinkContext>>) -> Self {
        Self {
            ids: HashMap::new(),
            modules: Vec::new(),
            packages,
        }
    }

    pub(crate) fn load_graph(&mut self, entry: &Path) -> Result<(), Diagnostic> {
        ACTIVE_PACKAGES.with(|slot| {
            *slot.borrow_mut() = self.packages.clone();
        });
        let mut stack = Vec::new();
        let result = self.load_module(entry, &mut stack);
        ACTIVE_PACKAGES.with(|slot| {
            *slot.borrow_mut() = None;
        });
        result
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
        let mut module_requests: Vec<ModuleRequest> = Vec::new();
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
                    // ModuleRequests: phase-aware; same specifier+phase is not duplicated.
                    push_module_request(
                        &mut module_requests,
                        dep.clone(),
                        phase == ImportPhase::Defer,
                    );
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
                        push_module_request(&mut module_requests, dep.clone(), false);
                        eval_deps.push(dep.clone());
                        for s in specifiers {
                            if exports.contains_key(&s.exported.name)
                                || named_reexports
                                    .iter()
                                    .any(|r| r.exported == s.exported.name)
                                || namespace_reexports
                                    .iter()
                                    .any(|r| r.local == s.exported.name)
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
                                || namespace_reexports
                                    .iter()
                                    .any(|r| r.local == s.exported.name)
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
                    declaration, local, ..
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
                    exported, source, ..
                } => {
                    let spec = source.value.to_string_strict().ok_or_else(|| {
                        Diagnostic::new(
                            "module specifier must be a well-formed string".to_string(),
                            source.span,
                        )
                    })?;
                    let dep = resolve_specifier(parent, &spec, source.span)?;
                    dep_paths.push(dep.clone());
                    push_module_request(&mut module_requests, dep.clone(), false);
                    eval_deps.push(dep.clone());
                    if let Some(ns) = exported {
                        if exports.insert(ns.name.clone(), ns.name.clone()).is_some()
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
            module_requests,
            dynamic_defer_targets,
            dynamic_import_targets,
        });
        stack.pop();
        Ok(())
    }
}

pub(crate) fn push_module_request(reqs: &mut Vec<ModuleRequest>, path: PathBuf, deferred: bool) {
    if !reqs
        .iter()
        .any(|r| r.path == path && r.deferred == deferred)
    {
        reqs.push(ModuleRequest { path, deferred });
    }
}

/// Expand multi-declarator export (`export let a, b`) from a Block of Lets into
/// individual declarations (parser packs multi-declarators as `Stmt::Block`).
pub(crate) fn expand_export_decl(decl: Stmt) -> Vec<Stmt> {
    match decl {
        Stmt::Block { body, .. } if body.iter().all(|s| matches!(s, Stmt::Let { .. })) => body,
        other => vec![other],
    }
}

pub(crate) fn collect_decl_exports(
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
        Stmt::Block { body, .. } if body.iter().all(|s| matches!(s, Stmt::Let { .. })) => {
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

pub(crate) fn top_level_names(body: &[Stmt]) -> HashSet<String> {
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

#[cfg(test)]
mod tests {
    use crate::{link_entry, link_entry_with_packages, PackageLinkContext};
    use std::fs;

    /// K06.01: `from "github.com/org/pkg"` resolves via lock + cache root.
    #[test]
    fn link_module_path_import_package_root() {
        let root = std::env::temp_dir().join(format!(
            "draconic-link-pkg-root-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let oid = "0123456789abcdef0123456789abcdef01234567";
        let module_path = "github.com/org/pkg";
        let cache = draconic_pkg::ModuleCache::new(root.join("cache"));
        let pkg_dir = cache.entry_dir(module_path, oid).unwrap();
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(
            pkg_dir.join("index.drac"),
            "export let value = 41;\nexport function inc(x) { return x + 1; }\n",
        )
        .unwrap();
        let hash = draconic_pkg::content_hash_tree(&pkg_dir).unwrap();
        fs::write(pkg_dir.join(".draconic-checkout-oid"), format!("{oid}\n")).unwrap();

        let entry = draconic_pkg::LockEntry::new(
            module_path,
            "1.0.0",
            "https://github.com/org/pkg.git",
            oid,
            hash,
        )
        .unwrap();
        let mut packages = std::collections::BTreeMap::new();
        packages.insert(module_path.to_string(), entry);
        let lock = draconic_pkg::LockFile {
            version: 1,
            packages,
        };
        let ctx = PackageLinkContext { lock, cache };

        let main = root.join("main.drac");
        fs::write(
            &main,
            "import { value, inc } from \"github.com/org/pkg\";\nlet a = value;\nlet b = inc(value);\n",
        )
        .unwrap();

        let program = link_entry_with_packages(&main, Some(&ctx)).expect("link module path");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(
            dump.contains("41") || dump.contains("inc") || dump.contains("__m"),
            "{dump}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// K06.01: `from "github.com/org/pkg/util"` resolves subpath under package root.
    #[test]
    fn link_module_path_import_subpath() {
        let root = std::env::temp_dir().join(format!(
            "draconic-link-pkg-sub-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let oid = "0123456789abcdef0123456789abcdef01234567";
        let module_path = "github.com/org/pkg";
        let cache = draconic_pkg::ModuleCache::new(root.join("cache"));
        let pkg_dir = cache.entry_dir(module_path, oid).unwrap();
        fs::create_dir_all(pkg_dir.join("util")).unwrap();
        fs::write(pkg_dir.join("util.drac"), "export let helper = 7;\n").unwrap();
        let hash = draconic_pkg::content_hash_tree(&pkg_dir).unwrap();
        fs::write(pkg_dir.join(".draconic-checkout-oid"), format!("{oid}\n")).unwrap();

        let entry = draconic_pkg::LockEntry::new(
            module_path,
            "1.0.0",
            "https://github.com/org/pkg.git",
            oid,
            hash,
        )
        .unwrap();
        let mut packages = std::collections::BTreeMap::new();
        packages.insert(module_path.to_string(), entry);
        let lock = draconic_pkg::LockFile {
            version: 1,
            packages,
        };
        let ctx = PackageLinkContext { lock, cache };

        let main = root.join("main.drac");
        fs::write(
            &main,
            "import { helper } from \"github.com/org/pkg/util\";\nlet h = helper;\n",
        )
        .unwrap();

        let program = link_entry_with_packages(&main, Some(&ctx)).expect("link subpath");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("h"), "{dump}");
        assert!(
            dump.contains("7") || dump.contains("helper") || dump.contains("__m"),
            "{dump}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// K06.02: relative import from a package module must not escape package root.
    #[test]
    fn link_rejects_relative_escape_outside_package_root() {
        let root = std::env::temp_dir().join(format!(
            "draconic-link-pkg-escape-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let oid = "0123456789abcdef0123456789abcdef01234567";
        let module_path = "github.com/org/pkg";
        let cache = draconic_pkg::ModuleCache::new(root.join("cache"));
        let pkg_dir = cache.entry_dir(module_path, oid).unwrap();
        fs::create_dir_all(&pkg_dir).unwrap();
        // Sibling of package checkout root (outside the package boundary).
        fs::write(
            pkg_dir.parent().unwrap().join("outside.drac"),
            "export let secret = 99;\n",
        )
        .unwrap();
        // From package root, `../outside.drac` escapes the checkout.
        fs::write(
            pkg_dir.join("index.drac"),
            "export { secret } from \"../outside.drac\";\n",
        )
        .unwrap();
        let hash = draconic_pkg::content_hash_tree(&pkg_dir).unwrap();
        fs::write(pkg_dir.join(".draconic-checkout-oid"), format!("{oid}\n")).unwrap();

        let entry = draconic_pkg::LockEntry::new(
            module_path,
            "1.0.0",
            "https://github.com/org/pkg.git",
            oid,
            hash,
        )
        .unwrap();
        let mut packages = std::collections::BTreeMap::new();
        packages.insert(module_path.to_string(), entry);
        let lock = draconic_pkg::LockFile {
            version: 1,
            packages,
        };
        let ctx = PackageLinkContext { lock, cache };

        let main = root.join("main.drac");
        fs::write(
            &main,
            "import { secret } from \"github.com/org/pkg\";\nlet s = secret;\n",
        )
        .unwrap();

        let err = link_entry_with_packages(&main, Some(&ctx)).expect_err("must reject escape");
        let msg = err.message.clone();
        assert!(
            msg.contains("package boundary")
                || msg.contains("outside package")
                || msg.contains("escape"),
            "{msg}"
        );
        let _ = fs::remove_dir_all(&root);
    }

    /// K06.02: relative imports that stay inside the package root still link.
    #[test]
    fn link_allows_relative_within_package_root() {
        let root = std::env::temp_dir().join(format!(
            "draconic-link-pkg-within-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        fs::create_dir_all(&root).unwrap();
        let oid = "0123456789abcdef0123456789abcdef01234567";
        let module_path = "github.com/org/pkg";
        let cache = draconic_pkg::ModuleCache::new(root.join("cache"));
        let pkg_dir = cache.entry_dir(module_path, oid).unwrap();
        fs::create_dir_all(pkg_dir.join("nested")).unwrap();
        fs::write(
            pkg_dir.join("index.drac"),
            "export { helper } from \"./nested/util.drac\";\n",
        )
        .unwrap();
        fs::write(pkg_dir.join("nested/util.drac"), "export let helper = 7;\n").unwrap();
        let hash = draconic_pkg::content_hash_tree(&pkg_dir).unwrap();
        fs::write(pkg_dir.join(".draconic-checkout-oid"), format!("{oid}\n")).unwrap();

        let entry = draconic_pkg::LockEntry::new(
            module_path,
            "1.0.0",
            "https://github.com/org/pkg.git",
            oid,
            hash,
        )
        .unwrap();
        let mut packages = std::collections::BTreeMap::new();
        packages.insert(module_path.to_string(), entry);
        let lock = draconic_pkg::LockFile {
            version: 1,
            packages,
        };
        let ctx = PackageLinkContext { lock, cache };

        let main = root.join("main.drac");
        fs::write(
            &main,
            "import { helper } from \"github.com/org/pkg\";\nlet h = helper;\n",
        )
        .unwrap();

        let program = link_entry_with_packages(&main, Some(&ctx)).expect("within package");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("h"), "{dump}");
        let _ = fs::remove_dir_all(&root);
    }

    /// K06.01: discover lock + default cache from workspace ancestors.
    #[test]
    fn link_module_path_discovers_workspace_lock() {
        let root = std::env::temp_dir().join(format!(
            "draconic-link-pkg-discover-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let src_dir = root.join("src");
        fs::create_dir_all(&src_dir).unwrap();
        let oid = "fedcba9876543210fedcba9876543210fedcba98";
        let module_path = "github.com/acme/lib";
        let cache = draconic_pkg::ModuleCache::new(draconic_pkg::default_cache_root(&root));
        let pkg_dir = cache.entry_dir(module_path, oid).unwrap();
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(pkg_dir.join("mod.drac"), "export let answer = 42;\n").unwrap();
        let hash = draconic_pkg::content_hash_tree(&pkg_dir).unwrap();
        fs::write(pkg_dir.join(".draconic-checkout-oid"), format!("{oid}\n")).unwrap();

        let entry = draconic_pkg::LockEntry::new(
            module_path,
            "1.2.3",
            "https://github.com/acme/lib.git",
            oid,
            hash,
        )
        .unwrap();
        let mut packages = std::collections::BTreeMap::new();
        packages.insert(module_path.to_string(), entry);
        let lock = draconic_pkg::LockFile {
            version: 1,
            packages,
        };
        fs::write(
            root.join(draconic_pkg::LOCK_FILE),
            draconic_pkg::write_lock(&lock),
        )
        .unwrap();

        let main = src_dir.join("main.drac");
        fs::write(
            &main,
            "import { answer } from \"github.com/acme/lib\";\nlet a = answer;\n",
        )
        .unwrap();

        let program = link_entry(&main).expect("discover lock");
        let dump = draconic_ast::dump_program(&program);
        assert!(dump.contains("a"), "{dump}");
        assert!(
            dump.contains("42") || dump.contains("answer") || dump.contains("__m"),
            "{dump}"
        );
        let _ = fs::remove_dir_all(&root);
    }
}
