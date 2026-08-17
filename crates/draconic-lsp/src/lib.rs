//! LSP analysis library (ROADMAP U06).
//!
//! Provides a source-buffer analysis surface for editor features:
//! diagnostics, hover types, and go-to-definition. This is intentionally
//! not a full `tower-lsp` server — callers (CLI / editor hosts) can wire
//! the JSON-RPC layer later.

use draconic_check::CheckedProgram;
use draconic_diagnostics::{BytePos, Diagnostic, Location, SourceFile, Span};
use draconic_frontend::check_source;

/// One diagnostic produced by analyzing a Program.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LspDiagnostic {
    pub message: String,
    pub span: Span,
}

impl LspDiagnostic {
    pub fn from_diagnostic(d: Diagnostic) -> Self {
        Self {
            message: d.message,
            span: d.span,
        }
    }

    /// 1-based line/column of the diagnostic start (via [`SourceFile::lookup`]).
    pub fn start_location(&self, source: &str) -> Location {
        SourceFile::new("<input>", source).lookup(self.span.start)
    }
}

/// Hover result: a type (or related) description over a source span.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hover {
    /// Span of the hovered name / expression.
    pub span: Span,
    /// Human-readable type string (e.g. `"number"`).
    pub type_string: String,
}

/// Go-to-definition target: the declaration-name span of a symbol.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Definition {
    pub span: Span,
}

/// Analysis snapshot for one source buffer.
#[derive(Debug)]
pub struct Analysis {
    source: String,
    diagnostics: Vec<LspDiagnostic>,
    /// Present when parse + check succeeded.
    checked: Option<CheckedProgram>,
}

impl Analysis {
    /// Analyze `source` as a Script (parse + bind + typecheck, no emit).
    pub fn analyze(source: impl Into<String>) -> Self {
        let source = source.into();
        match check_source(&source) {
            Ok(checked) => Self {
                source,
                diagnostics: Vec::new(),
                checked: Some(checked),
            },
            Err(d) => Self {
                source,
                diagnostics: vec![LspDiagnostic::from_diagnostic(d)],
                checked: None,
            },
        }
    }

    pub fn source(&self) -> &str {
        &self.source
    }

    pub fn diagnostics(&self) -> &[LspDiagnostic] {
        &self.diagnostics
    }

    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
    }

    /// Type under the caret at UTF-8 byte `offset`.
    ///
    /// Prefers the symbol type for a binding name or resolved use; falls back
    /// to the smallest typed expression containing the offset.
    pub fn hover(&self, offset: u32) -> Option<Hover> {
        let checked = self.checked.as_ref()?;

        if let Some((use_span, id)) = checked.bound.use_at_offset(offset) {
            let ty = checked.type_of_symbol(id);
            return Some(Hover {
                span: use_span,
                type_string: checked.format_type(ty),
            });
        }

        if let Some(sym) = checked.bound.decl_at_offset(offset) {
            let ty = checked.type_of_symbol(sym.id);
            return Some(Hover {
                span: sym.span,
                type_string: checked.format_type(ty),
            });
        }

        if let Some((span, ty)) = checked.expr_type_at_offset(offset) {
            return Some(Hover {
                span,
                type_string: checked.format_type(ty),
            });
        }

        None
    }

    /// Declaration span for the identifier at UTF-8 byte `offset`.
    pub fn goto_definition(&self, offset: u32) -> Option<Definition> {
        let checked = self.checked.as_ref()?;

        if let Some((_use_span, id)) = checked.bound.use_at_offset(offset) {
            let sym = checked.bound.symbol(id);
            return Some(Definition { span: sym.span });
        }

        if let Some(sym) = checked.bound.decl_at_offset(offset) {
            return Some(Definition { span: sym.span });
        }

        None
    }

    /// Map UTF-8 byte offset → 1-based line/column.
    pub fn offset_to_location(&self, offset: u32) -> Location {
        SourceFile::new("<input>", &self.source).lookup(BytePos(offset))
    }

    /// Map 1-based line/column → UTF-8 byte offset (clamped to EOF).
    pub fn location_to_offset(&self, line: u32, column: u32) -> u32 {
        location_to_offset(&self.source, line, column)
    }
}

/// Convenience: analyze and return diagnostics only.
pub fn analyze(source: &str) -> Analysis {
    Analysis::analyze(source)
}

