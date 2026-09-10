//! H05 / H05.03–H05.05: timers via Runtime timer + job queue ABI.
//!
//! Supported subset for conformance:
//! - top-level number/bool/string locals
//! - `nowMs()` / `Date.now()` / `monotonicMs()` wall and monotonic clocks
//! - `setTimeout` / `setInterval(function () { … }, delay)` with number assigns
//! - nested `setTimeout` / `setInterval` inside timer callbacks
//! - `clearTimeout(id)` / `clearInterval(id)`
//! - `typeof` on timer host APIs and clock calls
//! - comparison `id > 0` / `ticks >= n` / clock range checks
//! - `if (test) { … }` in timer callbacks (clear after N ticks)
//!
//! End of main: `job_drain` (promotes due timers, sleeps until future timers
//! are due — H05.05 — runs callbacks), then print observation locals
//! (numbers, strings, bools) in declaration order.

use std::collections::{HashMap, HashSet};

use draconic_ast::{BinaryOp, UnaryOp};
use draconic_diagnostics::{Diagnostic, Span};
use draconic_ir::{Arg, AssignTarget, Expr, IrType as Type, Local, LocalId, Module, Stmt};
use draconic_runtime::abi::{
    llvm_declares, GC_INIT, HOST_MONOTONIC_MS, HOST_NOW_MS, HOST_TIMER_DECLARES, JOB_DRAIN,
    PRINT_BOOL, PRINT_I64, PRINT_STR, TIMER_CLEAR, TIMER_SET, TIMER_SET_INTERVAL,
};

mod classify;
mod emit;

use classify::try_classify;

pub(crate) fn is_host_timer_module(module: &Module) -> bool {
    match try_classify(module) {
        Ok(info) => info.uses_timer,
        Err(_) => false,
    }
}

pub(crate) fn emit_host_timers(module: &Module) -> Result<String, Diagnostic> {
    let info = try_classify(module).map_err(diag)?;
    if !info.uses_timer {
        return Err(diag("internal: not a host_timer module"));
    }
    let mut em = Emitter::new(module, info);
    em.emit_module()?;
    Ok(em.finish())
}

pub(crate) fn walk_host_timers(module: &Module) -> Option<Result<String, Diagnostic>> {
    if !is_host_timer_module(module) {
        return None;
    }
    Some(emit_host_timers(module))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SlotKind {
    Number,
    Bool,
    String,
}

struct ModuleInfo {
    uses_timer: bool,
    user_locals: Vec<(LocalId, SlotKind)>,
}

fn diag(msg: impl Into<String>) -> Diagnostic {
    Diagnostic::new(msg, Span::dummy())
}

struct Emitter<'a> {
    module: &'a Module,
    info: ModuleInfo,
    out: String,
    body: String,
    helpers: String,
    tmp: u32,
    next_fn: u32,
    allocas: HashMap<LocalId, String>,
    str_globals: Vec<(String, String)>,
    /// Locals assigned inside the callback currently being emitted.
    reaction_captures: Vec<LocalId>,
    in_callback: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_frontend::compile_source;

    fn ir_of(src: &str) -> Module {
        compile_source(src).expect("compile")
    }

    #[test]
    fn classifies_set_timeout_fixture() {
        let m = ir_of(
            r#"
            let fired = 0;
            setTimeout(function () { fired = 1; }, 0);
            let t = typeof setTimeout;
            "#,
        );
        assert!(is_host_timer_module(&m));
        let ir = emit_host_timers(&m).expect("emit");
        assert!(ir.contains("draconic_rt_timer_set"), "{ir}");
        assert!(ir.contains("draconic_rt_job_drain"), "{ir}");
    }

    #[test]
    fn classifies_clear_timeout() {
        let m = ir_of(
            r#"
            let cancelled = 0;
            let id = setTimeout(function () { cancelled = 1; }, 0);
            clearTimeout(id);
            "#,
        );
        assert!(is_host_timer_module(&m));
        let ir = emit_host_timers(&m).expect("emit");
        assert!(ir.contains("draconic_rt_timer_clear"), "{ir}");
    }

    #[test]
    fn classifies_set_interval_fixture() {
        let m = ir_of(
            r#"
            let ticks = 0;
            let id = setInterval(function () {
              ticks = ticks + 1;
              if (ticks >= 3) {
                clearInterval(id);
              }
            }, 0);
            let t = typeof setInterval;
            "#,
        );
        assert!(is_host_timer_module(&m));
        let ir = emit_host_timers(&m).expect("emit");
        assert!(ir.contains("draconic_rt_timer_set_interval"), "{ir}");
        assert!(ir.contains("draconic_rt_timer_clear"), "{ir}");
        assert!(ir.contains("draconic_rt_job_drain"), "{ir}");
    }

    #[test]
    fn classifies_nonzero_delay_timer_wait() {
        let m = ir_of(
            r#"
            let fired = 0;
            setTimeout(function () { fired = 1; }, 40);
            let t = typeof setTimeout;
            "#,
        );
        assert!(is_host_timer_module(&m));
        let ir = emit_host_timers(&m).expect("emit");
        assert!(ir.contains("draconic_rt_timer_set"), "{ir}");
        assert!(ir.contains("draconic_rt_job_drain"), "{ir}");
    }

    #[test]
    fn classifies_combined_time_surface() {
        let m = ir_of(
            r#"
            let now_ok = nowMs() > 1600000000000;
            let date_ok = Date.now() > 1600000000000;
            let mono_ok = monotonicMs() >= 0;
            let t_now = typeof nowMs();
            let fired = 0;
            setTimeout(function () { fired = 1; }, 20);
            let t = typeof setTimeout;
            "#,
        );
        match try_classify(&m) {
            Ok(info) => assert!(info.uses_timer, "combined surface must use timers"),
            Err(e) => panic!("classify failed: {e}"),
        }
        let ir = emit_host_timers(&m).expect("emit");
        assert!(ir.contains("draconic_rt_host_now_ms"), "{ir}");
        assert!(ir.contains("draconic_rt_host_monotonic_ms"), "{ir}");
        assert!(ir.contains("draconic_rt_timer_set"), "{ir}");
        assert!(ir.contains("draconic_rt_job_drain"), "{ir}");
    }
}
