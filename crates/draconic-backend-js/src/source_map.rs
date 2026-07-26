//! Source Map v3 (VLQ) for JS emit (ROADMAP U03).

use draconic_diagnostics::{Location, SourceFile, Span};

const BASE64: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Options controlling source map generation alongside JS emit.
#[derive(Debug, Clone, Copy)]
pub struct SourceMapOptions<'a> {
    /// Original Program path recorded in `sources`.
    pub source_name: &'a str,
    /// Original Program text; when set and `inline_sources_content`, stored in the map.
    pub source_content: Option<&'a str>,
    /// Generated file name (`file` field), e.g. `out.js`.
    pub output_file: Option<&'a str>,
    /// When true, embed `source_content` as `sourcesContent`.
    pub inline_sources_content: bool,
}

impl<'a> SourceMapOptions<'a> {
    pub fn new(source_name: &'a str) -> Self {
        Self {
            source_name,
            source_content: None,
            output_file: None,
            inline_sources_content: true,
        }
    }

    pub fn with_content(mut self, content: &'a str) -> Self {
        self.source_content = Some(content);
        self
    }

    pub fn with_output_file(mut self, file: &'a str) -> Self {
        self.output_file = Some(file);
        self
    }
}

/// Source Map revision 3 JSON object (in-memory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceMap {
    pub version: u32,
    pub file: Option<String>,
    pub source_root: Option<String>,
    pub sources: Vec<String>,
    pub sources_content: Vec<Option<String>>,
    pub names: Vec<String>,
    pub mappings: String,
}

impl SourceMap {
    /// Serialize to Source Map v3 JSON (no trailing newline).
    pub fn to_json(&self) -> String {
        let mut out = String::from("{\n");
        out.push_str("  \"version\": 3,\n");
        if let Some(file) = &self.file {
            out.push_str("  \"file\": ");
            push_json_string(&mut out, file);
            out.push_str(",\n");
        }
        if let Some(root) = &self.source_root {
            out.push_str("  \"sourceRoot\": ");
            push_json_string(&mut out, root);
            out.push_str(",\n");
        }
        out.push_str("  \"sources\": [");
        for (i, s) in self.sources.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            push_json_string(&mut out, s);
        }
        out.push_str("],\n");
        if !self.sources_content.is_empty() {
            out.push_str("  \"sourcesContent\": [");
            for (i, c) in self.sources_content.iter().enumerate() {
                if i > 0 {
                    out.push_str(", ");
                }
                match c {
                    Some(text) => push_json_string(&mut out, text),
                    None => out.push_str("null"),
                }
            }
            out.push_str("],\n");
        }
        out.push_str("  \"names\": [");
        for (i, n) in self.names.iter().enumerate() {
            if i > 0 {
                out.push_str(", ");
            }
            push_json_string(&mut out, n);
        }
        out.push_str("],\n");
        out.push_str("  \"mappings\": ");
        push_json_string(&mut out, &self.mappings);
        out.push_str("\n}");
        out
    }
}

fn push_json_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => {
                use std::fmt::Write as _;
                let _ = write!(out, "\\u{:04x}", c as u32);
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

/// Encode a signed value as Base64 VLQ (Source Map v3).
pub fn encode_vlq(value: i32) -> String {
    let mut vlq = if value < 0 {
        ((-value) << 1) + 1
    } else {
        value << 1
    };
    let mut out = String::new();
    loop {
        let mut digit = vlq & 0b11111;
        vlq >>= 5;
        if vlq > 0 {
            digit |= 0b100000;
        }
        out.push(BASE64[digit as usize] as char);
        if vlq == 0 {
            break;
        }
    }
    out
}

/// Decode one Base64 VLQ value; returns (value, bytes_consumed).
pub fn decode_vlq(input: &str) -> Option<(i32, usize)> {
    let bytes = input.as_bytes();
    let mut result: i32 = 0;
    let mut shift = 0;
    let mut i = 0;
    loop {
        if i >= bytes.len() {
            return None;
        }
        let b = bytes[i];
        i += 1;
        let digit = base64_digit(b)?;
        result |= (digit & 0b11111) << shift;
        shift += 5;
        if digit & 0b100000 == 0 {
            break;
        }
    }
    let value = if result & 1 != 0 {
        -(result >> 1)
    } else {
        result >> 1
    };
    Some((value, i))
}

fn base64_digit(b: u8) -> Option<i32> {
    let idx = match b {
        b'A'..=b'Z' => b - b'A',
        b'a'..=b'z' => b - b'a' + 26,
        b'0'..=b'9' => b - b'0' + 52,
        b'+' => 62,
        b'/' => 63,
        _ => return None,
    };
    Some(idx as i32)
}

/// One decoded mapping segment (0-based lines/columns).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mapping {
    pub generated_line: u32,
    pub generated_column: u32,
    pub source_index: u32,
    pub original_line: u32,
    pub original_column: u32,
}

