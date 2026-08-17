//! U07: LLVM DWARF debug info mapping Draconic source lines.

use std::path::Path;

use draconic_diagnostics::{SourceFile, Span};
use draconic_ir::Module;

/// Source context for native DWARF emission.
#[derive(Debug, Clone)]
pub struct SourceDebug {
    /// Absolute or relative path shown in DWARF `DIFile`.
    pub path: String,
    /// Full source text (for span → line/column).
    pub source: String,
}

impl SourceDebug {
    pub fn from_path(path: &Path, source: impl Into<String>) -> Self {
        Self {
            path: path.display().to_string(),
            source: source.into(),
        }
    }

    fn file_name(&self) -> &str {
        Path::new(&self.path)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or(self.path.as_str())
    }

    fn directory(&self) -> String {
        Path::new(&self.path)
            .parent()
            .map(|p| {
                if p.as_os_str().is_empty() {
                    ".".to_string()
                } else {
                    p.display().to_string()
                }
            })
            .unwrap_or_else(|| ".".to_string())
    }

    fn location(&self, span: Span) -> (u32, u32) {
        if span.is_dummy() {
            return (1, 1);
        }
        let file = SourceFile::new(self.file_name(), &self.source);
        let loc = file.lookup(span.start);
        (loc.line.max(1), loc.column.max(1))
    }
}

/// First non-dummy top-level body span line/column, else (1, 1).
pub fn primary_location(module: &Module, debug: &SourceDebug) -> (u32, u32) {
    for span in &module.body_spans {
        if !span.is_dummy() {
            return debug.location(*span);
        }
    }
    (1, 1)
}

/// Attach DWARF metadata to LLVM IR text.
///
/// - Declares `DICompileUnit` / `DIFile` / `DISubprogram` for `@main`
/// - Ensures `@main` carries `!dbg !subprogram`
/// - Adds `!dbg !DILocation` on instructions inside `@main` that lack one,
///   using the first body span line (coarse map for adapters without per-stmt tags)
/// - Honors `; draconic-dbg: line col` markers (native_ints per-stmt map)
pub fn attach_debug_info(ir: &str, module: &Module, debug: &SourceDebug) -> String {
    if ir.contains("!llvm.dbg.cu") {
        return ir.to_string();
    }

    let (prim_line, prim_col) = primary_location(module, debug);
    let file_name = debug.file_name().to_string();
    let directory = debug.directory();

    // Metadata ids (stable small set + dynamic locations).
    // !0 = CU, !1 = file, !2 = empty, !3 = main SP, !4 = subroutine type,
    // !5 = type list, !6 = int type, !7+ = DILocations
    let mut locations: Vec<(u32, u32)> = Vec::new();
    let mut loc_index = |line: u32, col: u32| -> usize {
        if let Some(i) = locations.iter().position(|&(l, c)| l == line && c == col) {
            return i;
        }
        locations.push((line, col));
        locations.len() - 1
    };
    let default_loc = loc_index(prim_line, prim_col);

    let mut out = String::with_capacity(ir.len() + 1024);
    let mut in_main = false;
    let mut main_done = false;
    let mut current_loc = default_loc;

    for line in ir.lines() {
        let trimmed = line.trim_start();

        // Per-statement markers from native_ints (not emitted as IR).
        if let Some(rest) = trimmed.strip_prefix("; draconic-dbg:") {
            let mut parts = rest.split_whitespace();
            if let (Some(ls), Some(cs)) = (parts.next(), parts.next()) {
                if let (Ok(l), Ok(c)) = (ls.parse::<u32>(), cs.parse::<u32>()) {
                    current_loc = loc_index(l.max(1), c.max(1));
                }
            }
            continue;
        }

        if !main_done && trimmed.starts_with("define ") && trimmed.contains("@main") {
            in_main = true;
            if trimmed.contains("!dbg ") {
                out.push_str(line);
                out.push('\n');
            } else if let Some(stripped) = trimmed.strip_suffix('{') {
                let indent = &line[..line.len() - trimmed.len()];
                out.push_str(indent);
                out.push_str(stripped.trim_end());
                out.push_str(" !dbg !3 {\n");
            } else {
                out.push_str(line);
                out.push('\n');
            }
            current_loc = default_loc;
            continue;
        }

        if in_main {
            if trimmed == "}" {
                in_main = false;
                main_done = true;
                out.push_str(line);
                out.push('\n');
                continue;
            }

            // Instructions: indented, not labels, not comments.
            let is_label = trimmed.ends_with(':') && !trimmed.contains(' ');
            let is_comment = trimmed.starts_with(';');
            let is_inst = line.starts_with(' ')
                && !trimmed.is_empty()
                && !is_label
                && !is_comment
                && !trimmed.starts_with("target ");

            if is_inst && !trimmed.contains("!dbg ") {
                let base = line.trim_end();
                out.push_str(base);
                // loc metadata id = 7 + index
                let meta_id = 7 + current_loc;
                out.push_str(&format!(", !dbg !{meta_id}\n"));
                continue;
            }
        }

        out.push_str(line);
        out.push('\n');
    }

    // Module flags + CU
    out.push_str("\n!llvm.dbg.cu = !{!0}\n");
    out.push_str("!llvm.module.flags = !{!10, !11}\n");
    out.push_str("!llvm.ident = !{!12}\n\n");

    out.push_str(&format!(
        "!0 = distinct !DICompileUnit(language: DW_LANG_C_plus_plus, file: !1, producer: \"draconic\", isOptimized: false, runtimeVersion: 0, emissionKind: FullDebug, enums: !2)\n"
    ));
    out.push_str(&format!(
        "!1 = !DIFile(filename: \"{}\", directory: \"{}\")\n",
        escape_md(&file_name),
        escape_md(&directory)
    ));
    out.push_str("!2 = !{}\n");
    out.push_str(&format!(
        "!3 = distinct !DISubprogram(name: \"main\", scope: !1, file: !1, line: {prim_line}, type: !4, scopeLine: {prim_line}, spFlags: DISPFlagDefinition, unit: !0, retainedNodes: !2)\n"
    ));
    out.push_str("!4 = !DISubroutineType(types: !5)\n");
    out.push_str("!5 = !{!6}\n");
    out.push_str("!6 = !DIBasicType(name: \"int\", size: 32, encoding: DW_ATE_signed)\n");

    for (i, &(line, col)) in locations.iter().enumerate() {
        let id = 7 + i;
        out.push_str(&format!(
            "!{id} = !DILocation(line: {line}, column: {col}, scope: !3)\n"
        ));
    }

    out.push_str("!10 = !{i32 7, !\"Dwarf Version\", i32 4}\n");
    out.push_str("!11 = !{i32 2, !\"Debug Info Version\", i32 3}\n");
    out.push_str("!12 = !{!\"draconic\"}\n");

    out
}

