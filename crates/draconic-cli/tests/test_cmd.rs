//! ROADMAP U01: `draconic test` runner integration.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

fn draconic() -> Command {
    Command::new(env!("CARGO_BIN_EXE_draconic"))
}

fn temp_dir() -> PathBuf {
    static N: AtomicU64 = AtomicU64::new(0);
    let mut dir = std::env::temp_dir();
    dir.push(format!(
        "draconic-cli-test-{}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn write(dir: &Path, name: &str, contents: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, contents).unwrap();
    path
}

fn run(cmd: &mut Command) -> (i32, String, String) {
    let output = cmd
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn draconic");
    let code = output.status.code().unwrap_or(1);
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    (code, stdout, stderr)
}

#[test]
fn help_lists_test_command() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("draconic test") || stdout.contains("test "),
        "help should list test:\n{stdout}"
    );
}

#[test]
fn test_missing_path_exits_usage() {
    let (code, _stdout, stderr) = run(draconic().arg("test"));
    assert_eq!(code, 2, "stderr={stderr}");
    assert!(
        stderr.contains("usage") || stderr.contains("test"),
        "stderr={stderr}"
    );
}

#[test]
fn test_runs_passing_fixture_dir() {
    let dir = temp_dir();
    write(&dir, "smoke.drac", "let x = 1 + 2;\n");
    write(
        &dir,
        "smoke.meta",
        "\
id: smoke
targets: js
js.exit: 0
js.check: if (x !== 3) process.exit(1);
",
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("ok") || stdout.contains("passed"),
        "stdout={stdout}"
    );
    assert!(
        stdout.contains("smoke") || stdout.contains("js") || stdout.contains("native"),
        "stdout={stdout}"
    );
}

#[test]
fn test_fails_when_js_check_fails() {
    let dir = temp_dir();
    write(&dir, "bad.drac", "let x = 1;\n");
    write(
        &dir,
        "bad.meta",
        "\
id: bad
targets: js
js.exit: 0
js.check: if (x !== 99) process.exit(1);
",
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir));
    assert_ne!(
        code, 0,
        "expected failure\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("FAIL")
            || stdout.contains("fail")
            || stderr.contains("FAIL")
            || stderr.contains("fail")
            || stdout.contains("bad"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

#[test]
fn test_runs_single_fixture_file() {
    let dir = temp_dir();
    let src = write(&dir, "one.drac", "let n = 0;\n");
    write(
        &dir,
        "one.meta",
        "\
id: one
targets: js
js.exit: 0
js.check: if (n !== 0) process.exit(1);
",
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("ok") || stdout.contains("passed") || stdout.contains("one"),
        "stdout={stdout}"
    );
}

#[test]
fn test_missing_path_reports_error() {
    let missing = temp_dir().join("does-not-exist");
    let (code, _stdout, stderr) = run(draconic().arg("test").arg(&missing));
    assert_ne!(code, 0, "stderr={stderr}");
    assert!(
        stderr.contains("error") || stderr.contains("missing") || stderr.contains("not"),
        "stderr={stderr}"
    );
}

/// ROADMAP U11: `draconic test --coverage` reports JS line coverage.
#[test]
fn test_coverage_reports_line_hits() {
    let dir = temp_dir();
    write(&dir, "cov.drac", "let a = 1;\nlet b = 2;\nlet c = a + b;\n");
    write(
        &dir,
        "cov.meta",
        "\
id: cov
targets: js
js.exit: 0
js.check: if (c !== 3) process.exit(1);
",
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg("--coverage").arg(&dir));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("coverage"),
        "expected coverage section:\n{stdout}"
    );
    assert!(
        stdout.contains("lines") && (stdout.contains('%') || stdout.contains("/")),
        "expected line counts:\n{stdout}"
    );
    assert!(
        stdout.contains("cov.drac") || stdout.contains("total:"),
        "expected file or total line:\n{stdout}"
    );
    // Fully executed straight-line program should hit at least one line.
    assert!(
        !stdout.contains("0/0 lines") || stdout.contains("total:"),
        "stdout={stdout}"
    );
    let total_ok = stdout.lines().any(|l| {
        l.starts_with("total:") && l.contains("lines") && !l.contains("0/0") && !l.contains("0/")
    }) || stdout
        .lines()
        .any(|l| l.contains("lines") && l.contains('%') && !l.contains("0%"));
    assert!(total_ok, "expected non-zero coverage hits:\n{stdout}");
}

#[test]
fn test_coverage_flag_order_flexible() {
    let dir = temp_dir();
    write(&dir, "x.drac", "let n = 1;\n");
    write(
        &dir,
        "x.meta",
        "\
id: x
targets: js
js.exit: 0
js.check: if (n !== 1) process.exit(1);
",
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir).arg("--coverage"));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(stdout.contains("coverage"), "stdout={stdout}");
}

