//! Frontend: compile Draconic source (or an entry path) to IR.
//!
//! Owns Script vs Module (link) policy, then check → lower. Callers should not
//! re-assemble parser/check/ir stages or gate modules on source substrings.

use std::path::Path;

use draconic_ast::{Program, Stmt};
use draconic_check::check;
use draconic_diagnostics::Diagnostic;
use draconic_ir::lower;
use draconic_linker::link_entry;
use draconic_parser::parse;

pub use draconic_check::CheckedProgram;
pub use draconic_ir::Module;

/// Compile `source` as a Script (no filesystem link graph).
///
/// Suitable for Embed and single-buffer inputs. Relative imports are not resolved.
pub fn compile_source(source: &str) -> Result<Module, Diagnostic> {
    let checked = check_source(source)?;
    Ok(lower(&checked))
}

/// Compile a filesystem entry: Script parse, or Module link when the entry has
/// import/export syntax (parse-driven, not a source substring heuristic).
pub fn compile_path(entry: &Path) -> Result<Module, Diagnostic> {
    let checked = check_path(entry)?;
    Ok(lower(&checked))
}

/// Parse + check `source` as a Script without lowering.
pub fn check_source(source: &str) -> Result<CheckedProgram, Diagnostic> {
    let program = parse(source)?;
    check(program)
}

/// Parse or link `entry`, then check, without lowering.
pub fn check_path(entry: &Path) -> Result<CheckedProgram, Diagnostic> {
    let program = load_program(entry)?;
    check(program)
}

fn load_program(entry: &Path) -> Result<Program, Diagnostic> {
    let source = std::fs::read_to_string(entry).map_err(|e| {
        Diagnostic::new(
            format!("read {}: {e}", entry.display()),
            draconic_diagnostics::Span::dummy(),
        )
    })?;
    let program = parse(&source)?;
    if program_has_module_syntax(&program) {
        link_entry(entry)
    } else {
        Ok(program)
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
    fn compile_path_script_skips_link() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-frontend-script-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("main.drac");
        std::fs::write(&path, "let x = 1;\n").unwrap();
        let module = compile_path(&path).expect("compile path script");
        assert!(module.locals.iter().any(|l| l.name == "x") || !module.body.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn compile_path_module_links_import() {
        let dir = std::env::temp_dir().join(format!(
            "draconic-frontend-mod-{}",
            std::process::id()
        ));
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
}
