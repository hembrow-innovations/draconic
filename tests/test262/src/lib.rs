//! Test262 staged harness (ROADMAP E19 / ADR 0007).
//!
//! js target only. Resolves the suite from `TEST262_ROOT` or
//! `<workspace>/third_party/test262`. When the suite is absent, runs skip
//! (CI stays green). When present, compiles each allowlisted test through the
//! Draconic frontend → JS backend and executes under Node with a minimal
//! `$ERROR` / `assert` shim.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use draconic_backend_js::emit_js;
use draconic_frontend::compile_source;

/// Outcome bucket for one allowlisted path.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Pass,
    Fail,
    Skip,
}

impl Status {
    pub fn as_str(self) -> &'static str {
        match self {
            Status::Pass => "pass",
            Status::Fail => "fail",
            Status::Skip => "skip",
        }
    }
}

/// One allowlist entry after a run attempt.
#[derive(Debug, Clone)]
pub struct CaseResult {
    pub path: String,
    pub status: Status,
    pub message: String,
}

/// Aggregate of a harness run.
#[derive(Debug, Clone)]
pub struct Report {
    pub suite_root: Option<PathBuf>,
    pub suite_present: bool,
    pub cases: Vec<CaseResult>,
}

impl Report {
    pub fn counts(&self) -> (usize, usize, usize) {
        let mut pass = 0;
        let mut fail = 0;
        let mut skip = 0;
        for c in &self.cases {
            match c.status {
                Status::Pass => pass += 1,
                Status::Fail => fail += 1,
                Status::Skip => skip += 1,
            }
        }
        (pass, fail, skip)
    }

    /// Markdown baseline report (checked in or written under target/).
    pub fn to_markdown(&self) -> String {
        let (pass, fail, skip) = self.counts();
        let mut out = String::new();
        out.push_str("# Test262 baseline report\n\n");
        out.push_str("Staged roll-in (ADR 0007). Target: **js** only.\n");
        out.push_str("Failures are report-only until triage promotes Roadmap rows.\n\n");
        match &self.suite_root {
            Some(p) if self.suite_present => {
                out.push_str(&format!("- Suite root: `{}`\n", p.display()));
                out.push_str("- Suite: **present**\n");
            }
            Some(p) => {
                out.push_str(&format!("- Suite root (missing): `{}`\n", p.display()));
                out.push_str("- Suite: **absent** (all allowlist entries skipped)\n");
            }
            None => out.push_str("- Suite root: unresolved\n"),
        }
        out.push_str(&format!(
            "- Totals: pass={pass} fail={fail} skip={skip} (allowlist={})\n\n",
            self.cases.len()
        ));
        out.push_str("| Path | Status | Message |\n");
        out.push_str("|------|--------|---------|\n");
        for c in &self.cases {
            let msg = c.message.replace('|', "\\|").replace('\n', " ");
            out.push_str(&format!(
                "| `{}` | {} | {} |\n",
                c.path,
                c.status.as_str(),
                msg
            ));
        }
        out.push('\n');
        out
    }
}

/// Package root (`tests/test262`).
pub fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Workspace root (parent of `tests/`).
pub fn workspace_root() -> PathBuf {
    package_root()
        .parent()
        .and_then(|p| p.parent())
        .map(Path::to_path_buf)
        .expect("tests/test262 → workspace root")
}

