//! ROADMAP E08.01–E08.06: number, BigInt, Math, and Number global fixtures on js + native.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn number_literals_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/numbers/number_literals"),
        "missing es/numbers/number_literals fixture, got {ids:?}"
    );
}

#[test]
fn number_literals_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/numbers/number_literals")
        .expect("es/numbers/number_literals");
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
fn bigint_literals_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/numbers/bigint_literals"),
        "missing es/numbers/bigint_literals fixture, got {ids:?}"
    );
}

#[test]
fn bigint_literals_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/numbers/bigint_literals")
        .expect("es/numbers/bigint_literals");
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
fn bigint_ops_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/numbers/bigint_ops"),
        "missing es/numbers/bigint_ops fixture, got {ids:?}"
    );
}

#[test]
fn bigint_ops_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/numbers/bigint_ops")
        .expect("es/numbers/bigint_ops");
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
fn bigint_pow_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/numbers/bigint_pow"),
        "missing es/numbers/bigint_pow fixture, got {ids:?}"
    );
}

#[test]
fn bigint_pow_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/numbers/bigint_pow")
        .expect("es/numbers/bigint_pow");
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
fn math_basics_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/numbers/math_basics"),
        "missing es/numbers/math_basics fixture, got {ids:?}"
    );
}

#[test]
fn math_basics_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/numbers/math_basics")
        .expect("es/numbers/math_basics");
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
fn number_global_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/numbers/number_global"),
        "missing es/numbers/number_global fixture, got {ids:?}"
    );
}

#[test]
fn number_global_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/numbers/number_global")
        .expect("es/numbers/number_global");
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
