//! Binder (scopes + symbol resolution) and Checker (TypeScript-inspired).
//! Binder: ROADMAP B04. Checker: ROADMAP B05. Host API registry: H00.01.

mod binder;
mod checker;
mod early;
mod host_api;
mod symbols;
mod syntax;
mod types;

use checker::Checker;

pub use host_api::{
    host_apis, is_available as host_api_is_available, is_host_api, lookup as lookup_host_api,
    unsupported_diagnostic as host_api_unsupported_diagnostic, CompileTarget, HostApiEntry,
    HostAvailability,
};
pub use symbols::{BoundProgram, CheckedProgram, Symbol, SymbolId};
pub use types::{GenericFnSig, IntersectionType, NativeType, ObjectShape, Type, UnionType};

pub(crate) use early::{
    catch_lexical_conflict, check_statement_list_early_errors, collect_lexically_declared_names,
    collect_var_declared_names_stmt,
};
pub(crate) use syntax::{
    body_has_use_strict, expr_contains_super, expr_has_optional_chain, fn_params_of_expr,
    is_iteration_labelled_item, is_simple_parameter_list, is_void_type_ann, params_contain_super,
    params_contain_super_call, peel_parens, stmt_cannot_fall_through, stmt_contains_super,
    stmt_contains_super_call, stmt_list_has_use_strict, stmt_span, strict_forbidden_assign_target,
};
pub(crate) use types::format_type_full;

use draconic_ast::Program;
use draconic_diagnostics::{codes, Diagnostic, Span};

/// Hard diagnostic when `extern "C"` / FFI appears on the js target (F08.01).
pub fn extern_unsupported_on_js_diagnostic(name: &str, span: Span) -> Diagnostic {
    Diagnostic::new(
        format!("extern \"C\" function `{name}` is unsupported on js target (native-only FFI)"),
        span,
    )
    .with_code(codes::EXTERN_UNSUPPORTED)
    .with_help("compile with the native backend, or remove the extern declaration")
}

/// Bind scopes and resolve identifiers for a minimal Program.
pub fn bind(program: Program) -> Result<BoundProgram, Diagnostic> {
    let mut checker = Checker::new();
    checker.typecheck = false;
    checker.analyze(&program, false)?;
    Ok(checker.into_bound(program))
}

/// Bind under Module goal (E19.67): top-level functions are lexical, not var-like.
pub fn bind_module(program: Program) -> Result<BoundProgram, Diagnostic> {
    let mut checker = Checker::new();
    checker.typecheck = false;
    checker.binder.strict = true;
    checker.analyze(&program, true)?;
    Ok(checker.into_bound(program))
}

pub fn check(program: Program) -> Result<CheckedProgram, Diagnostic> {
    // Script goal: top-level `await` / `for await` rejected.
    // No host-target policy (call [`check_for_target`] when the backend is known).
    check_with_module_goal(program, false, None)
}

/// Check a Program under the Module goal (E19.28): top-level `await` and
/// `for await` are allowed (async module). Nested non-async functions still
/// reject `await`.
pub fn check_module(program: Program) -> Result<CheckedProgram, Diagnostic> {
    check_with_module_goal(program, true, None)
}

/// Check a Script-goal Program for a specific compile target (H00.01).
///
/// Free references to registered host APIs that are unavailable on `target`
/// produce a hard diagnostic ([`codes::HOST_API_UNSUPPORTED`]).
pub fn check_for_target(
    program: Program,
    target: CompileTarget,
) -> Result<CheckedProgram, Diagnostic> {
    check_with_module_goal(program, false, Some(target))
}

/// Check a Module-goal Program for a specific compile target (H00.01).
pub fn check_module_for_target(
    program: Program,
    target: CompileTarget,
) -> Result<CheckedProgram, Diagnostic> {
    check_with_module_goal(program, true, Some(target))
}

fn check_with_module_goal(
    program: Program,
    module_goal: bool,
    target: Option<CompileTarget>,
) -> Result<CheckedProgram, Diagnostic> {
    let mut checker = Checker::new();
    checker.typecheck = true;
    // Module evaluation may be async when the body uses top-level await.
    checker.in_async = module_goal;
    checker.host_target = target;
    if module_goal {
        checker.binder.strict = true;
    }
    checker.analyze(&program, module_goal)?;
    Ok(checker.into_checked(program))
}

#[cfg(test)]
mod bind_globals;
#[cfg(test)]
mod check_globals;
