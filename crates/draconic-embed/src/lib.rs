//! Embed: compiler-in-runtime for eval / Function. Not yet implemented — see ROADMAP N07, E16.

use draconic_diagnostics::Diagnostic;

pub fn eval_source(_source: &str) -> Result<(), Diagnostic> {
    Err(Diagnostic::new(
        "embed eval not implemented",
        draconic_diagnostics::Span::dummy(),
    ))
}
