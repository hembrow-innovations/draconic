//! ROADMAP H01 / H14 / H15: process + signals + run + spawn + async wait.
//! H01 parent locks the combined args / env / pid / exitCode surface in one Program.

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

fn assert_fixture_runs_js_and_native(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "{id} must target js and native"
    );
    assert_fixture_runs(id);
}

#[test]
fn process_args_fixture_present() {
    assert_fixture_present("host/process/process_args");
}

#[test]
fn process_args_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_args");
}

#[test]
fn process_args_empty_fixture_present() {
    assert_fixture_present("host/process/process_args_empty");
}

#[test]
fn process_args_empty_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_args_empty");
}

#[test]
fn process_env_fixture_present() {
    assert_fixture_present("host/process/process_env");
}

#[test]
fn process_env_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_env");
}

#[test]
fn process_exit_fixture_present() {
    assert_fixture_present("host/process/process_exit");
}

#[test]
fn process_exit_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_exit");
}

#[test]
fn process_exit_code_fixture_present() {
    assert_fixture_present("host/process/process_exit_code");
}

#[test]
fn process_exit_code_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_exit_code");
}

#[test]
fn process_exit_default_fixture_present() {
    assert_fixture_present("host/process/process_exit_default");
}

#[test]
fn process_exit_default_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_exit_default");
}

#[test]
fn process_pid_fixture_present() {
    assert_fixture_present("host/process/process_pid");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("host/process/surface");
}

#[test]
fn surface_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/process/surface")
        .expect("host/process/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "host/process/surface must target js and native"
    );
    assert_eq!(
        fixture.expect_native.args,
        vec!["surface-arg".to_string()],
        "H01 surface must pass one user arg"
    );
    assert_eq!(
        fixture.expect_js.args,
        vec!["surface-arg".to_string()],
        "H01 surface must pass one user arg on js"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("1\nsurface-arg\nalpha\nundefined\nundefined\nnumber\nnumber\ntrue\ntrue\n0\n"),
        "H01 surface must observe args, env get/set/delete, pid/ppid, and exitCode"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "H01 surface must terminate with exitCode 0"
    );
    assert_fixture_runs_js_and_native("host/process/surface");
}

#[test]
fn process_pid_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_pid");
}

#[test]
fn process_run_exit_fixture_present() {
    assert_fixture_present("host/process/process_run_exit");
}

#[test]
fn process_run_exit_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_run_exit");
}

#[test]
fn process_run_cwd_fixture_present() {
    assert_fixture_present("host/process/process_run_cwd");
}

#[test]
fn process_run_cwd_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_run_cwd");
}

#[test]
fn process_run_env_fixture_present() {
    assert_fixture_present("host/process/process_run_env");
}

#[test]
fn process_run_env_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_run_env");
}

#[test]
fn process_spawn_capture_fixture_present() {
    assert_fixture_present("host/process/process_spawn_capture");
}

#[test]
fn process_spawn_capture_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_spawn_capture");
}

#[test]
fn process_spawn_kill_fixture_present() {
    assert_fixture_present("host/process/process_spawn_kill");
}

#[test]
fn process_spawn_kill_runs_js_and_native() {
    assert_fixture_runs_js_and_native("host/process/process_spawn_kill");
}

#[test]
fn process_wait_async_fixture_present() {
    assert_fixture_present("host/process/process_wait_async");
}

#[test]
fn process_wait_async_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/process/process_wait_async")
        .expect("host/process/process_wait_async");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert!(!fixture.targets.contains(&Target::Js), "native-only");
    assert_fixture_runs("host/process/process_wait_async");
}

#[test]
fn signal_watch_sigterm_fixture_present() {
    assert_fixture_present("host/process/signal_watch_sigterm");
}

#[test]
fn signal_watch_sigterm_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/process/signal_watch_sigterm")
        .expect("host/process/signal_watch_sigterm");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert!(!fixture.targets.contains(&Target::Js), "native-only");
    assert_fixture_runs("host/process/signal_watch_sigterm");
}

#[test]
fn signal_watch_sigint_fixture_present() {
    assert_fixture_present("host/process/signal_watch_sigint");
}

#[test]
fn signal_watch_sigint_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/process/signal_watch_sigint")
        .expect("host/process/signal_watch_sigint");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/process/signal_watch_sigint");
}

#[test]
fn signal_ignore_fixture_present() {
    assert_fixture_present("host/process/signal_ignore");
}

#[test]
fn signal_ignore_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/process/signal_ignore")
        .expect("host/process/signal_ignore");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert!(!fixture.targets.contains(&Target::Js), "native-only");
    assert_fixture_runs("host/process/signal_ignore");
}

#[test]
fn signal_restore_rewatch_fixture_present() {
    assert_fixture_present("host/process/signal_restore_rewatch");
}

#[test]
fn signal_restore_rewatch_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/process/signal_restore_rewatch")
        .expect("host/process/signal_restore_rewatch");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/process/signal_restore_rewatch");
}
