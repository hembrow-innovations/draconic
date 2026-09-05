//! Conformance harness: load fixtures, run on js + native runners (ROADMAP E00).

mod coverage;

pub use coverage::CoverageReport;

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_js::{emit_js, emit_js_with_map, SourceMapOptions};
use draconic_backend_llvm::{
    build_c_dynamic_lib, build_c_static_lib, build_native_binary,
    build_native_binary_with_dynamic_libs, build_native_binary_with_static_libs, emit_llvm_ir,
};
use draconic_diagnostics::SourceFile;
use draconic_frontend::compile_path;

use coverage::{instrument_js, read_hits, temp_cov_path, wrap_coverage_dump};

/// Backend a fixture may target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Target {
    Js,
    Native,
}

impl Target {
    pub fn as_str(self) -> &'static str {
        match self {
            Target::Js => "js",
            Target::Native => "native",
        }
    }

    fn parse(s: &str) -> Result<Self, String> {
        match s.trim() {
            "js" => Ok(Target::Js),
            "native" => Ok(Target::Native),
            other => Err(format!("unknown target `{other}`")),
        }
    }
}

/// Per-target expectations for a fixture.
#[derive(Debug, Clone, Default)]
pub struct TargetExpect {
    pub exit: i32,
    /// Appended after emitted JS and executed under Node (js only).
    pub check: Option<String>,
    /// Exact stdout (native; optional for js).
    pub stdout: Option<String>,
    /// Exact stderr (optional; H02.02 stderr write).
    pub stderr: Option<String>,
    /// When set, compile/emit must fail and the diagnostic message must contain this substring.
    /// Used for native-only features on the JS target (N04).
    pub error_contains: Option<String>,
    /// When set, compile/emit must fail and the diagnostic string must contain this code
    /// (e.g. `E0300`). Additive with `error_contains` (U09).
    pub error_code: Option<String>,
    /// Program user args forwarded to the runner (H01.01 `processArgs`).
    pub args: Vec<String>,
    /// Bytes written to the process stdin before run (H02.03).
    pub stdin: Option<String>,
    /// Extra static archives or C sources to compile to `.a` (F04.01 `native.link`).
    pub link: Vec<PathBuf>,
    /// Extra shared libraries or C sources to compile to `.so`/`.dylib`/`.dll` (F05.01 `native.dylink`).
    pub dylink: Vec<PathBuf>,
}

/// One conformance fixture loaded from disk.
#[derive(Debug, Clone)]
pub struct Fixture {
    pub id: String,
    pub source_path: PathBuf,
    pub source: String,
    pub targets: Vec<Target>,
    pub expect_js: TargetExpect,
    pub expect_native: TargetExpect,
    /// R02.01 explicit permission grant subset (`grants: fs-read,fs-write`).
    /// Empty means no grant subset (R02.04 permissive default).
    pub grants: Vec<String>,
}

/// Outcome of running one fixture on one target.
#[derive(Debug)]
pub struct RunResult {
    pub fixture_id: String,
    pub target: Target,
    pub ok: bool,
    pub message: String,
}

/// Directory that contains `fixtures/` (the conformance package root).
/// L10.01 HMAC-SHA256 and L10.02 AEAD fixtures live under `fixtures/stdlib/crypto/`.
pub fn package_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Default fixtures directory: `<package>/fixtures`.
pub fn fixtures_dir() -> PathBuf {
    package_root().join("fixtures")
}

/// Discover `*.drac` fixtures under `root` (recursive) with optional `*.meta` sidecars.
pub fn load_fixtures(root: &Path) -> Result<Vec<Fixture>, String> {
    if !root.is_dir() {
        return Err(format!("fixtures root missing: {}", root.display()));
    }
    let mut paths = Vec::new();
    collect_drac(root, &mut paths)?;
    paths.sort();
    let mut out = Vec::with_capacity(paths.len());
    for path in paths {
        out.push(load_fixture(&path)?);
    }
    Ok(out)
}

