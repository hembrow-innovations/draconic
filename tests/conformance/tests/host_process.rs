//! ROADMAP H01.01 / H01.02 / H01.03 / H01.04: process args + env + exit + pid/ppid.

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
fn process_args_fixture_present() {
    assert_fixture_present("host/process/process_args");
}

#[test]
fn process_args_runs_js_and_native() {
    assert_fixture_runs("host/process/process_args");
}

#[test]
fn process_args_empty_fixture_present() {
    assert_fixture_present("host/process/process_args_empty");
}

#[test]
fn process_args_empty_runs_js_and_native() {
    assert_fixture_runs("host/process/process_args_empty");
}

#[test]
fn process_env_fixture_present() {
    assert_fixture_present("host/process/process_env");
}

#[test]
fn process_env_runs_js_and_native() {
    assert_fixture_runs("host/process/process_env");
}

#[test]
fn process_exit_fixture_present() {
    assert_fixture_present("host/process/process_exit");
}

#[test]
fn process_exit_runs_js_and_native() {
    assert_fixture_runs("host/process/process_exit");
}

#[test]
fn process_exit_code_fixture_present() {
    assert_fixture_present("host/process/process_exit_code");
}

#[test]
fn process_exit_code_runs_js_and_native() {
    assert_fixture_runs("host/process/process_exit_code");
}

#[test]
fn process_exit_default_fixture_present() {
    assert_fixture_present("host/process/process_exit_default");
}

#[test]
fn process_exit_default_runs_js_and_native() {
    assert_fixture_runs("host/process/process_exit_default");
}

#[test]
fn process_pid_fixture_present() {
    assert_fixture_present("host/process/process_pid");
}

#[test]
fn process_pid_runs_js_and_native() {
    assert_fixture_runs("host/process/process_pid");
}
