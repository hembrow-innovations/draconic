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
use draconic_frontend::{compile_path, compile_source, compile_source_module};
use rayon::prelude::*;

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
// E19.29: minimal propertyHelper.js `verifyProperty` (descriptor checks via
// getOwnPropertyDescriptor; no destructive writable/configurable probes).
function verifyProperty(obj, name, desc, options) {
  let label = (options && options.label) || String(name);
  let originalDesc = Object.getOwnPropertyDescriptor(obj, name);
  if (desc === undefined) {
    assert.sameValue(originalDesc, undefined, label + " descriptor should be undefined");
    return true;
  }
  if (!Object.prototype.hasOwnProperty.call(obj, name)) {
    $ERROR(label + " should be an own property");
  }
  if (desc === null || typeof desc !== "object") {
    $ERROR("The desc argument should be an object or undefined");
  }
  if (Object.prototype.hasOwnProperty.call(desc, "value")) {
    let sameV = originalDesc.value === desc.value;
    if (originalDesc.value !== originalDesc.value && desc.value !== desc.value) {
      sameV = true;
    }
    if (sameV === false) {
      $ERROR(label + " descriptor value should be " + String(desc.value));
    }
    let cur = obj[name];
    let sameCur = cur === desc.value;
    if (cur !== cur && desc.value !== desc.value) {
      sameCur = true;
    }
    if (sameCur === false) {
      $ERROR(label + " value should be " + String(desc.value));
    }
  }
  if (Object.prototype.hasOwnProperty.call(desc, "enumerable") && desc.enumerable !== undefined) {
    if (desc.enumerable !== originalDesc.enumerable) {
      $ERROR(label + " descriptor should " + (desc.enumerable ? "" : "not ") + "be enumerable");
    }
  }
  if (Object.prototype.hasOwnProperty.call(desc, "writable") && desc.writable !== undefined) {
    if (desc.writable !== originalDesc.writable) {
      $ERROR(label + " descriptor should " + (desc.writable ? "" : "not ") + "be writable");
    }
  }
  if (Object.prototype.hasOwnProperty.call(desc, "configurable") && desc.configurable !== undefined) {
    if (desc.configurable !== originalDesc.configurable) {
      $ERROR(label + " descriptor should " + (desc.configurable ? "" : "not ") + "be configurable");
    }
  }
  if (Object.prototype.hasOwnProperty.call(desc, "get")) {
    if (originalDesc.get !== desc.get) {
      $ERROR(label + " getter mismatch");
    }
  }
  if (Object.prototype.hasOwnProperty.call(desc, "set")) {
    if (originalDesc.set !== desc.set) {
      $ERROR(label + " setter mismatch");
    }
  }
  return true;
}
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

/// True when frontmatter has the `async` **flag** (not `features: [async-…]`).
///
/// E19.26: async tests settle via `$DONE` rather than sync script completion.
pub fn is_async_flag(source: &str) -> bool {
    flag_token(source, "async")
}

/// True when frontmatter has the `module` **flag** (Module goal / top-level await).
///
/// E19.28: top-level `await` is valid only under Module goal.
pub fn is_module_flag(source: &str) -> bool {
    flag_token(source, "module")
}

/// True when frontmatter has the `raw` **flag** (hashbang / full-file source).
///
/// E19.39: hashbang and other early-byte tests must compile the full file so
/// content before the YAML frontmatter is not stripped away.
pub fn is_raw_flag(source: &str) -> bool {
    flag_token(source, "raw")
}

/// Match a single comma/bracket-separated token on a `flags:` frontmatter line,
/// or a YAML list item under `flags:` (`flags:\n  - module`).
fn flag_token(source: &str, token: &str) -> bool {
    let Some(meta) = frontmatter_meta(source) else {
        return false;
    };
    let mut in_flags_list = false;
    for line in meta.lines() {
        let t = line.trim();
        if t.starts_with("flags:") {
            for part in t.trim_start_matches("flags:").split([',', '[', ']']) {
                if part.trim() == token {
                    return true;
                }
            }
            // Multi-line YAML list: `flags:` alone or with nothing else on the line.
            let rest = t.trim_start_matches("flags:").trim();
            in_flags_list = rest.is_empty() || rest == "[]" || rest == "[";
            continue;
        }
        if in_flags_list {
            // Next top-level key ends the list.
            if !t.is_empty() && !t.starts_with('-') && t.contains(':') {
                in_flags_list = false;
                continue;
            }
            if let Some(item) = t.strip_prefix('-') {
                if item.trim() == token {
                    return true;
                }
            }
        }
    }
    false
}

