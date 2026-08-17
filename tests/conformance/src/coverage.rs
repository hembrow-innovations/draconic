//! Line coverage helpers for conformance runs (ROADMAP U11).
//!
//! JS path: source-map-guided probes in generated JS; hits dumped after Node runs.
//! Native is not instrumented in v1 (`js and/or native`).

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_js::{decode_mappings, SourceMap};

/// Aggregate line coverage across fixtures (one entry per source path).
#[derive(Debug, Default, Clone)]
pub struct CoverageReport {
    files: BTreeMap<String, FileCoverage>,
}

#[derive(Debug, Default, Clone)]
struct FileCoverage {
    /// 1-based original lines that have at least one source-map segment.
    executable: BTreeSet<u32>,
    /// 1-based original lines observed at runtime.
    hit: BTreeSet<u32>,
}

impl CoverageReport {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.files.is_empty()
    }

    pub fn merge_file(&mut self, path: &str, executable: BTreeSet<u32>, hit: BTreeSet<u32>) {
        let entry = self.files.entry(path.to_string()).or_default();
        entry.executable.extend(executable);
        entry.hit.extend(hit);
    }

    /// Human-readable summary for CLI stdout.
    pub fn format_summary(&self) -> String {
        let mut out = String::from("coverage (js line):\n");
        if self.files.is_empty() {
            out.push_str("  (no instrumented lines)\n");
            return out;
        }
        let mut total_exec = 0u32;
        let mut total_hit = 0u32;
        for (path, file) in &self.files {
            let exec = file.executable.len() as u32;
            let hit = file
                .executable
                .iter()
                .filter(|l| file.hit.contains(l))
                .count() as u32;
            total_exec += exec;
            total_hit += hit;
            let pct = if exec == 0 {
                100
            } else {
                (hit * 100) / exec
            };
            out.push_str(&format!("  {path}: {hit}/{exec} lines ({pct}%)\n"));
        }
        let pct = if total_exec == 0 {
            100
        } else {
            (total_hit * 100) / total_exec
        };
        out.push_str(&format!(
            "total: {total_hit}/{total_exec} lines ({pct}%)\n"
        ));
        out
    }

    pub fn total_hit(&self) -> u32 {
        self.files
            .values()
            .map(|f| {
                f.executable
                    .iter()
                    .filter(|l| f.hit.contains(l))
                    .count() as u32
            })
            .sum()
    }

    pub fn total_executable(&self) -> u32 {
        self.files.values().map(|f| f.executable.len() as u32).sum()
    }
}

/// Inject probes for each original line and return instrumented code + executable set (1-based).
pub fn instrument_js(code: &str, map: &SourceMap) -> (String, BTreeSet<u32>) {
    let mappings = decode_mappings(&map.mappings);
    let mut first_gen_line: BTreeMap<u32, u32> = BTreeMap::new();
    let mut executable = BTreeSet::new();
    for m in &mappings {
        let orig_1 = m.original_line + 1;
        executable.insert(orig_1);
        first_gen_line
            .entry(m.original_line)
            .and_modify(|g| {
                if m.generated_line < *g {
                    *g = m.generated_line;
                }
            })
            .or_insert(m.generated_line);
    }

    let mut probes_at_gen: BTreeMap<u32, Vec<u32>> = BTreeMap::new();
    for (orig0, gen_line) in first_gen_line {
        probes_at_gen
            .entry(gen_line)
            .or_default()
            .push(orig0 + 1);
    }

    let line_starts = line_start_offsets(code);
    let mut inserts: Vec<(usize, String)> = Vec::new();
    for (gen_line, lines) in probes_at_gen {
        let Some(&start) = line_starts.get(gen_line as usize) else {
            continue;
        };
        let mut probe = String::new();
        for line in lines {
            probe.push_str(&format!("globalThis.__drcov[{line}]=1;"));
        }
        inserts.push((start, probe));
    }
    inserts.sort_by_key(|(off, _)| std::cmp::Reverse(*off));

    let mut out = code.to_string();
    for (off, probe) in inserts {
        out.insert_str(off, &probe);
    }
    let header = "globalThis.__drcov=globalThis.__drcov||Object.create(null);\n";
    out.insert_str(0, header);
    (out, executable)
}

/// Wrap script so hits are written to `cov_path` even if the body throws.
pub fn wrap_coverage_dump(script: &str, cov_path: &Path) -> String {
    let path_json = json_string(&cov_path.to_string_lossy());
    format!(
        "try {{\n{script}\n}} finally {{\n\
         try {{\n\
         require('fs').writeFileSync({path_json}, JSON.stringify(globalThis.__drcov||{{}}));\n\
         }} catch (e) {{}}\n\
         }}\n"
    )
}

pub fn read_hits(path: &Path) -> BTreeSet<u32> {
    let Ok(text) = fs::read_to_string(path) else {
        return BTreeSet::new();
    };
    parse_hits_json(&text)
}

pub fn temp_cov_path(id: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let safe: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-cov-{}-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        N.fetch_add(1, Ordering::Relaxed),
        safe
    ));
    let _ = fs::create_dir_all(&dir);
    dir.join("hits.json")
}

fn line_start_offsets(code: &str) -> Vec<usize> {
    let mut starts = vec![0];
    for (i, b) in code.bytes().enumerate() {
        if b == b'\n' {
            starts.push(i + 1);
        }
    }
    starts
}

fn json_string(s: &str) -> String {
    let mut out = String::from("\"");
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
    out
}

/// Parse `{"1":1,"3":1}` style object from the coverage dump.
fn parse_hits_json(text: &str) -> BTreeSet<u32> {
    let mut hit = BTreeSet::new();
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' {
            i += 1;
            let start = i;
            while i < bytes.len() && bytes[i] != b'"' {
                i += 1;
            }
            if i > start {
                if let Ok(n) = std::str::from_utf8(&bytes[start..i])
                    .unwrap_or("")
                    .parse::<u32>()
                {
                    hit.insert(n);
                }
            }
        }
        i += 1;
    }
    hit
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_hits_json_keys() {
        let h = parse_hits_json(r#"{"1":1,"2":1,"10":1}"#);
        assert!(h.contains(&1));
        assert!(h.contains(&2));
        assert!(h.contains(&10));
        assert!(!h.contains(&3));
    }

    #[test]
    fn format_summary_empty() {
        let r = CoverageReport::new();
        let s = r.format_summary();
        assert!(s.contains("coverage"));
        assert!(s.contains("no instrumented"));
    }

    #[test]
    fn format_summary_with_file() {
        let mut r = CoverageReport::new();
        let mut exec = BTreeSet::new();
        exec.insert(1);
        exec.insert(2);
        let mut hit = BTreeSet::new();
        hit.insert(1);
        r.merge_file("a.drac", exec, hit);
        let s = r.format_summary();
        assert!(s.contains("a.drac"));
        assert!(s.contains("1/2"));
        assert!(s.contains("50%"));
    }
}
