//! ROADMAP H02.01 / H02.02 / H02.03: stdout + stderr write; stdin read line/bytes.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "{id} must target js and native"
    );
    for r in run_fixture(fixture) {
        assert!(
            r.ok,
            "{} @ {}: {}",
            r.fixture_id,
            r.target.as_str(),
            r.message
        );
    }
}

#[test]
fn stdout_write_string_fixture_present() {
    assert_fixture_present("host/stdio/stdout_write_string");
}

#[test]
fn stdout_write_string_runs_js_and_native() {
    assert_fixture_runs("host/stdio/stdout_write_string");
}

#[test]
fn stdout_write_bytes_fixture_present() {
    assert_fixture_present("host/stdio/stdout_write_bytes");
}

#[test]
fn stdout_write_bytes_runs_js_and_native() {
    assert_fixture_runs("host/stdio/stdout_write_bytes");
}

#[test]
fn stderr_write_string_fixture_present() {
    assert_fixture_present("host/stdio/stderr_write_string");
}

#[test]
fn stderr_write_string_runs_js_and_native() {
    assert_fixture_runs("host/stdio/stderr_write_string");
}

#[test]
fn stderr_write_bytes_fixture_present() {
    assert_fixture_present("host/stdio/stderr_write_bytes");
}

#[test]
fn stderr_write_bytes_runs_js_and_native() {
    assert_fixture_runs("host/stdio/stderr_write_bytes");
}

#[test]
fn stdin_read_line_fixture_present() {
    assert_fixture_present("host/stdio/stdin_read_line");
}

#[test]
fn stdin_read_line_runs_js_and_native() {
    assert_fixture_runs("host/stdio/stdin_read_line");
}

#[test]
fn stdin_read_line_eof_fixture_present() {
    assert_fixture_present("host/stdio/stdin_read_line_eof");
}

#[test]
fn stdin_read_line_eof_runs_js_and_native() {
    assert_fixture_runs("host/stdio/stdin_read_line_eof");
}

#[test]
fn stdin_read_bytes_fixture_present() {
    assert_fixture_present("host/stdio/stdin_read_bytes");
}

#[test]
fn stdin_read_bytes_runs_js_and_native() {
    assert_fixture_runs("host/stdio/stdin_read_bytes");
}