/// Resolve suite root: `TEST262_ROOT` env, else `<workspace>/third_party/test262`.
pub fn resolve_suite_root() -> PathBuf {
    if let Ok(p) = std::env::var("TEST262_ROOT") {
        let trimmed = p.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    workspace_root().join("third_party").join("test262")
}

/// True when `root` looks like a usable test262 checkout (`test/` directory).
pub fn suite_present(root: &Path) -> bool {
    root.is_dir() && root.join("test").is_dir()
}

/// Default allowlist path.
pub fn allowlist_path() -> PathBuf {
    package_root().join("allowlist.txt")
}

/// Load relative test paths from allowlist (comments/blank lines ignored).
pub fn load_allowlist(path: &Path) -> Result<Vec<String>, String> {
    let text = fs::read_to_string(path)
        .map_err(|e| format!("read allowlist {}: {e}", path.display()))?;
    let mut out = Vec::new();
    for (i, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if line.contains("..") {
            return Err(format!(
                "allowlist line {}: path must not contain `..`: {line}",
                i + 1
            ));
        }
        out.push(line.to_string());
    }
    if out.is_empty() {
        return Err(format!("allowlist empty: {}", path.display()));
    }
    Ok(out)
}

/// Minimal Test262 harness symbols, parseable by Draconic.
///
/// Real `harness/assert.js` is not compiled through the frontend (it uses
/// patterns outside the current surface). This shim covers `$ERROR` and
/// `assert.sameValue` / `assert.notSameValue` / `assert.throws` used by
/// early language tests (incl. E19.07 BigInt mixed-type TypeError paths).
pub const HARNESS_SHIM: &str = r#"
function $ERROR(message) {
  throw new Error(String(message));
}
function Test262Error(message) {
  this.message = message;
}
function assert(mustBeTrue, message) {
  if (mustBeTrue !== true) {
    $ERROR(message || ("Expected true but got " + String(mustBeTrue)));
  }
}
assert.sameValue = function(actual, expected, message) {
  let same = actual === expected;
  if (actual !== actual && expected !== expected) {
    same = true;
  }
  if (same === false) {
    $ERROR(message || ("Expected SameValue, got " + String(actual) + " vs " + String(expected)));
  }
};
assert.notSameValue = function(actual, unexpected, message) {
  let same = actual === unexpected;
  if (actual !== actual && unexpected !== unexpected) {
    same = true;
  }
  if (same === true) {
    $ERROR(message || "Unexpected SameValue match");
  }
};
assert.throws = function(expectedErrorConstructor, func, message) {
  if (typeof func !== "function") {
    $ERROR("assert.throws requires two arguments: the error constructor and a function to run");
  }
  let msg = "";
  if (message !== undefined) {
    msg = message + " ";
  }
  let threw = false;
  try {
    func();
  } catch (thrown) {
    threw = true;
    if (typeof thrown !== "object" || thrown === null) {
      $ERROR(msg + "Thrown value was not an object!");
    } else if (thrown.constructor !== expectedErrorConstructor) {
      let expectedName = expectedErrorConstructor.name;
      let actualName = thrown.constructor.name;
      $ERROR(msg + "Expected a " + expectedName + " but got a " + actualName);
    }
  }
  if (threw === false) {
    $ERROR(msg + "Expected a " + expectedErrorConstructor.name + " to be thrown but no exception was thrown at all");
  }
};
assert.compareArray = function(actual, expected, message) {
  if (typeof actual !== "object" || actual === null || typeof expected !== "object" || expected === null) {
    $ERROR(message || "assert.compareArray requires array-like arguments");
  }
  let al = actual.length;
  let el = expected.length;
  if (al !== el) {
    $ERROR(message || ("Expected array length " + el + " but got " + al + ": [" + Array.prototype.join.call(actual, ",") + "] vs [" + Array.prototype.join.call(expected, ",") + "]"));
  }
  let i = 0;
  while (i < al) {
    let a = actual[i];
    let e = expected[i];
    let same = a === e;
    if (a !== a && e !== e) {
      same = true;
    }
    if (same === false) {
      $ERROR(message || ("arrays differ at " + i + ": " + String(a) + " vs " + String(e) + " (actual=[" + Array.prototype.join.call(actual, ",") + "])"));
    }
    i = i + 1;
  }
};
"#;

/// Locate Test262 YAML frontmatter (`/*--- ... ---*/`), if present.
///
/// Frontmatter may follow a copyright line-comment prologue.
fn frontmatter_meta(source: &str) -> Option<&str> {
    let start = source.find("/*---")?;
    let after = &source[start + 5..];
    let end = after.find("---*/")?;
    Some(&after[..end])
}

/// Strip Test262 YAML frontmatter comment block if present.
pub fn strip_frontmatter(source: &str) -> &str {
    let Some(start) = source.find("/*---") else {
        return source;
    };
    let after = &source[start + 5..];
    let Some(end) = after.find("---*/") else {
        return source;
    };
    after[end + 5..].trim_start()
}

/// True when frontmatter declares a negative parse/early SyntaxError expectation.
pub fn is_negative_parse(source: &str) -> bool {
    let Some(meta) = frontmatter_meta(source) else {
        return false;
    };
    if !meta.contains("negative:") {
        return false;
    }
    meta.contains("phase: parse") || meta.contains("phase: early")
}

/// True when frontmatter declares a negative runtime expectation (error must be thrown).
pub fn is_negative_runtime(source: &str) -> bool {
    let Some(meta) = frontmatter_meta(source) else {
        return false;
    };
    meta.contains("negative:") && meta.contains("phase: runtime")
}

/// True when frontmatter requires strict mode only (`flags: [onlyStrict]`).
///
/// E19.19: strict PutValue TypeError on compound assignment needs a leading
/// `"use strict"` so Node observes the same mode as Test262's onlyStrict run.
pub fn is_only_strict(source: &str) -> bool {
    let Some(meta) = frontmatter_meta(source) else {
        return false;
    };
    // flags: [onlyStrict] or flags: [onlyStrict, ...] — bracket form used by suite.
    meta.lines().any(|line| {
        let t = line.trim();
        t.starts_with("flags:") && t.contains("onlyStrict")
    }) || meta.contains("onlyStrict")
}

/// Compile Test262 test body (+ shim) through frontend → JS emit.
pub fn compile_test_to_js(test_body: &str) -> Result<String, String> {
    let body = strip_frontmatter(test_body);
    // `"use strict"` must be the first statement so the whole script (incl. body) is strict.
    let source = if is_only_strict(test_body) {
        format!("\"use strict\";\n{HARNESS_SHIM}\n{body}")
    } else {
        format!("{HARNESS_SHIM}\n{body}")
    };
    let module = compile_source(&source).map_err(|d| format!("compile: {d}"))?;
    emit_js(&module).map_err(|d| format!("emit_js: {d}"))
}

/// Run emitted JS under Node. Exit 0 = pass.
pub fn run_js_in_node(js: &str) -> Result<(), String> {
    let output = Command::new("node")
        .arg("-e")
        .arg(js)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("spawn node: {e}"))?;
    let code = output.status.code().unwrap_or(1);
    if code == 0 {
        return Ok(());
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    Err(format!(
        "node exit {code}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}"
    ))
}

/// Run one allowlisted relative path against `suite_root`.
///
/// Panics from the compiler (e.g. mid-UTF-8 lexer bugs) are caught and reported
/// as `Fail` so baseline triage stays report-only (ADR 0007 / E19.02).
pub fn run_case(suite_root: &Path, rel: &str) -> CaseResult {
    let path = rel.to_string();
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        run_case_inner(suite_root, rel)
    })) {
        Ok(c) => c,
        Err(_) => CaseResult {
            path,
            status: Status::Fail,
            message: "panic during compile/run (see E19.03 lexer UTF-8 / related)".into(),
        },
    }
}

