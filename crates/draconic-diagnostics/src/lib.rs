use std::fmt;

/// Byte offset into the source file (UTF-8 bytes).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BytePos(pub u32);

/// Half-open span `[start, end)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Span {
    pub start: BytePos,
    pub end: BytePos,
}

impl Span {
    pub fn new(start: u32, end: u32) -> Self {
        Self {
            start: BytePos(start),
            end: BytePos(end),
        }
    }

    pub fn dummy() -> Self {
        Self::new(0, 0)
    }

    pub fn is_dummy(self) -> bool {
        self.start.0 == 0 && self.end.0 == 0
    }

    pub fn len(self) -> u32 {
        self.end.0.saturating_sub(self.start.0)
    }
}

/// 1-based line and column within a source file.
///
/// `column` counts UTF-8 bytes from the start of the line (1-based).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Location {
    pub line: u32,
    pub column: u32,
}

/// Source text with an optional path, used for location lookup and pretty-print.
#[derive(Debug, Clone, Copy)]
pub struct SourceFile<'a> {
    pub name: &'a str,
    pub src: &'a str,
}

impl<'a> SourceFile<'a> {
    pub fn new(name: &'a str, src: &'a str) -> Self {
        Self { name, src }
    }

    /// Map a byte offset to 1-based line/column.
    ///
    /// Offsets past the end of the source clamp to the final position
    /// (end-of-file: last line, column = line length + 1).
    pub fn lookup(&self, pos: BytePos) -> Location {
        let bytes = self.src.as_bytes();
        let target = (pos.0 as usize).min(bytes.len());

        let mut line: u32 = 1;
        let mut line_start: usize = 0;

        for (i, &b) in bytes.iter().enumerate() {
            if i >= target {
                break;
            }
            if b == b'\n' {
                line += 1;
                line_start = i + 1;
            }
        }

        let column = (target - line_start) as u32 + 1;
        Location { line, column }
    }

    /// Text of the 1-based `line`, without the trailing newline.
    /// Returns empty string if the line is out of range.
    pub fn line_text(&self, line: u32) -> &str {
        if line == 0 {
            return "";
        }
        let mut current: u32 = 1;
        let mut start: usize = 0;
        let bytes = self.src.as_bytes();

        for (i, &b) in bytes.iter().enumerate() {
            if b == b'\n' {
                if current == line {
                    return &self.src[start..i];
                }
                current += 1;
                start = i + 1;
            }
        }

        if current == line {
            return &self.src[start..];
        }
        ""
    }
}

/// Stable diagnostic code (`E0300`, …). Displayed as `E` + four zero-padded digits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ErrorCode(pub u32);

impl ErrorCode {
    pub const fn new(n: u32) -> Self {
        Self(n)
    }

    /// Canonical label, e.g. `E0300`.
    pub fn label(self) -> String {
        format!("E{:04}", self.0)
    }
}

impl fmt::Display for ErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "E{:04}", self.0)
    }
}

/// Common checker diagnostic codes (U09). Stable once assigned.
pub mod codes {
    use super::ErrorCode;

