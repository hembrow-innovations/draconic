//! Conformance harness: load fixtures, run on js + native runners (ROADMAP E00).

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use draconic_backend_js::emit_js;
use draconic_backend_llvm::{build_native_binary, emit_llvm_ir};
use draconic_check::check;
use draconic_ir::lower;
use draconic_parser::{link_entry, parse};

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
    /// When set, compile/emit must fail and the diagnostic message must contain this substring.
    /// Used for native-only features on the JS target (N04).
    pub error_contains: Option<String>,
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

fn collect_drac(dir: &Path, out: &mut Vec<PathBuf>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("read_dir {}: {e}", dir.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| format!("read_dir entry: {e}"))?;
        let path = entry.path();
        if path.is_dir() {
            collect_drac(&path, out)?;
        } else if path.extension().and_then(|e| e.to_str()) == Some("drac") {
            // Only entry fixtures have a `.meta` sidecar. Dependency modules
            // (imported by entries) are plain `.drac` without meta.
            let meta = path.with_extension("meta");
            if meta.is_file() {
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
}

impl Meta {
    fn default_for(source_path: &Path) -> Self {
        let mut m = Meta {
            id: Some(default_id(source_path)),
            targets: vec![Target::Js, Target::Native],
            ..Meta::default()
        };
        m.expect_js.exit = 0;
        m.expect_native.exit = 0;
        m.expect_native.stdout = Some("hello\n".to_string());
        m
    }
}

/// Line-oriented sidecar:
/// ```text
/// id: smoke/let-add
/// targets: js,native
/// js.exit: 0
/// js.check: if (x !== 3) process.exit(1);
/// native.exit: 0
/// native.stdout: hello\n
/// ```
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
            "js.error" => meta.expect_js.error_contains = Some(unescape(value)),
            "native.exit" => {
                meta.expect_native.exit = parse_exit(value, lineno + 1)?;
            }
            "native.stdout" => meta.expect_native.stdout = Some(unescape(value)),
            "native.error" => meta.expect_native.error_contains = Some(unescape(value)),
            "native.check" => {
                return Err(format!(
                    "meta line {}: native.check is not supported",
                    lineno + 1
                ));
            }
            other => {
                return Err(format!(
                    "meta line {}: unknown key `{other}`",
                    lineno + 1
                ));
            }
        }
    }

    if meta.expect_native.stdout.is_none() {
        meta.expect_native.stdout = Some("hello\n".to_string());
    }
    Ok(meta)
}

fn parse_exit(value: &str, line: usize) -> Result<i32, String> {
    value
        .parse()
        .map_err(|_| format!("meta line {line}: invalid exit code `{value}`"))
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

/// Compile a fixture entry through the Frontend + IR (links static imports).
fn compile_module(source_path: &Path, source: &str) -> Result<draconic_ir::Module, String> {
    let program = if source.contains("import ") || source.contains("export ") {
        link_entry(source_path).map_err(|d| format!("link: {d}"))?
    } else {
        parse(source).map_err(|d| format!("parse: {d}"))?
    };
    let checked = check(program).map_err(|d| format!("check: {d}"))?;
    Ok(lower(&checked))
}

/// Run one fixture on one target.
pub fn run_fixture_target(fixture: &Fixture, target: Target) -> RunResult {
    let result = match target {
        Target::Js => run_js(fixture),
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
    fixture
        .targets
        .iter()
        .copied()
        .map(|t| run_fixture_target(fixture, t))
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

fn run_js(fixture: &Fixture) -> Result<(), String> {
    let expect = &fixture.expect_js;
    if let Some(needle) = &expect.error_contains {
        return expect_compile_or_emit_error(fixture, Target::Js, needle);
    }

    let module = compile_module(&fixture.source_path, &fixture.source)?;
    let js = emit_js(&module).map_err(|d| format!("emit_js: {d}"))?;

    let script = if let Some(check) = &expect.check {
        format!("{js}\n{check}")
    } else {
        js
    };

    let output = Command::new("node")
        .arg("-e")
        .arg(&script)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| format!("spawn node: {e}"))?;

    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

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
    Ok(())
}

fn run_native(fixture: &Fixture) -> Result<(), String> {
    let expect = &fixture.expect_native;
    if let Some(needle) = &expect.error_contains {
        return expect_compile_or_emit_error(fixture, Target::Native, needle);
    }

    let module = compile_module(&fixture.source_path, &fixture.source)?;
    let ll = emit_llvm_ir(&module).map_err(|d| format!("emit_llvm_ir: {d}"))?;
    let out = temp_bin_path(&fixture.id);
    build_native_binary(&ll, &out).map_err(|d| format!("build_native_binary: {d}"))?;

    let output = Command::new(&out)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
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
    Ok(())
}

/// Expect frontend or backend emit to fail with a message containing `needle`.
fn expect_compile_or_emit_error(
    fixture: &Fixture,
    target: Target,
    needle: &str,
) -> Result<(), String> {
    let module = match compile_module(&fixture.source_path, &fixture.source) {
        Ok(m) => m,
        Err(msg) => {
            if msg.contains(needle) {
                return Ok(());
            }
            return Err(format!(
                "{} compile error did not contain {needle:?}\ngot: {msg}",
                target.as_str()
            ));
        }
    };
    let err = match target {
        Target::Js => emit_js(&module).err().map(|d| format!("emit_js: {d}")),
        Target::Native => emit_llvm_ir(&module)
            .err()
            .map(|d| format!("emit_llvm_ir: {d}")),
    };
    match err {
        Some(msg) if msg.contains(needle) => Ok(()),
        Some(msg) => Err(format!(
            "{} emit error did not contain {needle:?}\ngot: {msg}",
            target.as_str()
        )),
        None => Err(format!(
            "{} expected emit/compile error containing {needle:?}, but succeeded",
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
targets: js,native
js.exit: 0
js.check: if (x !== 3) process.exit(1);
native.exit: 0
native.stdout: hello\\n
",
        )
        .unwrap();
        assert_eq!(meta.id.as_deref(), Some("smoke/let-add"));
        assert_eq!(meta.targets, vec![Target::Js, Target::Native]);
        assert_eq!(meta.expect_js.exit, 0);
        assert_eq!(
            meta.expect_js.check.as_deref(),
            Some("if (x !== 3) process.exit(1);")
        );
        assert_eq!(meta.expect_native.stdout.as_deref(), Some("hello\n"));
    }

    #[test]
    fn unescape_newlines() {
        assert_eq!(unescape(r"hello\n"), "hello\n");
    }
}