#[test]
fn help_lists_test_coverage() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("--coverage") || stdout.contains("coverage"),
        "help should mention coverage:\n{stdout}"
    );
}

/// ROADMAP L05.01: `describe` / `it` suite that all pass → `draconic test` exit 0.
#[test]
fn test_runs_in_language_describe_it() {
    let dir = temp_dir();
    let src = write(
        &dir,
        "suite.drac",
        r#"
describe("math", () => {
  it("adds", () => {
    if (1 + 1 !== 2) throw 1;
  });
});
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&src));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("ok") || stdout.contains("passed"),
        "stdout={stdout}"
    );
}

/// ROADMAP L05.01: a throwing `it` fails `draconic test`.
#[test]
fn test_fails_in_language_it_throw() {
    let dir = temp_dir();
    let src = write(
        &dir,
        "suite.drac",
        r#"
describe("math", () => {
  it("adds", () => {
    throw 1;
  });
});
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&src));
    assert_ne!(
        code, 0,
        "expected failure\nstdout={stdout}\nstderr={stderr}"
    );
}

/// ROADMAP L05.04: passing fixture + passing in-language suite in one dir → exit 0.
#[test]
fn test_aggregates_passing_fixture_and_passing_suite() {
    let dir = temp_dir();
    write(&dir, "smoke.drac", "let x = 1 + 2;\n");
    write(
        &dir,
        "smoke.meta",
        "\
id: smoke
targets: js
js.exit: 0
js.check: if (x !== 3) process.exit(1);
",
    );
    write(
        &dir,
        "suite.drac",
        r#"
describe("math", () => {
  it("adds", () => {
    if (1 + 1 !== 2) throw 1;
  });
});
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("smoke"),
        "expected fixture id in output:\n{stdout}"
    );
    assert!(
        stdout.contains("suite"),
        "expected in-language suite id in output:\n{stdout}"
    );
}

/// ROADMAP L05.04: passing fixture + failing in-language suite → non-zero exit.
#[test]
fn test_aggregates_failing_suite_with_passing_fixture() {
    let dir = temp_dir();
    write(&dir, "smoke.drac", "let x = 1 + 2;\n");
    write(
        &dir,
        "smoke.meta",
        "\
id: smoke
targets: js
js.exit: 0
js.check: if (x !== 3) process.exit(1);
",
    );
    write(
        &dir,
        "suite.drac",
        r#"
describe("math", () => {
  it("adds", () => {
    throw 1;
  });
});
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir));
    assert_ne!(
        code, 0,
        "expected failure\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("FAIL")
            || stdout.contains("fail")
            || stderr.contains("FAIL")
            || stderr.contains("fail")
            || stdout.contains("suite"),
        "stdout={stdout}\nstderr={stderr}"
    );
}

