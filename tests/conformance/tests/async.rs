//! ROADMAP E12.01+: Promise / async fixtures on js + native.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn promise_basics_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/async/promise_basics"),
        "missing es/async/promise_basics fixture, got {ids:?}"
    );
}

#[test]
fn promise_basics_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/async/promise_basics")
        .expect("es/async/promise_basics");
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
fn promise_resolve_reject_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/async/promise_resolve_reject"),
        "missing es/async/promise_resolve_reject fixture, got {ids:?}"
    );
}

#[test]
fn promise_resolve_reject_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/async/promise_resolve_reject")
        .expect("es/async/promise_resolve_reject");
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
fn promise_finally_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/async/promise_finally"),
        "missing es/async/promise_finally fixture, got {ids:?}"
    );
}

#[test]
fn promise_finally_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/async/promise_finally")
        .expect("es/async/promise_finally");
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
fn promise_all_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/async/promise_all"),
        "missing es/async/promise_all fixture, got {ids:?}"
    );
}

#[test]
fn promise_all_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/async/promise_all")
        .expect("es/async/promise_all");
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
fn promise_race_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/async/promise_race"),
        "missing es/async/promise_race fixture, got {ids:?}"
    );
}

#[test]
fn promise_race_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/async/promise_race")
        .expect("es/async/promise_race");
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
fn promise_all_settled_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/async/promise_allSettled"),
        "missing es/async/promise_allSettled fixture, got {ids:?}"
    );
}

#[test]
fn promise_all_settled_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/async/promise_allSettled")
        .expect("es/async/promise_allSettled");
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
fn promise_any_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/async/promise_any"),
        "missing es/async/promise_any fixture, got {ids:?}"
    );
}

#[test]
fn promise_any_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/async/promise_any")
        .expect("es/async/promise_any");
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
fn async_await_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/async/async_await"),
        "missing es/async/async_await fixture, got {ids:?}"
    );
}

#[test]
fn async_await_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/async/async_await")
        .expect("es/async/async_await");
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
fn async_arrow_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/async/async_arrow"),
        "missing es/async/async_arrow fixture, got {ids:?}"
    );
}

#[test]
fn async_arrow_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/async/async_arrow")
        .expect("es/async/async_arrow");
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
