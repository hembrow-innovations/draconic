//! Frontend: compile Draconic source (or an entry path) to IR.
//!
//! Owns Script vs Module (link) policy, then check → lower. Callers should not
//! re-assemble parser/check/ir stages or gate modules on source substrings.

use std::path::Path;

use draconic_ast::{Program, Stmt};
use draconic_check::{check, check_for_target, check_module, check_module_for_target};
use draconic_diagnostics::Diagnostic;
use draconic_ir::lower;
use draconic_linker::{link_entry, link_entry_with_named_exports};
use draconic_parser::{parse, parse_module};

pub use draconic_check::{CheckedProgram, CompileTarget};
pub use draconic_ir::{Module, NamedExport};

/// Compile `source` as a Script (no filesystem link graph).
///
/// Suitable for Embed and single-buffer inputs. Relative imports are not resolved.
/// Top-level `await` is rejected (Script goal). Does not retry Module; use
/// [`compile_source_module`] for a Module-goal buffer.
pub fn compile_source(source: &str) -> Result<Module, Diagnostic> {
    let checked = check_source(source)?;
    Ok(lower(&checked))
}

/// Compile `source` under the Module goal (E19.28): top-level `await` allowed.
///
/// Relative static imports are not resolved; import/export still needs
/// [`compile_path`] for a link graph.
pub fn compile_source_module(source: &str) -> Result<Module, Diagnostic> {
    let checked = check_source_module(source)?;
    Ok(lower(&checked))
}

/// Compile a filesystem entry: Script parse, or Module link when the entry has
/// import/export syntax (parse-driven, not a source substring heuristic).
/// Linked entries use the Module goal (top-level `await` allowed).
pub fn compile_path(entry: &Path) -> Result<Module, Diagnostic> {
    let loaded = load_program(entry)?;
    compile_loaded(loaded, None)
}

/// Compile a filesystem entry after checking host and FFI policy for `target`.
pub fn compile_path_for_target(entry: &Path, target: CompileTarget) -> Result<Module, Diagnostic> {
    let loaded = load_program(entry)?;
    compile_loaded(loaded, Some(target))
}

/// Always link `entry` as a Module graph, then check and lower.
///
/// [`compile_path`] links only when the entry AST has static import/export.
/// Dynamic-only entries that load `import defer` / `import.defer` still need
/// flatten — hosts that cannot parse that syntax (Node) must not see it.
pub fn compile_path_linked(entry: &Path) -> Result<Module, Diagnostic> {
    let (program, named_exports) = link_entry_with_named_exports(entry)?;
    compile_loaded(
        LoadedProgram {
            program,
            module_goal: true,
            named_exports,
        },
        None,
    )
}

/// Parse `source` Script-first, then Module. No filesystem link.
///
/// Fmt and single-buffer tools use this instead of copying the retry. Both
/// goals failing keeps the Script diagnostic.
pub fn parse_source(source: &str) -> Result<Program, Diagnostic> {
    match parse(source) {
        Ok(program) => Ok(program),
        Err(script_err) => match parse_module(source) {
            Ok(program) => Ok(program),
            Err(_) => Err(script_err),
        },
    }
}

/// Parse + check `source` as a Script without lowering.
///
/// Does not retry Module after a Script parse or check failure. [`parse_source`]
/// retries Module for fmt and dump tools; Module string check is
/// [`check_source_module`].
pub fn check_source(source: &str) -> Result<CheckedProgram, Diagnostic> {
    let program = parse(source)?;
    check(program)
}

/// Parse + check `source` as a Module without lowering (E19.28).
///
/// Top-level `await` is allowed. Import/export still needs [`check_path`].
pub fn check_source_module(source: &str) -> Result<CheckedProgram, Diagnostic> {
    let program = parse_module(source)?;
    check_module(program)
}

/// Parse or link `entry`, then check, without lowering.
pub fn check_path(entry: &Path) -> Result<CheckedProgram, Diagnostic> {
    let loaded = load_program(entry)?;
    check_loaded(loaded.program, loaded.module_goal, None)
}

