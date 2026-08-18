//! ROADMAP H00.01: host API registry — js unsupported → hard diagnostic.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs_js(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Js),
        "{id} must target js"
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
fn tcp_listen_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/tcp_listen_js_hard_error");
}

#[test]
fn tcp_listen_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/tcp_listen_js_hard_error");
}

#[test]
fn open_file_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/open_file_js_hard_error");
}

#[test]
fn open_file_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/open_file_js_hard_error");
}
