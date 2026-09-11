//! Host callee names classify through the Check catalog, not per-file fingerprints.

use draconic_check::HostApiEntry;
use draconic_ir::{Expr, Module};

pub(crate) fn catalog_callee(expr: &Expr) -> Option<&'static HostApiEntry> {
    catalog_callee_in(expr, None)
}

pub(crate) fn catalog_callee_in_module<'a>(
    expr: &'a Expr,
    module: &'a Module,
) -> Option<&'static HostApiEntry> {
    catalog_callee_in(expr, Some(module))
}

pub(crate) fn is_named_callee(expr: &Expr, want: &str) -> bool {
    match catalog_callee(expr) {
        Some(entry) => entry.name == want,
        None => ident_eq_in(expr, want, None) && !draconic_check::is_host_api(want),
    }
}

pub(crate) fn is_named_callee_in(expr: &Expr, want: &str, module: Option<&Module>) -> bool {
    match catalog_callee_in(expr, module) {
        Some(entry) => entry.name == want,
        None => ident_eq_in(expr, want, module) && !draconic_check::is_host_api(want),
    }
}

fn catalog_callee_in(expr: &Expr, module: Option<&Module>) -> Option<&'static HostApiEntry> {
    match expr {
        Expr::IdentName { name, .. } => draconic_check::lookup_host_api(name),
        Expr::Local { id, .. } => {
            let module = module?;
            let local = module.locals.iter().find(|l| l.id == *id)?;
            draconic_check::lookup_host_api(&local.name)
        }
        _ => None,
    }
}

fn ident_eq_in(expr: &Expr, want: &str, module: Option<&Module>) -> bool {
    match expr {
        Expr::IdentName { name, .. } => name == want,
        Expr::Local { id, .. } => module
            .and_then(|m| m.locals.iter().find(|l| l.id == *id))
            .is_some_and(|l| l.name == want),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;
    use draconic_ir::Stmt;

    fn ident(name: &str) -> Expr {
        Expr::IdentName {
            name: name.to_string(),
            ty: draconic_check::Type::Any,
        }
    }

    #[test]
    fn host_call_names_classify_through_catalog() {
        assert!(draconic_check::lookup_host_api("notAHostApi").is_none());
        for api in draconic_check::host_apis() {
            let expr = ident(api.name);
            let entry = catalog_callee(&expr)
                .unwrap_or_else(|| panic!("{} must be a catalog row", api.name));
            assert_eq!(entry.name, api.name);
            assert!(
                is_named_callee(&expr, api.name),
                "{} must classify through lookup_host_api",
                api.name
            );
        }
        let fake = ident("notAHostApi");
        assert!(catalog_callee(&fake).is_none());
        assert!(!is_named_callee(&fake, "readFileText"));
        assert!(!is_named_callee(&ident("cwd"), "readFileText"));
    }

    #[test]
    fn non_host_ident_still_matches_by_name() {
        assert!(!draconic_check::is_host_api("Uint8Array"));
        assert!(is_named_callee(&ident("Uint8Array"), "Uint8Array"));
    }

    #[test]
    fn leftover_walk_host_fingerprint_adapters_are_gone() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut hits = Vec::new();
        collect_rs(&root, &mut hits);
        let needle = concat!("fn ", "walk_host_");
        let leftover: Vec<String> = hits
            .into_iter()
            .filter_map(|p| {
                let text = std::fs::read_to_string(&p).ok()?;
                if text.contains(needle) {
                    p.file_name()
                        .and_then(|n| n.to_str())
                        .map(|n| n.to_string())
                } else {
                    None
                }
            })
            .collect();
        assert!(
            leftover.is_empty(),
            "fingerprint adapters still present: {}",
            leftover.join(",")
        );
    }

    fn collect_rs(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let entries = std::fs::read_dir(dir).expect("read llvm src");
        for entry in entries {
            let path = entry.expect("dirent").path();
            if path.is_dir() {
                collect_rs(&path, out);
            } else if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                out.push(path);
            }
        }
    }

    #[test]
    fn compiled_host_call_resolves_through_catalog() {
        let m = compile_source(r#"let t = readFileText("hello.txt");"#).expect("compile");
        let mut found = false;
        for stmt in &m.body {
            let Stmt::Declare {
                init: Some(expr), ..
            } = stmt
            else {
                continue;
            };
            let Expr::Call { callee, .. } = expr else {
                continue;
            };
            let entry = catalog_callee(callee).expect("readFileText must be a catalog row");
            assert_eq!(entry.name, "readFileText");
            assert!(entry.note.starts_with("H04"));
            found = true;
        }
        assert!(found, "expected a readFileText call in lowered IR");
    }
}