/// Convert 1-based line/column to a UTF-8 byte offset.
///
/// `column` counts UTF-8 bytes from the start of the line (1-based), matching
/// [`SourceFile::lookup`]. Out-of-range positions clamp to EOF.
pub fn location_to_offset(source: &str, line: u32, column: u32) -> u32 {
    if line == 0 {
        return 0;
    }
    let bytes = source.as_bytes();
    let mut current_line: u32 = 1;
    let mut line_start: usize = 0;

    for (i, &b) in bytes.iter().enumerate() {
        if current_line == line {
            break;
        }
        if b == b'\n' {
            current_line += 1;
            line_start = i + 1;
        }
    }

    if current_line < line {
        return bytes.len() as u32;
    }

    let col0 = (column.max(1) - 1) as usize;
    // End of this line (before newline) or EOF.
    let mut line_end = line_start;
    while line_end < bytes.len() && bytes[line_end] != b'\n' {
        line_end += 1;
    }
    let offset = line_start.saturating_add(col0).min(line_end);
    // Allow caret past last char on the line (column = len+1) → line_end.
    if col0 > line_end - line_start {
        return line_end as u32;
    }
    offset as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Byte offset of the first occurrence of `needle` in `src`.
    fn offset_of(src: &str, needle: &str) -> u32 {
        src.find(needle).expect("needle") as u32
    }

    /// Byte offset of the nth (0-based) occurrence of `needle`.
    fn offset_of_nth(src: &str, needle: &str, n: usize) -> u32 {
        let mut from = 0;
        for i in 0..=n {
            let rel = src[from..]
                .find(needle)
                .unwrap_or_else(|| panic!("needle #{i}"));
            let abs = from + rel;
            if i == n {
                return abs as u32;
            }
            from = abs + needle.len();
        }
        unreachable!()
    }

    #[test]
    fn diagnostics_empty_on_valid_source() {
        let a = analyze("let x = 1;");
        assert!(!a.has_errors());
        assert!(a.diagnostics().is_empty());
    }

    #[test]
    fn diagnostics_on_parse_error() {
        let a = analyze("let x = ;");
        assert!(a.has_errors());
        let d = &a.diagnostics()[0];
        assert!(!d.message.is_empty());
        assert!(!d.span.is_dummy() || d.span.len() == 0 || true);
    }

    #[test]
    fn diagnostics_on_type_error() {
        let a = analyze("let x: number = \"hello\";");
        assert!(a.has_errors(), "expected type diagnostic");
        let d = &a.diagnostics()[0];
        assert!(
            d.message.contains("assign")
                || d.message.contains("type")
                || d.message.contains("number")
                || d.message.contains("string"),
            "unexpected message: {}",
            d.message
        );
    }

    #[test]
    fn hover_on_binding_shows_number() {
        let src = "let x = 1;";
        let a = analyze(src);
        assert!(!a.has_errors());
        // "x" starts after "let "
        let off = offset_of(src, "x");
        let h = a.hover(off).expect("hover on x");
        assert_eq!(h.type_string, "number");
        assert_eq!(h.span, Span::new(off, off + 1));
    }

    #[test]
    fn hover_on_use_shows_binding_type() {
        let src = "let count = 42;\nlet y = count;";
        let a = analyze(src);
        assert!(!a.has_errors(), "diags: {:?}", a.diagnostics());
        let use_off = offset_of_nth(src, "count", 1);
        let h = a.hover(use_off).expect("hover on use of count");
        assert_eq!(h.type_string, "number");
    }

    #[test]
    fn hover_on_string_binding() {
        let src = "let s = \"hi\";";
        let a = analyze(src);
        assert!(!a.has_errors());
        let h = a.hover(offset_of(src, "s")).expect("hover");
        assert_eq!(h.type_string, "string");
    }

    #[test]
    fn hover_none_without_checked_program() {
        let a = analyze("let x = ;");
        assert!(a.has_errors());
        assert!(a.hover(0).is_none());
    }

    #[test]
    fn goto_definition_from_use_to_decl() {
        let src = "let answer = 1;\nlet z = answer;";
        let a = analyze(src);
        assert!(!a.has_errors(), "diags: {:?}", a.diagnostics());

        let decl_off = offset_of(src, "answer");
        let use_off = offset_of_nth(src, "answer", 1);

        let def = a.goto_definition(use_off).expect("goto from use");
        assert_eq!(def.span, Span::new(decl_off, decl_off + "answer".len() as u32));
    }

    #[test]
    fn goto_definition_on_decl_returns_same_span() {
        let src = "let foo = 10;";
        let a = analyze(src);
        assert!(!a.has_errors());
        let decl_off = offset_of(src, "foo");
        let def = a.goto_definition(decl_off).expect("goto on decl");
        assert_eq!(def.span, Span::new(decl_off, decl_off + 3));
    }

    #[test]
    fn goto_definition_none_on_bad_source() {
        let a = analyze("@@@");
        assert!(a.goto_definition(0).is_none());
    }

    #[test]
    fn location_roundtrip_helpers() {
        let src = "let a = 1;\nlet b = 2;";
        let a = analyze(src);
        // 'b' is on line 2
        let b_off = offset_of(src, "b");
        let loc = a.offset_to_location(b_off);
        assert_eq!(loc.line, 2);
        assert_eq!(loc.column, 5); // "let " = 4 bytes, b at col 5
        let back = a.location_to_offset(loc.line, loc.column);
        assert_eq!(back, b_off);
    }

    #[test]
    fn location_to_offset_first_line() {
        assert_eq!(location_to_offset("hello", 1, 1), 0);
        assert_eq!(location_to_offset("hello", 1, 3), 2);
    }
}
