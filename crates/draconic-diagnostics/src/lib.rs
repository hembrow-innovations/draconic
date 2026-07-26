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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub message: String,
    pub span: Span,
}

impl Diagnostic {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
        }
    }

    /// Rustc-style multi-line diagnostic with source snippet and caret underline.
    ///
    /// ```text
    /// error: message
    ///  --> name:line:col
    ///   |
    /// N | source line
    ///   |     ^^^^
    /// ```
    ///
    /// Dummy / empty spans omit the snippet and caret.
    pub fn pretty(&self, file: &SourceFile<'_>) -> String {
        let loc = file.lookup(self.span.start);
        let mut out = String::new();
        out.push_str("error: ");
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

        out
    }
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} at {}..{}",
            self.message, self.span.start.0, self.span.end.0
        )
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
        assert_eq!(d.to_string(), "unexpected token at 4..5");
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
