//! ROADMAP H03.01: pathJoin / pathNormalize (pure string path helpers).

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
fn path_join_fixture_present() {
    assert_fixture_present("host/path/path_join");
}

#[test]
fn path_join_runs_js_and_native() {
    assert_fixture_runs("host/path/path_join");
}

#[test]
fn path_normalize_fixture_present() {
    assert_fixture_present("host/path/path_normalize");
}

#[test]
fn path_normalize_runs_js_and_native() {
    assert_fixture_runs("host/path/path_normalize");
}