    /// Type is not assignable to expected type.
    pub const NOT_ASSIGNABLE: ErrorCode = ErrorCode(300);
    /// Value is not callable.
    pub const NOT_CALLABLE: ErrorCode = ErrorCode(301);
    /// Value is not constructable with `new`.
    pub const NOT_CONSTRUCTABLE: ErrorCode = ErrorCode(302);
    /// Wrong number of arguments at a call.
    pub const WRONG_ARITY: ErrorCode = ErrorCode(303);
    /// Annotated non-void function may fall off the end without returning.
    pub const MISSING_RETURN: ErrorCode = ErrorCode(304);
    /// Object literal has a property absent from the annotated shape.
    pub const EXCESS_PROPERTY: ErrorCode = ErrorCode(305);
    /// Property read/write names a key absent from an annotated shape.
    pub const UNKNOWN_PROPERTY: ErrorCode = ErrorCode(306);
    /// Extern / FFI signature uses a non-ABI type or missing annotation (F06.02).
    pub const INVALID_EXTERN_TYPE: ErrorCode = ErrorCode(307);
    /// Host API is not available on the compile target (H00.01).
    pub const HOST_API_UNSUPPORTED: ErrorCode = ErrorCode(400);
    /// `extern "C"` / FFI is native-only; unsupported on the js target (F08.01).
    pub const EXTERN_UNSUPPORTED: ErrorCode = ErrorCode(401);
    /// `--link` shared library path does not exist (F05.02).
    pub const MISSING_DYNAMIC_LIB: ErrorCode = ErrorCode(402);
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
    /// Optional stable error code (U09).
    pub code: Option<ErrorCode>,
    /// Optional help / suggestion line shown under the caret in pretty output.
    pub help: Option<String>,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            code: None,
            help: None,
        }
    }

    pub fn with_code(mut self, code: ErrorCode) -> Self {
        self.code = Some(code);
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }

    /// Rustc-style multi-line diagnostic with source snippet and caret underline.
    ///
    /// ```text
    /// error[E0300]: message
    ///  --> name:line:col
    ///   |
    /// N | source line
    ///   |     ^^^^
    ///   = help: suggestion
    /// ```
    ///
    /// Without a code the header is `error: message`. Dummy / empty spans omit
    /// the snippet and caret. Help is omitted when unset.
    pub fn pretty(&self, file: &SourceFile<'_>) -> String {
        let loc = file.lookup(self.span.start);
        let mut out = String::new();
        if let Some(code) = self.code {
            out.push_str("error[");
            out.push_str(&code.label());
            out.push_str("]: ");
        } else {
            out.push_str("error: ");
        }
        out.push_str(&self.message);
        out.push('\n');
        out.push_str(" --> ");
        out.push_str(file.name);
        out.push(':');
        out.push_str(&loc.line.to_string());
        out.push(':');
        out.push_str(&loc.column.to_string());
        out.push('\n');

        if self.span.is_dummy() || self.span.len() == 0 {
            if let Some(help) = &self.help {
                out.push_str("  = help: ");
                out.push_str(help);
                out.push('\n');
            }
            return out;
        }

        let line = file.line_text(loc.line);
        let line_no = loc.line.to_string();
        let gutter = line_no.len();

        // blank gutter row
        for _ in 0..gutter {
            out.push(' ');
        }
        out.push_str(" |\n");

        // source line
        out.push_str(&line_no);
        out.push_str(" | ");
        out.push_str(line);
        out.push('\n');

        // caret row
        for _ in 0..gutter {
            out.push(' ');
        }
        out.push_str(" | ");

        let col0 = (loc.column as usize).saturating_sub(1);
        let mut underline_len = self.span.len() as usize;

        // Clamp underline to remaining bytes on this line (single-line spans).
        let max_on_line = line.len().saturating_sub(col0);
        if underline_len > max_on_line {
            underline_len = max_on_line;
        }
        if underline_len == 0 {
            underline_len = 1;
        }

        for _ in 0..col0 {
            out.push(' ');
        }
        for _ in 0..underline_len {
            out.push('^');
        }
        out.push('\n');

        if let Some(help) = &self.help {
            out.push_str("  = help: ");
            out.push_str(help);
            out.push('\n');
        }

        out
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(code) = self.code {
            write!(
                f,
                "[{}] {} at {}..{}",
                code.label(),
                self.message,
                self.span.start.0,
                self.span.end.0
            )
        } else {
            write!(
                f,
                "{} at {}..{}",
                self.message, self.span.start.0, self.span.end.0
            )
        }
    }
}

impl std::error::Error for Diagnostic {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn span_new_and_dummy() {
        let s = Span::new(3, 7);
        assert_eq!(s.start.0, 3);
        assert_eq!(s.end.0, 7);
        assert_eq!(s.len(), 4);
        assert!(!s.is_dummy());

        let d = Span::dummy();
        assert!(d.is_dummy());
        assert_eq!(d.len(), 0);
    }

    #[test]
    fn diagnostic_message_and_display() {
        let d = Diagnostic::new("unexpected token", Span::new(4, 5));
        assert_eq!(d.message, "unexpected token");
        assert_eq!(d.span, Span::new(4, 5));
        assert_eq!(d.code, None);
        assert_eq!(d.help, None);
        assert_eq!(d.to_string(), "unexpected token at 4..5");
    }

    #[test]
    fn error_code_label_is_stable() {
        assert_eq!(ErrorCode::new(300).label(), "E0300");
        assert_eq!(codes::NOT_ASSIGNABLE.label(), "E0300");
        assert_eq!(codes::NOT_CALLABLE.label(), "E0301");
        assert_eq!(codes::NOT_CONSTRUCTABLE.label(), "E0302");
        assert_eq!(codes::WRONG_ARITY.label(), "E0303");
        assert_eq!(codes::MISSING_RETURN.label(), "E0304");
        assert_eq!(codes::EXCESS_PROPERTY.label(), "E0305");
        assert_eq!(codes::UNKNOWN_PROPERTY.label(), "E0306");
        assert_eq!(codes::INVALID_EXTERN_TYPE.label(), "E0307");
        assert_eq!(codes::HOST_API_UNSUPPORTED.label(), "E0400");
        assert_eq!(codes::EXTERN_UNSUPPORTED.label(), "E0401");
        assert_eq!(codes::MISSING_DYNAMIC_LIB.label(), "E0402");
        assert_eq!(codes::NOT_ASSIGNABLE.to_string(), "E0300");
    }

    #[test]
    fn diagnostic_with_code_and_help_defaults_off() {
        let d = Diagnostic::new("type mismatch", Span::new(0, 1))
            .with_code(codes::NOT_ASSIGNABLE)
            .with_help("widen the annotation or change the value");
        assert_eq!(d.code, Some(codes::NOT_ASSIGNABLE));
        assert_eq!(
            d.help.as_deref(),
            Some("widen the annotation or change the value")
        );
        assert_eq!(d.to_string(), "[E0300] type mismatch at 0..1");
    }