fn run_case_inner(suite_root: &Path, rel: &str) -> CaseResult {
    let full = suite_root.join(rel);
    if !full.is_file() {
        return CaseResult {
            path: rel.to_string(),
            status: Status::Fail,
            message: format!("missing file: {}", full.display()),
        };
    }
    let source = match fs::read_to_string(&full) {
        Ok(s) => s,
        Err(e) => {
            return CaseResult {
                path: rel.to_string(),
                status: Status::Fail,
                message: format!("read: {e}"),
            };
        }
    };
    if is_negative_parse(&source) {
        // Negative parse/early: pass iff frontend rejects the body.
        return match compile_test_to_js(&source) {
            Err(_) => CaseResult {
                path: rel.to_string(),
                status: Status::Pass,
                message: "ok (negative parse)".to_string(),
            },
            Ok(_) => CaseResult {
                path: rel.to_string(),
                status: Status::Fail,
                message: "expected compile failure for negative parse test".to_string(),
            },
        };
    }
    let js = match compile_test_to_js(&source) {
        Ok(j) => j,
        Err(e) => {
            return CaseResult {
                path: rel.to_string(),
                status: Status::Fail,
                message: e,
            };
        }
    };
    if is_negative_runtime(&source) {
        // Negative runtime: pass iff Node throws (exit ≠ 0).
        return match run_js_in_node(&js) {
            Err(_) => CaseResult {
                path: rel.to_string(),
                status: Status::Pass,
                message: "ok (negative runtime)".to_string(),
            },
            Ok(()) => CaseResult {
                path: rel.to_string(),
                status: Status::Fail,
                message: "expected runtime failure for negative runtime test".to_string(),
            },
        };
    }
    match run_js_in_node(&js) {
        Ok(()) => CaseResult {
            path: rel.to_string(),
            status: Status::Pass,
            message: "ok".to_string(),
        },
        Err(e) => CaseResult {
            path: rel.to_string(),
            status: Status::Fail,
            message: e,
        },
    }
}