/// Node-only host wrapper for Test262 `flags: [async]` (E19.26 / doneprintHandle).
///
/// Injected **after** frontend emit so `process` / `setTimeout` are not compiled.
/// Defines `$DONE` as a free global the emitted body looks up.
pub fn wrap_async_host(compiled_js: &str) -> String {
    format!(
        r#"
var __test262AsyncSettled = false;
var __test262AsyncTimer = setTimeout(function () {{
  if (!__test262AsyncSettled) {{
    console.error("Test262:AsyncTestFailure:Test262Error: timeout (no $DONE)");
    process.exit(1);
  }}
}}, 10000);
function $DONE(error) {{
  if (__test262AsyncSettled) {{
    return;
  }}
  __test262AsyncSettled = true;
  clearTimeout(__test262AsyncTimer);
  if (error) {{
    if (typeof error === "object" && error !== null && "name" in error) {{
      console.error(
        "Test262:AsyncTestFailure:" + error.name + ": " + String(error.message || "")
      );
    }} else {{
      console.error("Test262:AsyncTestFailure:Test262Error: " + String(error));
    }}
    process.exit(1);
  }}
  console.log("Test262:AsyncTestComplete");
  process.exit(0);
}}
process.on("unhandledRejection", function (reason) {{
  $DONE(reason);
}});
{compiled_js}
"#
    )
}

/// Compile Test262 test body (+ shim) through frontend → JS emit.
///
/// Script goal by default. When `flags: [module]` (E19.28), uses Module goal so
/// top-level `await` is accepted. When `test_path` is set and the body has
/// static import/export, links via a temp entry next to the test file.
pub fn compile_test_to_js(test_body: &str) -> Result<String, String> {
    compile_test_to_js_at(test_body, None)
}

/// Like [`compile_test_to_js`], with optional suite file path for Module link.
pub fn compile_test_to_js_at(test_body: &str, test_path: Option<&Path>) -> Result<String, String> {
    let module_goal = is_module_flag(test_body);
    // E19.39 raw: keep full file (hashbang + copyright + frontmatter-as-comment).
    // Hashbang must remain the first two bytes — append shim after the source.
    let source = if is_raw_flag(test_body) {
        if is_only_strict(test_body) {
            format!("{test_body}\n\"use strict\";\n{HARNESS_SHIM}")
        } else {
            format!("{test_body}\n{HARNESS_SHIM}")
        }
    } else {
        let body = strip_frontmatter(test_body);
        // `"use strict"` must be the first statement so the whole script (incl. body) is strict.
        if is_only_strict(test_body) {
            format!("\"use strict\";\n{HARNESS_SHIM}\n{body}")
        } else {
            format!("{HARNESS_SHIM}\n{body}")
        }
    };
    let scan_body = if is_raw_flag(test_body) {
        test_body
    } else {
        strip_frontmatter(test_body)
    };
    let needs_link = module_goal && source_has_static_module_syntax(scan_body);
    let module = if needs_link {
        let Some(path) = test_path else {
            return Err("compile: module test with import/export needs suite path".into());
        };
        let dir = path.parent().ok_or_else(|| "compile: test path has no parent".to_string())?;
        let tmp = dir.join(format!(
            ".draconic-test262-entry-{}.js",
            std::process::id()
        ));
        fs::write(&tmp, &source).map_err(|e| format!("compile: write temp entry: {e}"))?;
        let result = compile_path(&tmp).map_err(|d| format!("compile: {d}"));
        let _ = fs::remove_file(&tmp);
        result?
    } else if module_goal {
        compile_source_module(&source).map_err(|d| format!("compile: {d}"))?
    } else {
        compile_source(&source).map_err(|d| format!("compile: {d}"))?
    };
    emit_js(&module).map_err(|d| format!("emit_js: {d}"))
}

/// Rough scan: static `import`/`export` declarations (not dynamic `import()`).
fn source_has_static_module_syntax(body: &str) -> bool {
    for line in body.lines() {
        let t = line.trim_start();
        if t.starts_with("//") || t.starts_with("/*") {
            continue;
        }
        if t.starts_with("export ") || t.starts_with("export{") || t.starts_with("export*") {
            return true;
        }
        // `import … from` / `import "` / `import '` — not `import(`.
        if let Some(rest) = t.strip_prefix("import") {
            let rest = rest.trim_start();
            if rest.starts_with('(') {
                continue;
            }
            return true;
        }
    }
    false
}

/// Run emitted JS under Node. Exit 0 = pass.
///
/// When `cwd` is set (typically the test file's directory), relative
/// `import('./fixture.js')` resolves like Test262's host (E19.27).
pub fn run_js_in_node(js: &str) -> Result<(), String> {
    run_js_in_node_cwd(js, None, false)
}