/// Load fixtures from a directory (recursive) or a single `.drac` entry file.
pub fn load_path(path: &Path) -> Result<Vec<Fixture>, String> {
    if path.is_file() {
        if path.extension().and_then(|e| e.to_str()) != Some("drac") {
            return Err(format!(
                "expected a .drac fixture file, got {}",
                path.display()
            ));
        }
        return Ok(vec![load_fixture(path)?]);
    }
    if path.is_dir() {
        return load_fixtures(path);
    }
    Err(format!("path not found: {}", path.display()))
}

/// L05.04: a `.drac` without `.meta` that registers `describe`/`it` is a suite entry.
fn looks_like_in_language_suite(path: &Path) -> bool {
    let Ok(src) = fs::read_to_string(path) else {
        return false;
    };
    src.contains("describe(") || src.contains("describe (")
}

fn collect_drac(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read_dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_drac(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("drac") {
            // Entry fixtures have a `.meta` sidecar. Dependency modules are
            // plain `.drac` without meta. In-language `describe`/`it` suites
            // also lack meta (L05.04) and must still run under `draconic test`.
            let meta = path.with_extension("meta");
            if meta.is_file() || looks_like_in_language_suite(&path) {
                out.push(path);
            }
        }
    }
    Ok(())
}

fn load_fixture(source_path: &Path) -> Result<Fixture, String> {
    let source = fs::read_to_string(source_path)
        .map_err(|e| format!("read {}: {e}", source_path.display()))?;
    let meta_path = source_path.with_extension("meta");
    let meta = if meta_path.is_file() {
        let text = fs::read_to_string(&meta_path)
            .map_err(|e| format!("read {}: {e}", meta_path.display()))?;
        parse_meta(&text)?
    } else {
        Meta::default_for(source_path)
    };

    let id = meta.id.unwrap_or_else(|| default_id(source_path));
    let targets = if meta.targets.is_empty() {
        vec![Target::Js, Target::Native]
    } else {
        meta.targets
    };

    Ok(Fixture {
        id,
        source_path: source_path.to_path_buf(),
        source,
        targets,
        expect_js: meta.expect_js,
        expect_native: meta.expect_native,
        grants: meta.grants,
    })
}

fn default_id(source_path: &Path) -> String {
    source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unnamed")
        .to_string()
}

#[derive(Debug, Default)]
struct Meta {
    id: Option<String>,
    targets: Vec<Target>,
    expect_js: TargetExpect,
    expect_native: TargetExpect,
    grants: Vec<String>,
}

impl Meta {
    fn default_for(source_path: &Path) -> Self {
        let mut m = Meta {
            id: Some(default_id(source_path)),
            targets: vec![Target::Js],
            ..Meta::default()
        };
        m.expect_js.exit = 0;
        m
    }
}