/// ROADMAP L05.04: failing fixture + passing in-language suite → non-zero exit.
#[test]
fn test_aggregates_failing_fixture_with_passing_suite() {
    let dir = temp_dir();
    write(&dir, "bad.drac", "let x = 1;\n");
    write(
        &dir,
        "bad.meta",
        "\
id: bad
targets: js
js.exit: 0
js.check: if (x !== 99) process.exit(1);
",
    );
    write(
        &dir,
        "suite.drac",
        r#"
describe("math", () => {
  it("adds", () => {
    if (1 + 1 !== 2) throw 1;
  });
});
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir));
    assert_ne!(
        code, 0,
        "expected failure\nstdout={stdout}\nstderr={stderr}"
    );
}

/// ROADMAP L05.04: directory of only in-language suites (no .meta) still runs.
#[test]
fn test_dir_of_only_in_language_suites() {
    let dir = temp_dir();
    write(
        &dir,
        "suite.drac",
        r#"
describe("math", () => {
  it("adds", () => {
    if (1 + 1 !== 2) throw 1;
  });
});
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("ok") || stdout.contains("passed") || stdout.contains("suite"),
        "stdout={stdout}"
    );
}

/// ROADMAP L05: combined describe/it/expect + nested hooks suite with a fixture → exit 0.
#[test]
fn test_aggregates_surface_suite_with_passing_fixture() {
    let dir = temp_dir();
    write(&dir, "smoke.drac", "let x = 1 + 2;\n");
    write(
        &dir,
        "smoke.meta",
        "\
id: smoke
targets: js
js.exit: 0
js.check: if (x !== 3) process.exit(1);
",
    );
    write(
        &dir,
        "suite.drac",
        r#"
let order = "";
describe("outer", () => {
  before(() => {
    order = order + "B";
  });
  after(() => {
    order = order + "A";
  });
  beforeEach(() => {
    order = order + "b";
  });
  afterEach(() => {
    order = order + "a";
  });
  describe("inner", () => {
    beforeEach(() => {
      order = order + "i";
    });
    afterEach(() => {
      order = order + "j";
    });
    it("matchers", () => {
      expect(1).toBe(1);
      expect("x").toBeTruthy();
      expect(0).toBeFalsy();
      order = order + "T";
    });
  });
});
if (order !== "BbiTjaA") throw 1;
"#,
    );

    let (code, stdout, stderr) = run(draconic().arg("test").arg(&dir));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("smoke"),
        "expected fixture id in output:\n{stdout}"
    );
    assert!(
        stdout.contains("suite"),
        "expected in-language suite id in output:\n{stdout}"
    );
}

fn barrier_program(self_name: &str, peer_name: &str) -> String {
    format!(
        r#"
let dir = envGet("DRACONIC_C0401_BARRIER");
writeFileText(dir + "/{self_name}.started", "1");
let t0 = Date.now();
while (!exists(dir + "/{peer_name}.started")) {{
  if (Date.now() - t0 > 8000) {{
    throw 1;
  }}
}}
"#
    )
}

fn write_js_fixture(dir: &Path, id: &str, source: &str) {
    write(dir, &format!("{id}.drac"), source);
    write(
        dir,
        &format!("{id}.meta"),
        &format!(
            "\
id: {id}
targets: js
js.exit: 0
"
        ),
    );
}

/// ROADMAP C04.01: two fixtures that wait for each other only finish if workers > 1.
#[test]
fn test_worker_pool_overlaps_two_fixtures() {
    let dir = temp_dir();
    let barrier = dir.join("barrier");
    fs::create_dir_all(&barrier).unwrap();
    write_js_fixture(&dir, "left", &barrier_program("left", "right"));
    write_js_fixture(&dir, "right", &barrier_program("right", "left"));

    let (code, stdout, stderr) = run(draconic()
        .arg("test")
        .arg("--jobs")
        .arg("2")
        .env("DRACONIC_C0401_BARRIER", &barrier)
        .arg(&dir));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("left") && stdout.contains("right"),
        "stdout={stdout}"
    );
}

