//! Native `globalThis.console.log` lowering (portable print; not host `stdoutWrite`).
//!
//! Locks `toolchain.cli:build-targets` for Programs js already runs. Documented hello
//! using `globalThis.console.log` is portable, not `language.dual-worlds:js-only-hard-error-on-llvm`.

use std::collections::HashMap;

use draconic_ast::JsString;
use draconic_ir::{Arg, Expr, Local, LocalId, Module, Stmt};

pub(crate) fn module_has_console_log(module: &Module) -> bool {
    let by_id: HashMap<LocalId, &Local> = module.locals.iter().map(|l| (l.id, l)).collect();
    module.body.iter().any(|s| stmt_has_console_log(s, &by_id))
}

pub(crate) fn is_global_this_console(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    let Expr::Member {
        object,
        property,
        optional: false,
        ..
    } = expr
    else {
        return false;
    };
    ident_is_name(object, "globalThis", by_id) && static_key(property).as_deref() == Some("console")
}

pub(crate) fn console_log_string_arg<'a>(
    expr: &'a Expr,
    by_id: &HashMap<LocalId, &Local>,
) -> Option<&'a JsString> {
    let Expr::Call {
        callee,
        args,
        optional: false,
        ..
    } = expr
    else {
        return None;
    };
    let Expr::Member {
        object,
        property,
        optional: false,
        ..
    } = callee.as_ref()
    else {
        return None;
    };
    if !is_console_receiver(object, by_id) {
        return None;
    }
    if static_key(property).as_deref() != Some("log") {
        return None;
    }
    if args.len() != 1 {
        return None;
    }
    match &args[0] {
        Arg::Expr(Expr::String { value, .. }) => Some(value),
        _ => None,
    }
}

fn ident_is_name(expr: &Expr, name: &str, by_id: &HashMap<LocalId, &Local>) -> bool {
    match expr {
        Expr::IdentName { name: n, .. } => n == name,
        Expr::Local { id, .. } => by_id.get(id).is_some_and(|l| l.name == name),
        _ => false,
    }
}

fn static_key(expr: &Expr) -> Option<String> {
    match expr {
        Expr::String { value, .. } => Some(value.to_string_lossy()),
        Expr::IdentName { name, .. } => Some(name.clone()),
        _ => None,
    }
}

fn is_console_receiver(expr: &Expr, by_id: &HashMap<LocalId, &Local>) -> bool {
    ident_is_name(expr, "console", by_id) || is_global_this_console(expr, by_id)
}

fn stmt_has_console_log(stmt: &Stmt, by_id: &HashMap<LocalId, &Local>) -> bool {
    match stmt {
        Stmt::Expr { expr } => console_log_string_arg(expr, by_id).is_some(),
        Stmt::Declare { init: Some(e), .. } => console_log_string_arg(e, by_id).is_some(),
        Stmt::Block { body } => body.iter().any(|s| stmt_has_console_log(s, by_id)),
        Stmt::If {
            consequent,
            alternate,
            ..
        } => {
            stmt_has_console_log(consequent, by_id)
                || alternate
                    .as_ref()
                    .is_some_and(|a| stmt_has_console_log(a, by_id))
        }
        Stmt::While { body, .. } | Stmt::DoWhile { body, .. } | Stmt::Labeled { body, .. } => {
            stmt_has_console_log(body, by_id)
        }
        Stmt::For { body, .. } | Stmt::ForIn { body, .. } | Stmt::ForOf { body, .. } => {
            stmt_has_console_log(body, by_id)
        }
        Stmt::Function { body, .. } => body.iter().any(|s| stmt_has_console_log(s, by_id)),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    use crate::{build_native_binary, emit_llvm_ir, work_dir};
    use draconic_frontend::compile_source;

    fn module_of(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    fn run_native(src: &str, dir_name: &str, bin_name: &str) -> String {
        let m = module_of(src);
        assert!(module_has_console_log(&m), "IR must include console.log");
        let ir = emit_llvm_ir(&m).expect("emit");
        assert!(
            !ir.contains("draconic_rt_hello"),
            "console.log must not use hello stub:\n{ir}"
        );
        assert!(
            ir.contains("draconic_rt_print_str"),
            "console.log must print via stdout:\n{ir}"
        );
        let dir = work_dir(dir_name).expect("workdir");
        let bin = dir.join(bin_name);
        build_native_binary(&ir, &bin).expect("build");
        let output = Command::new(&bin).output().expect("run");
        assert!(
            output.status.success(),
            "exit {:?}\nstderr={}\nir=\n{ir}",
            output.status,
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8_lossy(&output.stdout).into_owned()
    }

    #[test]
    fn native_console_log_prints() {
        let stdout = run_native(
            "let console = globalThis.console;\nconsole.log(\"console-ok\");\n",
            "draconic-llvm-console-log",
            "console_ok",
        );
        assert!(stdout.contains("console-ok"), "stdout={stdout:?}");
    }

    #[test]
    fn native_console_log_with_function_prints() {
        let stdout = run_native(
            "function f() { return 1; }\nlet x = f();\nlet console = globalThis.console;\nconsole.log(\"fn-ok\");\n",
            "draconic-llvm-console-fn",
            "fn_ok",
        );
        assert!(stdout.contains("fn-ok"), "stdout={stdout:?}");
    }

    #[test]
    fn native_console_log_with_loop_prints() {
        let stdout = run_native(
            "let i = 0;\nwhile (i < 100) { i = i + 1; }\nlet console = globalThis.console;\nconsole.log(\"loop-ok\");\n",
            "draconic-llvm-console-loop",
            "loop_ok",
        );
        assert!(stdout.contains("loop-ok"), "stdout={stdout:?}");
    }
}