    #[test]
    fn lookup_first_line() {
        let file = SourceFile::new("t.drac", "let x = 1;\n");
        assert_eq!(file.lookup(BytePos(0)), Location { line: 1, column: 1 });
        assert_eq!(file.lookup(BytePos(4)), Location { line: 1, column: 5 });
    }

    #[test]
    fn lookup_second_line() {
        let src = "abc\ndef\n";
        let file = SourceFile::new("t.drac", src);
        // 'd' is at byte 4
        assert_eq!(file.lookup(BytePos(4)), Location { line: 2, column: 1 });
        assert_eq!(file.lookup(BytePos(6)), Location { line: 2, column: 3 });
    }

    #[test]
    fn lookup_past_end_clamps() {
        let file = SourceFile::new("t.drac", "hi");
        assert_eq!(file.lookup(BytePos(100)), Location { line: 1, column: 3 });
    }

    #[test]
    fn line_text_strips_newline() {
        let file = SourceFile::new("t.drac", "one\ntwo\nthree");
        assert_eq!(file.line_text(1), "one");
        assert_eq!(file.line_text(2), "two");
        assert_eq!(file.line_text(3), "three");
        assert_eq!(file.line_text(4), "");
        assert_eq!(file.line_text(0), "");
    }

    #[test]
    fn pretty_print_with_caret() {
        //              0123456789
        let src = "let foo = 1;\n";
        // underline "foo" at 4..7
        let d = Diagnostic::new("unresolved identifier `foo`", Span::new(4, 7));
        let file = SourceFile::new("main.drac", src);
        let pretty = d.pretty(&file);
        assert_eq!(
            pretty,
            "\
error: unresolved identifier `foo`
 --> main.drac:1:5
  |
1 | let foo = 1;
  |     ^^^
"
        );
    }

    #[test]
    fn pretty_print_with_code_and_help() {
        let src = "let x: string = 1;\n";
        // "1" at bytes 16..17
        let d = Diagnostic::new(
            "type `number` is not assignable to type `string`",
            Span::new(16, 17),
        )
        .with_code(codes::NOT_ASSIGNABLE)
        .with_help("change the value to match the expected type, or widen the annotation");
        let file = SourceFile::new("main.drac", src);
        let pretty = d.pretty(&file);
        assert_eq!(
            pretty,
            "\
error[E0300]: type `number` is not assignable to type `string`
 --> main.drac:1:17
  |
1 | let x: string = 1;
  |                 ^
  = help: change the value to match the expected type, or widen the annotation
"
        );
    }

    #[test]
    fn pretty_print_multiline_source_points_at_correct_line() {
        let src = "let a = 1;\nlet b = c;\n";
        // "c" is on line 2: bytes — line1 "let a = 1;\n" = 11 bytes, then "let b = c;"
        // c is at index 11 + 8 = 19
        let d = Diagnostic::new("unresolved identifier `c`", Span::new(19, 20));
        let file = SourceFile::new("x.drac", src);
        let pretty = d.pretty(&file);
        assert_eq!(
            pretty,
            "\
error: unresolved identifier `c`
 --> x.drac:2:9
  |
2 | let b = c;
  |         ^
"
        );
    }

    #[test]
    fn pretty_print_dummy_span_omits_snippet() {
        let d = Diagnostic::new("io failure", Span::dummy());
        let file = SourceFile::new("out.js", "");
        let pretty = d.pretty(&file);
        assert_eq!(
            pretty,
            "\
error: io failure
 --> out.js:1:1
"
        );
    }

    #[test]
    fn pretty_print_dummy_span_still_shows_help() {
        let d = Diagnostic::new("io failure", Span::dummy()).with_help("check the path");
        let file = SourceFile::new("out.js", "");
        let pretty = d.pretty(&file);
        assert_eq!(
            pretty,
            "\
error: io failure
 --> out.js:1:1
  = help: check the path
"
        );
    }

    #[test]
    fn pretty_print_empty_span_omits_snippet() {
        // empty but non-dummy (start == end != 0..0 is still empty length)
        let d = Diagnostic::new("here", Span::new(5, 5));
        let file = SourceFile::new("t.drac", "hello world");
        let pretty = d.pretty(&file);
        assert_eq!(
            pretty,
            "\
error: here
 --> t.drac:1:6
"
        );
    }

    #[test]
    fn pretty_print_wider_line_number_gutter() {
        // 10 lines so line 10 has 2-digit gutter
        let mut src = String::new();
        for _ in 0..9 {
            src.push_str("x\n");
        }
        src.push_str("bad token\n");
        // line 10 starts at byte 18 (9 * "x\n")
        let d = Diagnostic::new("bad", Span::new(18, 21));
        let file = SourceFile::new("t.drac", &src);
        let pretty = d.pretty(&file);
        assert_eq!(
            pretty,
            "\
error: bad
 --> t.drac:10:1
   |
10 | bad token
   | ^^^
"
        );
    }
}
