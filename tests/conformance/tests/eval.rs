//! ROADMAP E16.01+: eval / Function fixtures on js + native.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn direct_eval_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/eval/direct_eval"),
        "missing es/eval/direct_eval fixture, got {ids:?}"
    );
}

#[test]
fn direct_eval_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/eval/direct_eval")
        .expect("es/eval/direct_eval");
    assert!(fixture.targets.contains(&Target::Js));
    assert!(fixture.targets.contains(&Target::Native));
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
fn new_function_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/eval/new_function"),
        "missing es/eval/new_function fixture, got {ids:?}"
    );
}

#[test]
fn new_function_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/eval/new_function")
        .expect("es/eval/new_function");
    assert!(fixture.targets.contains(&Target::Js));
    assert!(fixture.targets.contains(&Target::Native));
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
fn indirect_eval_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/eval/indirect_eval"),
        "missing es/eval/indirect_eval fixture, got {ids:?}"
    );
}

#[test]
fn indirect_eval_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/eval/indirect_eval")
        .expect("es/eval/indirect_eval");
    assert!(fixture.targets.contains(&Target::Js));
    assert!(fixture.targets.contains(&Target::Native));
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