/// Emit a body marker consumed by [`attach_debug_info`].
pub fn dbg_marker(line: u32, column: u32) -> String {
    format!("; draconic-dbg: {line} {column}")
}

fn escape_md(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

#[cfg(test)]
mod tests {
    use super::*;
    use draconic_diagnostics::Span;
    use draconic_ir::{BindingKind, IrType, Local, LocalId, Module, Stmt};

    fn tiny_module() -> Module {
        Module {
            locals: vec![Local {
                id: LocalId(0),
                name: "x".into(),
                ty: IrType::Native(draconic_ir::NativeType::I32),
                kind: BindingKind::Let,
            }],
            body: vec![Stmt::Declare {
                local: LocalId(0),
                init: None,
                kind: BindingKind::Let,
            }],
            body_spans: vec![Span::new(0, 10)],
            shapes: vec![],
            has_extern_ffi: false,
        }
    }

    #[test]
    fn attach_adds_cu_and_main_dbg() {
        let ir = "define i32 @main() {\nentry:\n  ret i32 0\n}\n";
        let debug = SourceDebug {
            path: "/tmp/sample.drac".into(),
            source: "let x: i32 = 1;\n".into(),
        };
        let out = attach_debug_info(ir, &tiny_module(), &debug);
        assert!(out.contains("!llvm.dbg.cu"), "{out}");
        assert!(out.contains("DIFile(filename: \"sample.drac\""), "{out}");
        assert!(out.contains("define i32 @main() !dbg !3"), "{out}");
        assert!(out.contains("ret i32 0, !dbg !"), "{out}");
        assert!(out.contains("DILocation(line: 1,"), "{out}");
    }

    #[test]
    fn markers_select_line() {
        let ir = "\
define i32 @main() {
entry:
; draconic-dbg: 2 1
  %a = add i32 1, 2
; draconic-dbg: 3 1
  ret i32 0
}
";
        let debug = SourceDebug {
            path: "m.drac".into(),
            source: "\nlet a: i32 = 1;\nlet b: i32 = 2;\n".into(),
        };
        let out = attach_debug_info(ir, &tiny_module(), &debug);
        assert!(out.contains("DILocation(line: 2,"), "{out}");
        assert!(out.contains("DILocation(line: 3,"), "{out}");
        // both lines referenced
        assert!(out.matches("DILocation").count() >= 2, "{out}");
    }
}
