//! ROADMAP U06: LSP analysis — diagnostics, hover types, go-to-definition.

use draconic_diagnostics::Span;
use draconic_lsp::analyze;

#[test]
fn lsp_type_diagnostic_on_bad_annotation() {
    let a = analyze("let x: number = \"nope\";");
    assert!(a.has_errors());
    assert!(!a.diagnostics()[0].message.is_empty());
}

#[test]
fn lsp_hover_and_goto_definition_roundtrip() {
    let src = "let value = 7;\nlet out = value;";
    let a = analyze(src);
    assert!(!a.has_errors(), "unexpected diags: {:?}", a.diagnostics());

    let decl = src.find("value").expect("decl") as u32;
    let use_site = src.rfind("value").expect("use") as u32;

    let hover = a.hover(use_site).expect("hover use");
    assert_eq!(hover.type_string, "number");

    let def = a.goto_definition(use_site).expect("goto");
    assert_eq!(def.span, Span::new(decl, decl + "value".len() as u32));
}