/// Parse or link `entry`, then check host and FFI policy for `target`.
pub fn check_path_for_target(
    entry: &Path,
    target: CompileTarget,
) -> Result<CheckedProgram, Diagnostic> {
    let loaded = load_program(entry)?;
    check_loaded(loaded.program, loaded.module_goal, Some(target))
}

/// Always `link_entry` on `entry`, then check as Module, without lowering.
pub fn check_path_linked(entry: &Path) -> Result<CheckedProgram, Diagnostic> {
    check_loaded(link_entry(entry)?, true, None)
}

struct LoadedProgram {
    program: Program,
    module_goal: bool,
    named_exports: Vec<(String, String)>,
}

fn compile_loaded(
    loaded: LoadedProgram,
    target: Option<CompileTarget>,
) -> Result<Module, Diagnostic> {
    let checked = check_loaded(loaded.program, loaded.module_goal, target)?;
    let mut module = lower(&checked);
    module.named_exports = loaded
        .named_exports
        .into_iter()
        .map(|(public_name, local_name)| NamedExport {
            public_name,
            local_name,
        })
        .collect();
    Ok(module)
}

fn check_loaded(
    program: Program,
    module_goal: bool,
    target: Option<CompileTarget>,
) -> Result<CheckedProgram, Diagnostic> {
    match (module_goal, target) {
        (true, Some(target)) => check_module_for_target(program, target),
        (true, None) => check_module(program),
        (false, Some(target)) => check_for_target(program, target),
        (false, None) => check(program),
    }
}

fn load_program(entry: &Path) -> Result<LoadedProgram, Diagnostic> {
    let source = std::fs::read_to_string(entry).map_err(|e| {
        Diagnostic::new(
            format!("read {}: {e}", entry.display()),
            draconic_diagnostics::Span::dummy(),
        )
    })?;
    // Script-first detection. Export + top-level await fails Script parse once
    // `await` is IdentifierReference outside Module (E19.52) — retry Module.
    match parse(&source) {
        Ok(program) if program_has_module_syntax(&program) => {
            let (program, named_exports) = link_entry_with_named_exports(entry)?;
            Ok(LoadedProgram {
                program,
                module_goal: true,
                named_exports,
            })
        }
        Ok(program) => Ok(LoadedProgram {
            program,
            module_goal: false,
            named_exports: Vec::new(),
        }),
        Err(script_err) => match parse_module(&source) {
            Ok(program) if program_has_module_syntax(&program) => {
                let (program, named_exports) = link_entry_with_named_exports(entry)?;
                Ok(LoadedProgram {
                    program,
                    module_goal: true,
                    named_exports,
                })
            }
            _ => Err(script_err),
        },
    }
}

/// True when the program body contains ESM import/export statements.
fn program_has_module_syntax(program: &Program) -> bool {
    program.body.iter().any(stmt_is_module_syntax)
}