/// Run the curated allowlist. Suite absent → every case `skip`.
pub fn run_allowlist(suite_root: &Path, allowlist: &[String]) -> Report {
    let present = suite_present(suite_root);
    let cases = if !present {
        allowlist
            .iter()
            .map(|p| CaseResult {
                path: p.clone(),
                status: Status::Skip,
                message: format!(
                    "suite not present at {} (run: node scripts/fetch-test262.mjs)",
                    suite_root.display()
                ),
            })
            .collect()
    } else {
        allowlist
            .iter()
            .map(|p| run_case(suite_root, p))
            .collect()
    };
    Report {
        suite_root: Some(suite_root.to_path_buf()),
        suite_present: present,
        cases,
    }
}

/// Default entry: resolve root + package allowlist.
pub fn run_default() -> Result<Report, String> {
    let root = resolve_suite_root();
    let list = load_allowlist(&allowlist_path())?;
    Ok(run_allowlist(&root, &list))
}

/// Write markdown report next to the package (or `DRACONIC_TEST262_REPORT` path).
pub fn write_baseline_report(report: &Report) -> Result<PathBuf, String> {
    let path = if let Ok(p) = std::env::var("DRACONIC_TEST262_REPORT") {
        PathBuf::from(p)
    } else {
        package_root().join("baseline-report.md")
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir {}: {e}", parent.display()))?;
    }
    fs::write(&path, report.to_markdown())
        .map_err(|e| format!("write {}: {e}", path.display()))?;
    // Also stamp under target/ when available (CI artifact friendly).
    let stamp = workspace_root()
        .join("target")
        .join("test262-baseline-report.md");
    if let Some(parent) = stamp.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(&stamp, report.to_markdown());
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allowlist_loads_and_has_entries() {
        let list = load_allowlist(&allowlist_path()).expect("allowlist");
        // E19.02/E19.06/E19.10/E19.15/E19.20 expanded curated set (expressions + statements + more).
        assert!(
            list.len() >= 6500,
            "expected expanded curated allowlist (>=6500), got {}",
            list.len()
        );
        assert!(list.iter().all(|p| p.starts_with("test/")));
    }

    #[test]
    fn strip_frontmatter_removes_yaml_block() {
        let src = "/*---\ndescription: x\n---*/\nif (true) {}\n";
        let body = strip_frontmatter(src);
        assert!(body.starts_with("if (true)"));
    }

    #[test]
    fn harness_shim_compiles_and_runs() {
        let js = compile_test_to_js("assert.sameValue(1 + 2, 3);\n").expect("compile");
        run_js_in_node(&js).expect("node");
    }

    #[test]
    fn harness_shim_catches_failure() {
        let js = compile_test_to_js("assert.sameValue(1, 2, \"nope\");\n").expect("compile");
        assert!(run_js_in_node(&js).is_err());
    }

    #[test]
    fn harness_shim_throws_typeerror() {
        let js = compile_test_to_js(
            r#"
            assert.throws(TypeError, function() { 1n + 1; });
            assert.throws(TypeError, function() { 1 + 1n; });
            "#,
        )
        .expect("compile");
        run_js_in_node(&js).expect("node");
    }

    #[test]
    fn harness_shim_throws_fails_when_no_throw() {
        let js = compile_test_to_js("assert.throws(TypeError, function() { 1 + 1; });\n")
            .expect("compile");
        assert!(run_js_in_node(&js).is_err());
    }

    #[test]
    fn missing_suite_skips_all() {
        let root = workspace_root().join("third_party").join("test262-does-not-exist");
        let list = vec![
            "test/language/types/boolean/S8.3_A1_T1.js".to_string(),
            "test/language/types/null/S8.2_A1_T1.js".to_string(),
        ];
        let report = run_allowlist(&root, &list);
        assert!(!report.suite_present);
        assert_eq!(report.cases.len(), 2);
        assert!(report.cases.iter().all(|c| c.status == Status::Skip));
        let (p, f, s) = report.counts();
        assert_eq!((p, f, s), (0, 0, 2));
    }

    #[test]
    fn markdown_report_mentions_totals() {
        let report = Report {
            suite_root: Some(PathBuf::from("/tmp/nope")),
            suite_present: false,
            cases: vec![CaseResult {
                path: "test/x.js".into(),
                status: Status::Skip,
                message: "suite not present".into(),
            }],
        };
        let md = report.to_markdown();
        assert!(md.contains("pass=0"));
        assert!(md.contains("skip=1"));
        assert!(md.contains("test/x.js"));
    }

    #[test]
    fn negative_parse_meta_detected() {
        let src = "/*---\nnegative:\n  phase: parse\n  type: SyntaxError\n---*/\n1_\n";
        assert!(is_negative_parse(src));
        assert!(!is_negative_parse("/*---\ndescription: x\n---*/\n1\n"));
    }

    #[test]
    fn only_strict_meta_detected() {
        let src = "/*---\nflags: [onlyStrict]\ndescription: x\n---*/\n1\n";
        assert!(is_only_strict(src));
        assert!(!is_only_strict("/*---\ndescription: x\n---*/\n1\n"));
        assert!(!is_only_strict("/*---\nflags: [noStrict]\n---*/\n1\n"));
    }

    #[test]
    fn only_strict_compound_assign_putvalue_typeerror() {
        // E19.19: non-writable data prop + compound `*=` must TypeError under onlyStrict.
        let src = r#"
/*---
description: strict PutValue TypeError on compound assign to non-writable
flags: [onlyStrict]
---*/
var obj = {};
Object.defineProperty(obj, "prop", {
  value: 10,
  writable: false,
  enumerable: true,
  configurable: true
});
assert.throws(TypeError, function() {
  obj.prop *= 20;
});
assert.sameValue(obj.prop, 10, "obj.prop");
"#;
        let js = compile_test_to_js(src).expect("compile");
        assert!(
            js.contains("use strict"),
            "emitted JS must include use strict for onlyStrict: {js}"
        );
        run_js_in_node(&js).expect("node strict PutValue");
    }

    #[test]
    fn only_strict_compound_assign_nonextensible_typeerror() {
        // E19.19: missing prop on non-extensible object.
        let src = r#"
/*---
flags: [onlyStrict]
---*/
var obj = {};
Object.preventExtensions(obj);
assert.throws(TypeError, function() {
  obj.len *= 10;
});
"#;
        let js = compile_test_to_js(src).expect("compile");
        run_js_in_node(&js).expect("node non-extensible PutValue");
    }

    #[test]
    fn default_run_does_not_fail_ci_without_suite() {
        // Suite missing → all skip (CI green). Suite present → allowlist must pass.
        let report = run_default().expect("run_default");
        let path = write_baseline_report(&report).expect("write report");
        assert!(path.is_file(), "report path {}", path.display());
        let (pass, fail, skip) = report.counts();
        eprintln!(
            "test262 default: present={} pass={pass} fail={fail} skip={skip} report={}",
            report.suite_present,
            path.display()
        );
        if !report.suite_present {
            assert!(skip > 0);
            assert_eq!(fail, 0);
        }
        // E19.02: expanded allowlist must stay green when suite is present.
        if report.suite_present {
            assert_eq!(
                fail, 0,
                "allowlisted Test262 cases must pass (got fail={fail}); triage before expanding"
            );
            assert!(
                pass >= 6500,
                "expected expanded allowlist pass count >= 6500, got {pass}"
            );
        }
    }
}
