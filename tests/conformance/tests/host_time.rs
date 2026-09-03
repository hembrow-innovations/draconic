//! ROADMAP H05 / H05.01–H05.05: wall clock, monotonic clock, timers, run-loop wait.
//! H05 parent locks the combined time surface in one Program.

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
fn now_ms_fixture_present() {
    assert_fixture_present("host/time/now_ms");
}

#[test]
fn now_ms_runs_js_and_native() {
    assert_fixture_runs("host/time/now_ms");
}

#[test]
fn date_now_fixture_present() {
    assert_fixture_present("host/time/date_now");
}

#[test]
fn date_now_runs_js_and_native() {
    assert_fixture_runs("host/time/date_now");
}

#[test]
fn monotonic_ms_fixture_present() {
    assert_fixture_present("host/time/monotonic_ms");
}

#[test]
fn monotonic_ms_runs_js_and_native() {
    assert_fixture_runs("host/time/monotonic_ms");
}

#[test]
fn set_timeout_fixture_present() {
    assert_fixture_present("host/time/set_timeout");
}

#[test]
fn set_timeout_runs_js_and_native() {
    assert_fixture_runs("host/time/set_timeout");
}

#[test]
fn set_interval_fixture_present() {
    assert_fixture_present("host/time/set_interval");
}

#[test]
fn set_interval_runs_js_and_native() {
    assert_fixture_runs("host/time/set_interval");
}

#[test]
fn timer_wait_fixture_present() {
    assert_fixture_present("host/time/timer_wait");
}

#[test]
fn timer_wait_runs_js_and_native() {
    assert_fixture_runs("host/time/timer_wait");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("host/time/surface");
}

#[test]
fn surface_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/time/surface")
        .expect("host/time/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "host/time/surface must target js and native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("true\ntrue\ntrue\nnumber\nnumber\n1\n0\n2\n2\n3\nfunction\nfunction\nfunction\nfunction\ntrue\ntrue\n"),
        "H05 surface must observe wall clock, monotonic clock, timeout, interval, and timer ids"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "H05 surface must terminate with exit 0"
    );
    assert_fixture_runs("host/time/surface");
}