/// Decode the `mappings` field into a flat list of segments.
pub fn decode_mappings(mappings: &str) -> Vec<Mapping> {
    let mut out = Vec::new();
    let mut gen_line: u32 = 0;
    let mut last_gen_col: i32 = 0;
    let mut last_src: i32 = 0;
    let mut last_orig_line: i32 = 0;
    let mut last_orig_col: i32 = 0;

    for line in mappings.split(';') {
        last_gen_col = 0;
        if line.is_empty() {
            gen_line += 1;
            continue;
        }
        for seg in line.split(',') {
            let mut rest = seg;
            let Some((d_gen_col, n)) = decode_vlq(rest) else {
                break;
            };
            rest = &rest[n..];
            last_gen_col += d_gen_col;

            // Need at least source, orig line, orig col for a full segment.
            let Some((d_src, n)) = decode_vlq(rest) else {
                continue;
            };
            rest = &rest[n..];
            last_src += d_src;

            let Some((d_oline, n)) = decode_vlq(rest) else {
                continue;
            };
            rest = &rest[n..];
            last_orig_line += d_oline;

            let Some((d_ocol, _)) = decode_vlq(rest) else {
                continue;
            };
            last_orig_col += d_ocol;

            out.push(Mapping {
                generated_line: gen_line,
                generated_column: last_gen_col as u32,
                source_index: last_src as u32,
                original_line: last_orig_line as u32,
                original_column: last_orig_col as u32,
            });
        }
        gen_line += 1;
    }
    out
}

/// Builder that tracks generated position and encodes VLQ mappings.
#[derive(Debug)]
pub struct SourceMapBuilder {
    opts_source_name: String,
    opts_output_file: Option<String>,
    opts_source_content: Option<String>,
    inline_sources_content: bool,
    /// 0-based generated line/column (UTF-16 code units; ASCII emit ⇒ bytes).
    gen_line: u32,
    gen_col: u32,
    mappings: String,
    /// Whether current generated line already has a segment started.
    line_has_segment: bool,
    last_gen_col: i32,
    last_src: i32,
    last_orig_line: i32,
    last_orig_col: i32,
    /// Original source for span → location lookup.
    source_text: String,
}

impl SourceMapBuilder {
    pub fn new(opts: &SourceMapOptions<'_>) -> Self {
        Self {
            opts_source_name: opts.source_name.to_string(),
            opts_output_file: opts.output_file.map(|s| s.to_string()),
            opts_source_content: opts.source_content.map(|s| s.to_string()),
            inline_sources_content: opts.inline_sources_content,
            gen_line: 0,
            gen_col: 0,
            mappings: String::new(),
            line_has_segment: false,
            last_gen_col: 0,
            last_src: 0,
            last_orig_line: 0,
            last_orig_col: 0,
            source_text: opts.source_content.unwrap_or("").to_string(),
        }
    }

    pub fn generated_line(&self) -> u32 {
        self.gen_line
    }

    pub fn generated_column(&self) -> u32 {
        self.gen_col
    }

    /// Note that `text` was appended to the generated output.
    pub fn note_write(&mut self, text: &str) {
        for b in text.bytes() {
            if b == b'\n' {
                self.gen_line += 1;
                self.gen_col = 0;
                self.mappings.push(';');
                self.line_has_segment = false;
                self.last_gen_col = 0;
            } else {
                // ASCII-only emit path: 1 byte == 1 UTF-16 code unit.
                self.gen_col += 1;
            }
        }
    }