/// ROADMAP C04.01: default `draconic test` uses a worker pool (N>1).
#[test]
fn test_default_worker_pool_overlaps_two_fixtures() {
    let dir = temp_dir();
    let barrier = dir.join("barrier");
    fs::create_dir_all(&barrier).unwrap();
    write_js_fixture(&dir, "left", &barrier_program("left", "right"));
    write_js_fixture(&dir, "right", &barrier_program("right", "left"));

    let (code, stdout, stderr) = run(draconic()
        .arg("test")
        .env("DRACONIC_C0401_BARRIER", &barrier)
        .arg(&dir));
    assert_eq!(code, 0, "stdout={stdout}\nstderr={stderr}");
    assert!(
        stdout.contains("left") && stdout.contains("right"),
        "stdout={stdout}"
    );
}

#[test]
fn help_lists_test_jobs() {
    let (code, stdout, stderr) = run(draconic().arg("help"));
    assert_eq!(code, 0, "stderr={stderr}");
    assert!(
        stdout.contains("--jobs") || stdout.contains("jobs"),
        "help should mention --jobs:\n{stdout}"
    );
}

fn fail_ids(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|l| l.starts_with("FAIL "))
        .filter_map(|l| l.split_whitespace().nth(1))
        .collect()
}

fn write_failing_js_fixture(dir: &Path, file_stem: &str, id: &str) {
    write(dir, &format!("{file_stem}.drac"), "let x = 1;\n");
    write(
        dir,
        &format!("{file_stem}.meta"),
        &format!(
            "\
id: {id}
targets: js
js.exit: 0
js.check: if (x !== 99) process.exit(1);
"
        ),
    );
}

/// ROADMAP C04.02: any failure → exit 1, even with a passing sibling and N>1 workers.
#[test]
fn test_aggregate_exit_is_one_when_any_fixture_fails() {
    let dir = temp_dir();
    write_js_fixture(&dir, "ok_one", "let n = 1;\n");
    write_failing_js_fixture(&dir, "z_fail", "alpha");
    write_failing_js_fixture(&dir, "a_fail", "zeta");

    let (code, stdout, stderr) = run(draconic().arg("test").arg("--jobs").arg("2").arg(&dir));
    assert_eq!(
        code, 1,
        "expected aggregate exit 1\nstdout={stdout}\nstderr={stderr}"
    );
    let (code2, stdout2, stderr2) = run(draconic().arg("test").arg("--jobs").arg("2").arg(&dir));
    assert_eq!(
        code2, 1,
        "exit must be stable across runs\nstdout={stdout2}\nstderr={stderr2}"
    );
}

/// ROADMAP C04.02: FAIL lines are ordered by fixture id, not path or completion.
#[test]
fn test_failure_summary_order_is_fixture_id() {
    let dir = temp_dir();
    write_failing_js_fixture(&dir, "z_fail", "alpha");
    write_failing_js_fixture(&dir, "a_fail", "zeta");
    write_js_fixture(&dir, "ok_mid", "let n = 0;\n");

    let (code, stdout, stderr) = run(draconic().arg("test").arg("--jobs").arg("2").arg(&dir));
    assert_eq!(code, 1, "stdout={stdout}\nstderr={stderr}");
    assert_eq!(
        fail_ids(&stdout),
        ["alpha", "zeta"],
        "FAIL summary must be fixture-id order, not path order:\n{stdout}"
    );
    let (code2, stdout2, stderr2) = run(draconic().arg("test").arg("--jobs").arg("2").arg(&dir));
    assert_eq!(code2, 1, "stdout={stdout2}\nstderr={stderr2}");
    assert_eq!(
        fail_ids(&stdout2),
        fail_ids(&stdout),
        "FAIL order must be stable across runs\nfirst={stdout}\nsecond={stdout2}"
    );
}

fn assert_c04_parallel_surface(code: i32, stdout: &str, stderr: &str) {
    assert_eq!(
        code, 1,
        "C04 aggregate exit must be 1 when any fixture fails\nstdout={stdout}\nstderr={stderr}"
    );
    assert!(
        stdout.contains("ok left") && stdout.contains("ok right"),
        "C04 worker pool (N>1) must let overlapping fixtures pass:\n{stdout}"
    );
    assert_eq!(
        fail_ids(stdout),
        ["alpha", "zeta"],
        "C04 FAIL summary must be fixture-id order:\n{stdout}"
    );
}