/// Line-oriented sidecar:
/// ```text
/// id: smoke/let-add
/// targets: js
/// js.exit: 0
/// js.check: if (x !== 3) process.exit(1);
/// ```
///
/// Native expectations are only used when `targets` includes `native`. Do not
/// default `native.stdout` to the B08 hello stub — stub-hello is not feature coverage.
fn parse_meta(text: &str) -> Result<Meta, String> {
    let mut meta = Meta::default();
    meta.expect_js.exit = 0;
    meta.expect_native.exit = 0;

    for (lineno, raw) in text.lines().enumerate() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let (key, value) = line
            .split_once(':')
            .ok_or_else(|| format!("meta line {}: expected `key: value`", lineno + 1))?;
        let key = key.trim();
        let value = value.trim();
        match key {
            "id" => meta.id = Some(value.to_string()),
            "targets" => {
                meta.targets = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(Target::parse)
                    .collect::<Result<Vec<_>, _>>()?;
            }
            "js.exit" => {
                meta.expect_js.exit = parse_exit(value, lineno + 1)?;
            }
            "js.check" => meta.expect_js.check = Some(unescape(value)),
            "js.stdout" => meta.expect_js.stdout = Some(unescape(value)),
            "js.stderr" => meta.expect_js.stderr = Some(unescape(value)),
            "js.error" => meta.expect_js.error_contains = Some(unescape(value)),
            "js.error_code" => meta.expect_js.error_code = Some(value.to_string()),
            "js.args" => meta.expect_js.args = parse_args(value),
            "js.stdin" => meta.expect_js.stdin = Some(unescape(value)),
            "native.exit" => {
                meta.expect_native.exit = parse_exit(value, lineno + 1)?;
            }
            "native.stdout" => meta.expect_native.stdout = Some(unescape(value)),
            "native.stderr" => meta.expect_native.stderr = Some(unescape(value)),
            "native.error" => meta.expect_native.error_contains = Some(unescape(value)),
            "native.error_code" => meta.expect_native.error_code = Some(value.to_string()),
            "native.args" => meta.expect_native.args = parse_args(value),
            "native.stdin" => meta.expect_native.stdin = Some(unescape(value)),
            "native.link" => {
                if value.is_empty() {
                    return Err(format!(
                        "meta line {}: native.link requires a path",
                        lineno + 1
                    ));
                }
                meta.expect_native.link.push(PathBuf::from(value));
            }
            "native.dylink" => {
                if value.is_empty() {
                    return Err(format!(
                        "meta line {}: native.dylink requires a path",
                        lineno + 1
                    ));
                }
                meta.expect_native.dylink.push(PathBuf::from(value));
            }
            // Shared args for both targets (H01.01).
            "args" => {
                let a = parse_args(value);
                meta.expect_js.args = a.clone();
                meta.expect_native.args = a;
            }
            // Shared stdin for both targets (H02.03).
            "stdin" => {
                let s = unescape(value);
                meta.expect_js.stdin = Some(s.clone());
                meta.expect_native.stdin = Some(s);
            }
            // R02.01 explicit permission grant subset (both targets).
            "grants" => {
                meta.grants = value
                    .split(',')
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect();
            }
            "native.check" => {
                return Err(format!(
                    "meta line {}: native.check is not supported",
                    lineno + 1
                ));
            }
            other => {
                return Err(format!("meta line {}: unknown key `{other}`", lineno + 1));
            }
        }
    }

    Ok(meta)
}

fn parse_exit(value: &str, line: usize) -> Result<i32, String> {
    value
        .parse()
        .map_err(|_| format!("meta line {line}: invalid exit code `{value}`"))
}

/// R02.01: forward an explicit grant subset as `DRACONIC_PERMISSIONS`.
/// Empty grants leave the process permissive (R02.04): unset the env so a
/// parent lock-down cannot leak into default-policy fixtures.
fn apply_permission_grants(cmd: &mut Command, grants: &[String]) {
    if grants.is_empty() {
        cmd.env_remove("DRACONIC_PERMISSIONS");
    } else {
        cmd.env("DRACONIC_PERMISSIONS", grants.join(","));
    }
}

/// Whitespace-separated program args (`args: alpha beta`).
fn parse_args(value: &str) -> Vec<String> {
    value
        .split_whitespace()
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect()
}