    /// Record a mapping at the current generated position from `span`'s start.
    /// Dummy spans are skipped.
    pub fn add_mapping_span(&mut self, span: Span) {
        if span.is_dummy() {
            return;
        }
        let file = SourceFile::new(&self.opts_source_name, &self.source_text);
        let loc = file.lookup(span.start);
        self.add_mapping_loc(loc);
    }

    /// Record a mapping at the current generated position.
    /// `loc` is 1-based line/column (diagnostics convention); stored 0-based in the map.
    pub fn add_mapping_loc(&mut self, loc: Location) {
        let orig_line = loc.line.saturating_sub(1) as i32;
        let orig_col = loc.column.saturating_sub(1) as i32;
        let gen_col = self.gen_col as i32;

        if self.line_has_segment {
            self.mappings.push(',');
        }
        self.line_has_segment = true;

        // generated column (relative)
        let d_gen = gen_col - self.last_gen_col;
        self.mappings.push_str(&encode_vlq(d_gen));
        self.last_gen_col = gen_col;

        // source index (relative) — always source 0
        let d_src = 0 - self.last_src;
        self.mappings.push_str(&encode_vlq(d_src));
        self.last_src = 0;

        // original line (relative)
        let d_oline = orig_line - self.last_orig_line;
        self.mappings.push_str(&encode_vlq(d_oline));
        self.last_orig_line = orig_line;

        // original column (relative)
        let d_ocol = orig_col - self.last_orig_col;
        self.mappings.push_str(&encode_vlq(d_ocol));
        self.last_orig_col = orig_col;
    }

    pub fn finish(self) -> SourceMap {
        let sources_content = if self.inline_sources_content {
            vec![self.opts_source_content]
        } else {
            Vec::new()
        };
        SourceMap {
            version: 3,
            file: self.opts_output_file,
            source_root: None,
            sources: vec![self.opts_source_name],
            sources_content,
            names: Vec::new(),
            mappings: self.mappings,
        }
    }
}

/// `//# sourceMappingURL=` comment line (includes leading newline).
pub fn source_mapping_url_comment(url: &str) -> String {
    format!("\n//# sourceMappingURL={url}\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn vlq_roundtrip_small() {
        for v in [-100, -1, 0, 1, 2, 15, 16, 31, 32, 100, 1000] {
            let enc = encode_vlq(v);
            let (dec, n) = decode_vlq(&enc).expect("decode");
            assert_eq!(dec, v, "enc={enc}");
            assert_eq!(n, enc.len());
        }
    }

    #[test]
    fn source_map_json_shape() {
        let map = SourceMap {
            version: 3,
            file: Some("out.js".into()),
            source_root: None,
            sources: vec!["in.drac".into()],
            sources_content: vec![Some("let x = 1;\n".into())],
            names: vec![],
            mappings: "AAAA".into(),
        };
        let json = map.to_json();
        assert!(json.contains("\"version\": 3"), "{json}");
        assert!(json.contains("\"file\": \"out.js\""), "{json}");
        assert!(json.contains("\"sources\": [\"in.drac\"]"), "{json}");
        assert!(json.contains("\"mappings\": \"AAAA\""), "{json}");
        assert!(json.contains("let x = 1;\\n"), "{json}");
    }

    #[test]
    fn builder_one_mapping() {
        let opts = SourceMapOptions::new("t.drac").with_content("let x = 1;\n");
        let mut b = SourceMapBuilder::new(&opts);
        b.add_mapping_loc(Location { line: 1, column: 1 });
        b.note_write("let x = 1;\n");
        let map = b.finish();
        let segs = decode_mappings(&map.mappings);
        assert_eq!(segs.len(), 1);
        assert_eq!(
            segs[0],
            Mapping {
                generated_line: 0,
                generated_column: 0,
                source_index: 0,
                original_line: 0,
                original_column: 0,
            }
        );
    }
}