/// Like [`run_js_in_node`], optionally with a working directory and ESM mode.
///
/// E19.28: `as_module` uses `--input-type=module` so top-level `await` is valid.
pub fn run_js_in_node_cwd(js: &str, cwd: Option<&Path>, as_module: bool) -> Result<(), String> {
    let mut cmd = Command::new("node");
    if as_module {
        cmd.arg("--input-type=module");
    }
    cmd.arg("-e").arg(js).stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }
    let output = cmd.output().map_err(|e| format!("spawn node: {e}"))?;
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
    let test_path = Some(full.as_path());
    if is_negative_parse(&source) {
        // Negative parse/early: pass iff frontend rejects the body.
        return match compile_test_to_js_at(&source, test_path) {
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
    let js = match compile_test_to_js_at(&source, test_path) {
        Ok(j) => j,
        Err(e) => {
            return CaseResult {
                path: rel.to_string(),
                status: Status::Fail,
                message: e,
            };
        }
    };
    // E19.26: async-flag tests need `$DONE` host (Node wrapper around emitted JS).
    let js = if is_async_flag(&source) {
        wrap_async_host(&js)
    } else {
        js
    };
    // E19.27: resolve relative dynamic `import()` against the test file directory.
    // E19.28: Module-flag tests run as ESM (`--input-type=module`) for top-level await.
    let cwd = full.parent();
    let as_module = is_module_flag(&source);
    if is_negative_runtime(&source) {
        // Negative runtime: pass iff Node throws (exit ≠ 0).
        return match run_js_in_node_cwd(&js, cwd, as_module) {
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
    match run_js_in_node_cwd(&js, cwd, as_module) {
        Ok(()) => CaseResult {
            path: rel.to_string(),
            status: Status::Pass,
            message: if is_async_flag(&source) && as_module {
                "ok (module async $DONE)".to_string()
            } else if is_async_flag(&source) {
                "ok (async $DONE)".to_string()
            } else if as_module {
                "ok (module)".to_string()
            } else {
                "ok".to_string()
            },
        },
        Err(e) => CaseResult {
            path: rel.to_string(),
            status: Status::Fail,
            message: e,
        },
    }
}

/// Parallelism for allowlist runs (`DRACONIC_TEST262_JOBS`, default = CPUs).
pub fn test262_jobs() -> usize {
    if let Ok(raw) = std::env::var("DRACONIC_TEST262_JOBS") {
        if let Ok(n) = raw.trim().parse::<usize>() {
            return n.max(1);
        }
    }
    std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4)
}

/// Run the curated allowlist. Suite absent → every case `skip`.
///
/// Cases run in parallel (see [`test262_jobs`]). Order matches the allowlist.
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
        let jobs = test262_jobs();
        let root = suite_root.to_path_buf();
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(jobs)
            .stack_size(8 * 1024 * 1024)
            .build()
            .expect("test262 rayon pool");
        pool.install(|| {
            allowlist
                .par_iter()
                .map(|p| run_case(&root, p))
                .collect()
        })
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
        // E19.02/E19.06/E19.10/E19.15/E19.20/E19.25–E19.56 expanded curated set.
        assert!(
            list.len() >= 37500,
            "expected expanded curated allowlist (>=37500), got {}",
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
    fn async_flag_meta_detected() {
        // E19.26: flags token `async`, not features like `async-functions`.
        assert!(is_async_flag(
            "/*---\nflags: [generated, async]\nfeatures: [async-functions]\n---*/\n1\n"
        ));
        assert!(is_async_flag("/*---\nflags: [async]\n---*/\n1\n"));
        assert!(!is_async_flag(
            "/*---\nfeatures: [async-functions, async-iteration]\n---*/\n1\n"
        ));
        assert!(!is_async_flag("/*---\nflags: [generated]\n---*/\n1\n"));
    }

    #[test]
    fn module_flag_meta_detected() {
        // E19.28: flags token `module`.
        assert!(is_module_flag(
            "/*---\nflags: [generated, module]\nfeatures: [top-level-await]\n---*/\n1\n"
        ));
        assert!(is_module_flag("/*---\nflags: [module, async]\n---*/\n1\n"));
        assert!(!is_module_flag(
            "/*---\nfeatures: [top-level-await]\n---*/\n1\n"
        ));
        assert!(!is_module_flag("/*---\nflags: [async]\n---*/\n1\n"));
        assert!(is_module_flag(
            "/*---\nflags:\n  - module\nnegative:\n  phase: parse\n---*/\n1\n"
        ));
    }

    #[test]
    fn top_level_await_module_compiles_and_runs() {
        // E19.28: Module goal + ESM host accepts top-level await.
        let src = r#"
/*---
description: top-level await basics
flags: [module, async]
features: [top-level-await]
---*/
var x = await 42;
assert.sameValue(x, 42);
$DONE();
"#;
        let js = compile_test_to_js(src).expect("compile module TLA");
        assert!(js.contains("await"), "{js}");
        let js = wrap_async_host(&js);
        run_js_in_node_cwd(&js, None, true).expect("node ESM TLA");
    }

    #[test]
    fn top_level_await_script_rejected() {
        // E19.28 / E19.52: Script [~Await] — `await` is IdentifierReference, so
        // `await 1` is a syntax error (not AwaitExpression).
        let src = "var x = await 1;\n";
        let err = compile_test_to_js(src).expect_err("script TLA must fail");
        assert!(!err.is_empty(), "unexpected empty err");
    }

    #[test]
    fn async_done_success_settles() {
        // E19.26: promise chain + $DONE() → pass (no ReferenceError).
        let src = r#"
/*---
description: async $DONE success
flags: [async]
---*/
Promise.resolve(1).then(function (v) {
  assert.sameValue(v, 1);
}).then($DONE, $DONE);
"#;
        let js = compile_test_to_js(src).expect("compile");
        let js = wrap_async_host(&js);
        run_js_in_node(&js).expect("async $DONE success");
    }

    #[test]
    fn async_done_failure_rejects() {
        // E19.26: $DONE(error) → node non-zero.
        let src = r#"
/*---
flags: [async]
---*/
Promise.resolve().then(function () {
  $DONE(new Error("boom"));
});
"#;
        let js = compile_test_to_js(src).expect("compile");
        let js = wrap_async_host(&js);
        assert!(run_js_in_node(&js).is_err());
    }

    #[test]
    fn async_done_missing_is_reference_error_without_host() {
        // Without wrap_async_host, $DONE is unresolved at runtime.
        let src = r#"
/*---
flags: [async]
---*/
Promise.resolve().then($DONE, $DONE);
"#;
        let js = compile_test_to_js(src).expect("compile");
        assert!(
            run_js_in_node(&js).is_err(),
            "bare $DONE without host must fail"
        );
    }

    #[test]
    fn dynamic_import_call_compiles_and_emits() {
        // E19.27: ImportCall round-trip through frontend → JS.
        let src = r#"
/*---
features: [dynamic-import]
---*/
let p = import('./m.js');
assert.sameValue(typeof p.then, "function");
"#;
        let js = compile_test_to_js(src).expect("compile");
        assert!(js.contains("import(\"./m.js\")"), "{js}");
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
        // Fast path (default): suite absent → skip-all green; suite present → do not
        // run the full allowlist (tens of k Node spawns). Set DRACONIC_TEST262_FULL=1
        // for the full gate (allowlist-expand Loops / pre-push).
        let root = resolve_suite_root();
        let list = load_allowlist(&allowlist_path()).expect("allowlist");
        if !suite_present(&root) {
            let report = run_allowlist(&root, &list);
            let path = write_baseline_report(&report).expect("write report");
            assert!(path.is_file(), "report path {}", path.display());
            let (pass, fail, skip) = report.counts();
            eprintln!(
                "test262 default (suite absent): pass={pass} fail={fail} skip={skip} report={}",
                path.display()
            );
            assert!(skip > 0);
            assert_eq!(fail, 0);
            assert_eq!(pass, 0);
            return;
        }

        let full = std::env::var_os("DRACONIC_TEST262_FULL").is_some();
        if !full {
            // Smoke: a handful of stable allowlisted paths (parallel still).
            let smoke: Vec<String> = list.iter().take(32).cloned().collect();
            let report = run_allowlist(&root, &smoke);
            let (pass, fail, skip) = report.counts();
            eprintln!(
                "test262 smoke (set DRACONIC_TEST262_FULL=1 for full allowlist): pass={pass} fail={fail} skip={skip} jobs={}",
                test262_jobs()
            );
            assert_eq!(fail, 0, "smoke allowlist must pass; failing: {:?}", report.cases.iter().filter(|c| c.status == Status::Fail).map(|c| &c.path).collect::<Vec<_>>());
            assert_eq!(skip, 0);
            assert_eq!(pass, smoke.len());
            return;
        }

        // Full allowlist gate (parallel). Larger stack for deep debug recursion.
        let handle = std::thread::Builder::new()
            .name("test262-default-run".into())
            .stack_size(32 * 1024 * 1024)
            .spawn(|| {
                let report = run_default().expect("run_default");
                let path = write_baseline_report(&report).expect("write report");
                assert!(path.is_file(), "report path {}", path.display());
                let (pass, fail, skip) = report.counts();
                eprintln!(
                    "test262 FULL: present={} pass={pass} fail={fail} skip={skip} jobs={} report={}",
                    report.suite_present,
                    test262_jobs(),
                    path.display()
                );
                assert_eq!(
                    fail, 0,
                    "allowlisted Test262 cases must pass (got fail={fail}); triage before expanding"
                );
                assert!(
                    pass >= 37500,
                    "expected expanded allowlist pass count >= 37500, got {pass}"
                );
            })
            .expect("spawn test262-default-run");
        handle.join().expect("test262-default-run thread");
    }
}