fn unescape(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('r') => out.push('\r'),
                Some('t') => out.push('\t'),
                Some('0') => out.push('\0'),
                Some('\\') => out.push('\\'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Compile a fixture entry through the Frontend (links static imports when needed).
fn compile_module(source_path: &Path, source: &str) -> Result<draconic_frontend::Module, String> {
    compile_path(source_path).map_err(|d| {
        let name = source_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("program.drac");
        format!("compile: {}", d.pretty(&SourceFile::new(name, source)))
    })
}

/// Run one fixture on one target.
pub fn run_fixture_target(fixture: &Fixture, target: Target) -> RunResult {
    run_fixture_target_cov(fixture, target, None)
}

/// Run one fixture on one target, optionally collecting JS line coverage (U11).
pub fn run_fixture_target_cov(
    fixture: &Fixture,
    target: Target,
    coverage: Option<&mut CoverageReport>,
) -> RunResult {
    let result = match target {
        Target::Js => run_js(fixture, coverage),
        Target::Native => run_native(fixture),
    };
    match result {
        Ok(()) => RunResult {
            fixture_id: fixture.id.clone(),
            target,
            ok: true,
            message: "ok".to_string(),
        },
        Err(message) => RunResult {
            fixture_id: fixture.id.clone(),
            target,
            ok: false,
            message,
        },
    }
}

/// Run every declared target for a fixture.
pub fn run_fixture(fixture: &Fixture) -> Vec<RunResult> {
    run_fixture_cov(fixture, None)
}

/// Run every declared target; when `coverage` is `Some`, collect JS line hits (U11).
pub fn run_fixture_cov(
    fixture: &Fixture,
    mut coverage: Option<&mut CoverageReport>,
) -> Vec<RunResult> {
    fixture
        .targets
        .iter()
        .copied()
        .map(|t| {
            let cov = coverage.as_deref_mut();
            run_fixture_target_cov(fixture, t, cov)
        })
        .collect()
}

/// Load all fixtures under `root` and run each on its targets.
pub fn run_all(root: &Path) -> Result<Vec<RunResult>, String> {
    let fixtures = load_fixtures(root)?;
    if fixtures.is_empty() {
        return Err(format!("no .drac fixtures under {}", root.display()));
    }
    let mut results = Vec::new();
    for fixture in &fixtures {
        results.extend(run_fixture(fixture));
    }
    Ok(results)
}

fn run_js(fixture: &Fixture, coverage: Option<&mut CoverageReport>) -> Result<(), String> {
    let expect = &fixture.expect_js;
    if expect.error_contains.is_some() || expect.error_code.is_some() {
        return expect_compile_or_emit_error(
            fixture,
            Target::Js,
            expect.error_contains.as_deref(),
            expect.error_code.as_deref(),
        );
    }

    let module = compile_module(&fixture.source_path, &fixture.source)?;

    let (js_body, executable) = if coverage.is_some() {
        let source_name = fixture
            .source_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("program.drac");
        let opts = SourceMapOptions::new(source_name).with_content(&fixture.source);
        let emitted =
            emit_js_with_map(&module, &opts).map_err(|d| format!("emit_js_with_map: {d}"))?;
        if let Some(map) = &emitted.map {
            instrument_js(&emitted.code, map)
        } else {
            (emitted.code, Default::default())
        }
    } else {
        let js = emit_js(&module).map_err(|d| format!("emit_js: {d}"))?;
        (js, Default::default())
    };

    let script = if let Some(check) = &expect.check {
        format!("{js_body}\n{check}")
    } else {
        js_body
    };

    let cov_path = if coverage.is_some() {
        Some(temp_cov_path(&fixture.id))
    } else {
        None
    };
    let script = if let Some(path) = &cov_path {
        wrap_coverage_dump(&script, path)
    } else {
        script
    };

    let mut node = Command::new("node");
    node.arg("-e").arg(&script).args(&expect.args);
    apply_permission_grants(&mut node, &fixture.grants);
    let output = run_with_optional_stdin(&mut node, expect.stdin.as_deref())
        .map_err(|e| format!("spawn node: {e}"))?;

    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if let (Some(report), Some(path)) = (coverage, &cov_path) {
        let hit = read_hits(path);
        let display = fixture.source_path.display().to_string();
        report.merge_file(&display, executable, hit);
        let _ = fs::remove_file(path);
    }

    if code != expect.exit {
        return Err(format!(
            "js exit {code}, want {}\n--- script ---\n{script}\n--- stdout ---\n{stdout}\n--- stderr ---\n{stderr}",
            expect.exit
        ));
    }
    if let Some(want) = &expect.stdout {
        if stdout.as_ref() != want {
            return Err(format!(
                "js stdout mismatch\nwant: {want:?}\ngot:  {:?}\nstderr: {stderr}",
                stdout.as_ref()
            ));
        }
    }
    if let Some(want) = &expect.stderr {
        if stderr.as_ref() != want {
            return Err(format!(
                "js stderr mismatch\nwant: {want:?}\ngot:  {:?}\nstdout: {stdout}",
                stderr.as_ref()
            ));
        }
    }
    Ok(())
}

/// Run `cmd` with optional stdin bytes; stdout/stderr always piped.
fn run_with_optional_stdin(cmd: &mut Command, stdin: Option<&str>) -> Result<Output, String> {
    if stdin.is_some() {
        cmd.stdin(Stdio::piped());
    } else {
        cmd.stdin(Stdio::null());
    }
    cmd.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = cmd.spawn().map_err(|e| format!("spawn: {e}"))?;
    if let Some(data) = stdin {
        if let Some(mut sin) = child.stdin.take() {
            sin.write_all(data.as_bytes())
                .map_err(|e| format!("write stdin: {e}"))?;
        }
    }
    child.wait_with_output().map_err(|e| format!("wait: {e}"))
}

/// F04.01: `native.link` paths are relative to the fixture directory.
/// `.c` sources are compiled to a temp `.a`; `.a` paths are used as-is.
fn resolve_native_link_libs(fixture: &Fixture) -> Result<Vec<PathBuf>, String> {
    if fixture.expect_native.link.is_empty() {
        return Ok(Vec::new());
    }
    let base = fixture
        .source_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let mut out = Vec::new();
    for (i, rel) in fixture.expect_native.link.iter().enumerate() {
        let path = if rel.is_absolute() {
            rel.clone()
        } else {
            base.join(rel)
        };
        if !path.is_file() {
            return Err(format!("native.link not found: {}", path.display()));
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if ext == "a" {
            out.push(path);
            continue;
        }
        if ext != "c" {
            return Err(format!(
                "native.link must be a .c or .a file, got {}",
                path.display()
            ));
        }
        let archive = temp_bin_path(&format!("{}-link-{i}", fixture.id)).with_extension("a");
        if let Some(parent) = archive.parent() {
            let _ = fs::create_dir_all(parent);
        }
        build_c_static_lib(&path, &archive)
            .map_err(|d| format!("build static lib from {}: {d}", path.display()))?;
        out.push(archive);
    }
    Ok(out)
}

/// F05.01: `native.dylink` paths are relative to the fixture directory.
/// `.c` sources are compiled to a temp shared lib; `.so`/`.dylib`/`.dll` used as-is.
fn resolve_native_dylink_libs(fixture: &Fixture) -> Result<Vec<PathBuf>, String> {
    if fixture.expect_native.dylink.is_empty() {
        return Ok(Vec::new());
    }
    let base = fixture
        .source_path
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let mut out = Vec::new();
    for (i, rel) in fixture.expect_native.dylink.iter().enumerate() {
        let path = if rel.is_absolute() {
            rel.clone()
        } else {
            base.join(rel)
        };
        if !path.is_file() {
            return Err(format!("native.dylink not found: {}", path.display()));
        }
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        if matches!(ext, "so" | "dylib" | "dll") {
            out.push(path);
            continue;
        }
        if ext != "c" {
            return Err(format!(
                "native.dylink must be a .c or shared lib, got {}",
                path.display()
            ));
        }
        let dylib =
            temp_bin_path(&format!("{}-dylink-{i}", fixture.id)).with_extension(dynamic_lib_ext());
        if let Some(parent) = dylib.parent() {
            let _ = fs::create_dir_all(parent);
        }
        build_c_dynamic_lib(&path, &dylib)
            .map_err(|d| format!("build dynamic lib from {}: {d}", path.display()))?;
        out.push(dylib);
    }
    Ok(out)
}

fn dynamic_lib_ext() -> &'static str {
    if cfg!(target_os = "windows") {
        "dll"
    } else if cfg!(target_os = "macos") {
        "dylib"
    } else {
        "so"
    }
}

fn run_native(fixture: &Fixture) -> Result<(), String> {
    let expect = &fixture.expect_native;
    if expect.error_contains.is_some() || expect.error_code.is_some() {
        return expect_compile_or_emit_error(
            fixture,
            Target::Native,
            expect.error_contains.as_deref(),
            expect.error_code.as_deref(),
        );
    }

    let module = compile_module(&fixture.source_path, &fixture.source)?;
    let ll = emit_llvm_ir(&module).map_err(|d| format!("emit_llvm_ir: {d}"))?;
    let out = temp_bin_path(&fixture.id);
    let extra_libs = resolve_native_link_libs(fixture)?;
    let extra_dylibs = resolve_native_dylink_libs(fixture)?;
    if extra_dylibs.is_empty() {
        if extra_libs.is_empty() {
            build_native_binary(&ll, &out).map_err(|d| format!("build_native_binary: {d}"))?;
        } else {
            build_native_binary_with_static_libs(&ll, &out, &extra_libs)
                .map_err(|d| format!("build_native_binary: {d}"))?;
        }
    } else if extra_libs.is_empty() {
        build_native_binary_with_dynamic_libs(&ll, &out, &extra_dylibs)
            .map_err(|d| format!("build_native_binary: {d}"))?;
    } else {
        return Err("native.link and native.dylink together are not supported".to_string());
    }

    let mut native_cmd = Command::new(&out);
    native_cmd.args(&expect.args);
    apply_permission_grants(&mut native_cmd, &fixture.grants);
    let output = run_with_optional_stdin(&mut native_cmd, expect.stdin.as_deref())
        .map_err(|e| format!("run native binary: {e}"))?;

    let _ = fs::remove_file(&out);

    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    if code != expect.exit {
        return Err(format!(
            "native exit {code}, want {}\nstdout: {stdout}\nstderr: {stderr}",
            expect.exit
        ));
    }
    if let Some(want) = &expect.stdout {
        if stdout.as_ref() != want {
            return Err(format!(
                "native stdout mismatch\nwant: {want:?}\ngot:  {:?}\nstderr: {stderr}",
                stdout.as_ref()
            ));
        }
    }
    if let Some(want) = &expect.stderr {
        if stderr.as_ref() != want {
            return Err(format!(
                "native stderr mismatch\nwant: {want:?}\ngot:  {:?}\nstdout: {stdout}",
                stderr.as_ref()
            ));
        }
    }
    Ok(())
}

/// Expect frontend or backend emit to fail.
///
/// `message_needle` (from `js.error` / `native.error`) and `code_needle` (from
/// `js.error_code` / `native.error_code`) are optional; when set, the error
/// string must contain each. At least one should be set by the caller.
fn expect_compile_or_emit_error(
    fixture: &Fixture,
    target: Target,
    message_needle: Option<&str>,
    code_needle: Option<&str>,
) -> Result<(), String> {
    let match_needles = |msg: &str| -> Result<(), String> {
        if let Some(needle) = message_needle {
            if !msg.contains(needle) {
                return Err(format!(
                    "{} error did not contain message {needle:?}\ngot: {msg}",
                    target.as_str()
                ));
            }
        }
        if let Some(code) = code_needle {
            if !msg.contains(code) {
                return Err(format!(
                    "{} error did not contain code {code:?}\ngot: {msg}",
                    target.as_str()
                ));
            }
        }
        Ok(())
    };

    let module = match compile_module(&fixture.source_path, &fixture.source) {
        Ok(m) => m,
        Err(msg) => {
            return match_needles(&msg);
        }
    };
    let err = match target {
        Target::Js => emit_js(&module).err().map(|d| format!("emit_js: {d}")),
        Target::Native => emit_llvm_ir(&module)
            .err()
            .map(|d| format!("emit_llvm_ir: {d}")),
    };
    match err {
        Some(msg) => match_needles(&msg),
        None => Err(format!(
            "{} expected emit/compile error (message={message_needle:?}, code={code_needle:?}), but succeeded",
            target.as_str()
        )),
    }
}

fn temp_bin_path(id: &str) -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let safe: String = id
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect();
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-conformance-{}-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        N.fetch_add(1, Ordering::Relaxed),
        safe
    ));
    let _ = fs::create_dir_all(&dir);
    dir.join("prog")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_meta_smoke() {
        let meta = parse_meta(
            "\
id: smoke/let-add
targets: js
js.exit: 0
js.check: if (x !== 3) process.exit(1);
",
        )
        .unwrap();
        assert_eq!(meta.id.as_deref(), Some("smoke/let-add"));
        assert_eq!(meta.targets, vec![Target::Js]);
        assert_eq!(meta.expect_js.exit, 0);
        assert_eq!(
            meta.expect_js.check.as_deref(),
            Some("if (x !== 3) process.exit(1);")
        );
        assert!(meta.expect_native.stdout.is_none());
    }

    #[test]
    fn parse_meta_native_real_stdout() {
        let meta = parse_meta(
            "\
id: native/ints/arith_i32
targets: native
native.exit: 0
native.stdout: 10\\n3\\n13\\n
",
        )
        .unwrap();
        assert_eq!(meta.targets, vec![Target::Native]);
        assert_eq!(meta.expect_native.stdout.as_deref(), Some("10\n3\n13\n"));
    }

    #[test]
    fn unescape_newlines() {
        assert_eq!(unescape(r"hello\n"), "hello\n");
    }

    #[test]
    fn parse_meta_error_code() {
        let meta = parse_meta(
            "\
id: types/reject/call_type_mismatch
targets: js
js.error: not assignable
js.error_code: E0300
",
        )
        .unwrap();
        assert_eq!(
            meta.expect_js.error_contains.as_deref(),
            Some("not assignable")
        );
        assert_eq!(meta.expect_js.error_code.as_deref(), Some("E0300"));
    }

    #[test]
    fn parse_meta_native_link() {
        let meta = parse_meta(
            "\
id: ffi/link_static/resolve
targets: native
native.link: resolve.c
native.exit: 0
",
        )
        .unwrap();
        assert_eq!(meta.expect_native.link, vec![PathBuf::from("resolve.c")]);
    }

    #[test]
    fn load_path_dir_includes_in_language_suite_without_meta() {
        let dir = temp_bin_path("l0504-load").parent().unwrap().to_path_buf();
        let _ = fs::create_dir_all(&dir);
        fs::write(dir.join("smoke.drac"), "let x = 1;\n").unwrap();
        fs::write(
            dir.join("smoke.meta"),
            "id: smoke\ntargets: js\njs.exit: 0\n",
        )
        .unwrap();
        fs::write(
            dir.join("suite.drac"),
            "describe(\"s\", () => { it(\"t\", () => {}); });\n",
        )
        .unwrap();
        fs::write(dir.join("dep.drac"), "export let n = 1;\n").unwrap();

        let fixtures = load_path(&dir).expect("load");
        let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
        assert!(ids.contains(&"smoke"), "got {ids:?}");
        assert!(ids.contains(&"suite"), "got {ids:?}");
        assert!(
            !ids.contains(&"dep"),
            "dependency module should stay excluded, got {ids:?}"
        );
    }

    #[test]
    fn parse_meta_native_dylink() {
        let meta = parse_meta(
            "\
id: ffi/link_dynamic/resolve
targets: native
native.dylink: resolve.c
native.exit: 0
",
        )
        .unwrap();
        assert_eq!(meta.expect_native.dylink, vec![PathBuf::from("resolve.c")]);
    }

    #[test]
    fn parse_meta_grants() {
        let meta = parse_meta(
            "\
id: security/permissions/grant_fs
targets: js,native
grants: fs-read, fs-write
js.exit: 0
native.exit: 0
",
        )
        .unwrap();
        assert_eq!(
            meta.grants,
            vec!["fs-read".to_string(), "fs-write".to_string()]
        );
    }
}
