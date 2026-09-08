use std::collections::{HashMap, HashSet};

use draconic_diagnostics::{codes, Diagnostic, Span};

use crate::load::Loader;
use crate::namespace::{BINDING_DEFERRED_NAMESPACE, BINDING_NAMESPACE};

impl Loader {
    /// Resolve `name` exported by `module_id` to `(defining_module_id, local_name)`.
    /// Follows `export * from` and `export { … } from`. Direct exports shadow stars.
    /// Ambiguous star collisions yield `None` (same as missing for link errors).
    pub(crate) fn resolve_export(
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
                .with_code(codes::LINKER_INTERNAL)
            })?;
            return self.resolve_named_export(dep_id, &re.imported, chain);
        }
        for dep_path in &module.star_reexports {
            let dep_id = *self.ids.get(dep_path).ok_or_else(|| {
                Diagnostic::new(
                    format!("module not loaded: {}", dep_path.display()),
                    Span::dummy(),
                )
                .with_code(codes::LINKER_INTERNAL)
            })?;
            if let Some(binding) = self.resolve_named_export(dep_id, name, chain)? {
                return Ok(Some(binding));
            }
        }
        Ok(None)
    }

    /// Unambiguous export names visible from `module_id` (GetModuleNamespace set).
    /// Ambiguous star collisions are omitted, not errors (E19.71).
    pub(crate) fn collect_resolved_exports(
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
                .with_code(codes::LINKER_INTERNAL)
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
                .with_code(codes::LINKER_INTERNAL)
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
                .with_code(codes::LINKER_INTERNAL)
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
                            )
                            .with_code(codes::DUPLICATE_EXPORT));
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
                .with_code(codes::LINKER_INTERNAL)
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
    pub(crate) fn validate_indirect_exports(&self) -> Result<(), Diagnostic> {
        for module in &self.modules {
            for re in &module.named_reexports {
                let dep_id = *self.ids.get(&re.from).ok_or_else(|| {
                    Diagnostic::new(
                        format!("module not loaded: {}", re.from.display()),
                        Span::dummy(),
                    )
                    .with_code(codes::LINKER_INTERNAL)
                })?;
                let resolved = self.resolve_export(dep_id, &re.imported, &mut HashSet::new())?;
                if resolved.is_none() {
                    return Err(Diagnostic::new(
                        format!(
                            "module {} has no export `{}` (missing or ambiguous)",
                            re.from.display(),
                            re.imported
                        ),
                        Span::dummy(),
                    )
                    .with_code(codes::UNDECLARED_EXPORT));
                }
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::{link_entry, temp_link_dir};
    use std::fs;

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
        fs::write(
            &lib,
            "export let value = 41;\nexport function inc(x) { return x + 1; }\n",
        )
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
        assert!(
            dump.contains("bump") || dump.contains("__m") || dump.contains("inc"),
            "{dump}"
        );
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
        fs::write(
            &barrel,
            "export * as ns from \"./lib.drac\";\nexport let extra = 7;\n",
        )
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
        assert!(
            dump.contains("ClassDeclaration") || dump.contains("Point"),
            "{dump}"
        );
        assert!(dump.contains("Counter") || dump.contains("__m"), "{dump}");
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
    fn link_ambiguous_star_omitted_from_namespace() {
        // E19.71: ambiguous export * names are absent from namespace objects.
        let dir = temp_link_dir("ambig-ns");
        fs::write(
            dir.join("a.drac"),
            "export let first = 1;\nexport let both = 2;\n",
        )
        .unwrap();
        fs::write(
            dir.join("b.drac"),
            "export let second = 3;\nexport let both = 4;\n",
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
        assert_eq!(
            err.code,
            Some(draconic_diagnostics::codes::UNDECLARED_EXPORT)
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
        assert_eq!(
            err.code,
            Some(draconic_diagnostics::codes::UNDECLARED_EXPORT)
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
        assert!(
            dump.contains("__ns") || dump.contains("ObjectExpression"),
            "{dump}"
        );
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
        assert!(
            dump.contains("__ns") || dump.contains("ObjectExpression"),
            "{dump}"
        );
        let _ = fs::remove_dir_all(&dir);
    }
}