fn stmt_is_module_syntax(stmt: &Stmt) -> bool {
    matches!(
        stmt,
        Stmt::ImportDeclaration { .. }
            | Stmt::ExportNamedDeclaration { .. }
            | Stmt::ExportDefaultDeclaration { .. }
            | Stmt::ExportAllDeclaration { .. }
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn compile_source_script() {
        let module = compile_source("let x = 1;").expect("compile");
        assert!(!module.body.is_empty() || !module.locals.is_empty());
    }

    #[test]
    fn check_source_script() {
        check_source("let x = 1;").expect("script check");
    }

    #[test]
    fn check_source_and_compile_source_reject_export() {
        let export = "export let x = 1;";
        let check_err = check_source(export).expect_err("script check rejects export");
        assert!(
            !check_err.message.is_empty(),
            "script check must diagnostic, got empty message"
        );
        let compile_err = compile_source(export).expect_err("script compile rejects export");
        assert!(
            !compile_err.message.is_empty(),
            "script compile must diagnostic, got empty message"
        );
    }

    #[test]
    fn check_source_does_not_retry_module_on_top_level_await() {
        let tla = "let x = await 1;";
        let err = check_source(tla).expect_err("script check does not retry Module");
        assert!(
            !err.message.is_empty(),
            "script check must diagnostic, got empty message"
        );
        check_source_module(tla).expect("module check accepts TLA");
        compile_source_module(tla).expect("module compile accepts TLA");
    }

    #[test]
    fn check_source_module_does_not_link_export() {
        let err = check_source_module("export let x = 1;")
            .expect_err("module string check has no link graph");
        assert!(
            err.message.contains("import/export must be linked"),
            "got {}",
            err.message
        );
        let compile_err = compile_source_module("export let x = 1;")
            .expect_err("module string compile has no link graph");
        assert!(
            compile_err.message.contains("import/export must be linked"),
            "got {}",
            compile_err.message
        );
    }

    #[test]
    fn compile_path_script_skips_link() {
        let dir =
            std::env::temp_dir().join(format!("draconic-frontend-script-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("main.drac");
        std::fs::write(&path, "let x = 1;\n").unwrap();
        let module = compile_path(&path).expect("compile path script");
        assert!(module.locals.iter().any(|l| l.name == "x") || !module.body.is_empty());
        assert!(module.named_exports.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compile_path_module_links_import() {
        let dir =
            std::env::temp_dir().join(format!("draconic-frontend-mod-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("dep.drac"), "export let v = 2;\n").unwrap();
        let main = dir.join("main.drac");
        let mut f = std::fs::File::create(&main).unwrap();
        writeln!(f, "import {{ v }} from \"./dep.drac\";").unwrap();
        writeln!(f, "let x = v;").unwrap();
        drop(f);
        let module = compile_path(&main).expect("compile path module");
        assert!(!module.body.is_empty() || !module.locals.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn module_syntax_detects_export_not_comment_text() {
        let program = parse("let import_name = 1;").unwrap();
        assert!(!program_has_module_syntax(&program));
        let program = parse("export let x = 1;").unwrap();
        assert!(program_has_module_syntax(&program));
    }

    #[test]
    fn compile_source_module_allows_top_level_await() {
        // E19.28: Module goal accepts top-level await; Script rejects it.
        let module = compile_source_module("let x = await 1;\n").expect("module TLA");
        assert!(!module.body.is_empty() || !module.locals.is_empty());
        // E19.52: Script [~Await] treats bare `await` as IdentifierReference, so
        // `await 1` is a syntax error (not AwaitExpression) — any diagnostic is fine.
        let err = compile_source("let x = await 1;\n").expect_err("script TLA");
        assert!(
            !err.message.is_empty(),
            "script TLA must diagnostic, got empty message"
        );
    }

    #[test]
    fn parse_source_script() {
        let program = parse_source("let x = 1;").expect("script");
        assert!(!program_has_module_syntax(&program));
    }

    #[test]
    fn parse_source_retries_module_on_export() {
        let program = parse_source("export let x = 1;").expect("module retry");
        assert!(program_has_module_syntax(&program));
    }

    #[test]
    fn parse_source_keeps_script_error_when_both_fail() {
        let err = parse_source("let = ;").expect_err("both goals fail");
        assert!(
            !err.message.is_empty(),
            "script diagnostic must be kept, got empty message"
        );
    }

    fn named_export_local<'a>(module: &'a Module, public: &str) -> &'a str {
        module
            .named_exports
            .iter()
            .find(|e| e.public_name == public)
            .map(|e| e.local_name.as_str())
            .unwrap_or_else(|| panic!("missing public export `{public}`"))
    }

    #[test]
    fn entry_named_exports_on_ir() {
        let (dir, path) = write_temp_drac("view-export", "export const view = \"view\";\n");
        let module = compile_path(&path).expect("compile named-export entry");
        let local = named_export_local(&module, "view");
        assert_eq!(local, "view");
        assert!(
            module.locals.iter().any(|l| l.name == local),
            "IR local `{local}` missing"
        );
        println!("view-export-ok");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reexport_entry_export_names_on_ir() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-frontend-view-reexport-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("dep.drac"), "export const view = \"view\";\n").unwrap();
        let main = dir.join("lib.drac");
        std::fs::write(&main, "export { view } from \"./dep.drac\";\n").unwrap();
        let module = compile_path(&main).expect("compile re-export entry");
        let local = named_export_local(&module, "view");
        assert!(
            !local.is_empty(),
            "public name `view` must bind a flattened local"
        );
        assert!(
            module.locals.iter().any(|l| l.name == local),
            "flattened local `{local}` missing"
        );
        println!("view-reexport-ok");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn entry_export_alias_uses_public_name() {
        let (dir, path) = write_temp_drac(
            "view-alias",
            "const local = \"view\";\nexport { local as publicName };\n",
        );
        let module = compile_path(&path).expect("compile aliased named export");
        let local = named_export_local(&module, "publicName");
        assert_eq!(local, "local");
        assert!(
            module
                .named_exports
                .iter()
                .all(|e| e.public_name != "local"),
            "alias must use publicName as the public key"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compile_path_module_allows_top_level_await_export() {
        let dir =
            std::env::temp_dir().join(format!("draconic-frontend-tla-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("main.drac");
        std::fs::write(&path, "export let x = await 2;\n").unwrap();
        let module = compile_path(&path).expect("path module TLA");
        assert!(!module.body.is_empty() || !module.locals.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn write_temp_drac(label: &str, source: &str) -> (std::path::PathBuf, std::path::PathBuf) {
        let dir =
            std::env::temp_dir().join(format!("draconic-frontend-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("main.drac");
        std::fs::write(&path, source).unwrap();
        (dir, path)
    }

    #[test]
    fn check_path_without_target_allows_native_only_host() {
        let (dir, path) = write_temp_drac("untargeted-host", "tlsClientWrap;\n");
        check_path(&path).expect("untargeted check keeps today's host policy");
        compile_path(&path).expect("untargeted compile keeps today's host policy");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn check_path_for_target_js_rejects_native_only_host() {
        let (dir, path) = write_temp_drac("js-host", "tlsClientWrap;\n");
        let err = check_path_for_target(&path, CompileTarget::Js)
            .expect_err("js target must diagnostic native-only host use");
        assert_eq!(
            err.code,
            Some(draconic_diagnostics::codes::HOST_API_UNSUPPORTED)
        );
        assert!(
            err.message.contains("unsupported on js") && err.message.contains("native-only"),
            "got {}",
            err.message
        );
        let compile_err = compile_path_for_target(&path, CompileTarget::Js)
            .expect_err("js compile must diagnostic native-only host use");
        assert_eq!(
            compile_err.code,
            Some(draconic_diagnostics::codes::HOST_API_UNSUPPORTED)
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compile_path_skips_link_on_dynamic_import_defer_only() {
        let (dir, path) = write_temp_drac(
            "defer-dyn-script",
            "import.defer(\"./dep.drac\").then(function (ns) { let v = ns.x; });\n",
        );
        std::fs::write(dir.join("dep.drac"), "export let x = 1;\n").unwrap();
        compile_path(&path).expect("script-goal compile without flatten");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compile_path_linked_flattens_import_defer_without_static_import() {
        let (dir, path) = write_temp_drac(
            "defer-force-link",
            "import.defer(\"./dep.drac\").then(function (ns) { let v = ns.x; });\n",
        );
        std::fs::write(dir.join("dep.drac"), "export let x = 1;\n").unwrap();
        let module = compile_path_linked(&path).expect("force-link import.defer");
        assert!(!module.body.is_empty() || !module.locals.is_empty());
        check_path_linked(&path).expect("force-link check");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