fn write_c04_surface_dir() -> (PathBuf, PathBuf) {
    let dir = temp_dir();
    let barrier = dir.join("barrier");
    fs::create_dir_all(&barrier).unwrap();
    write_js_fixture(&dir, "left", &barrier_program("left", "right"));
    write_js_fixture(&dir, "right", &barrier_program("right", "left"));
    write_failing_js_fixture(&dir, "z_fail", "alpha");
    write_failing_js_fixture(&dir, "a_fail", "zeta");
    (dir, barrier)
}

/// ROADMAP C04: default N>1 workers + mixed pass/fail → exit 1 + stable FAIL order.
#[test]
fn test_c04_parallel_surface_default_jobs() {
    let (dir, barrier) = write_c04_surface_dir();
    let (code, stdout, stderr) = run(draconic()
        .arg("test")
        .env("DRACONIC_C0401_BARRIER", &barrier)
        .arg(&dir));
    assert_c04_parallel_surface(code, &stdout, &stderr);
    let (code2, stdout2, stderr2) = run(draconic()
        .arg("test")
        .env("DRACONIC_C0401_BARRIER", &barrier)
        .arg(&dir));
    assert_c04_parallel_surface(code2, &stdout2, &stderr2);
    assert_eq!(
        fail_ids(&stdout2),
        fail_ids(&stdout),
        "C04 FAIL order must be stable across default-jobs runs"
    );
}

/// ROADMAP C04: `--jobs` path of the combined parallel-test surface.
#[test]
fn test_c04_parallel_surface_jobs_flag() {
    let (dir, barrier) = write_c04_surface_dir();
    let (code, stdout, stderr) = run(draconic()
        .arg("test")
        .arg("--jobs")
        .arg("2")
        .env("DRACONIC_C0401_BARRIER", &barrier)
        .arg(&dir));
    assert_c04_parallel_surface(code, &stdout, &stderr);
}

/// ROADMAP C04: default worker pool and `--jobs` both yield exit 1 with a
/// passing sibling, and FAIL order is fixture-id stable across runs.
#[test]
fn test_parallel_surface_aggregate_exit_and_stable_fail_order() {
    let dir = temp_dir();
    write_js_fixture(&dir, "ok_mid", "let n = 0;\n");
    write_failing_js_fixture(&dir, "z_fail", "alpha");
    write_failing_js_fixture(&dir, "a_fail", "zeta");

    let (code, stdout, stderr) = run(draconic().arg("test").arg("--jobs").arg("2").arg(&dir));
    assert_eq!(
        code, 1,
        "--jobs 2 aggregate exit 1\nstdout={stdout}\nstderr={stderr}"
    );
    assert_eq!(
        fail_ids(&stdout),
        ["alpha", "zeta"],
        "FAIL summary must be fixture-id order:\n{stdout}"
    );
    assert!(
        stdout.contains("ok ok_mid") || stdout.contains("ok_mid"),
        "passing sibling must still be reported:\n{stdout}"
    );

    let (code2, stdout2, stderr2) = run(draconic().arg("test").arg(&dir));
    assert_eq!(
        code2, 1,
        "default pool aggregate exit 1\nstdout={stdout2}\nstderr={stderr2}"
    );
    assert_eq!(
        fail_ids(&stdout2),
        ["alpha", "zeta"],
        "default pool FAIL order must match --jobs:\n{stdout2}"
    );
    let (code3, stdout3, stderr3) = run(draconic().arg("test").arg(&dir));
    assert_eq!(code3, 1, "stdout={stdout3}\nstderr={stderr3}");
    assert_eq!(
        fail_ids(&stdout3),
        fail_ids(&stdout2),
        "FAIL order must be stable across default-pool runs\nfirst={stdout2}\nsecond={stdout3}"
    );
}
