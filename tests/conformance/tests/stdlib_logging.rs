//! ROADMAP L06 / L06.01 / L06.02: leveled logger + stderr/stdout sink.
//! L06 parent locks the combined logging library surface in one Program.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn levels_filter_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/logging/levels_filter"),
        "missing stdlib/logging/levels_filter fixture, got {ids:?}"
    );
}

#[test]
fn levels_filter_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/logging/levels_filter")
        .expect("stdlib/logging/levels_filter");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L06.01 targets both js and native"
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
fn stdio_sink_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/logging/stdio_sink"),
        "missing stdlib/logging/stdio_sink fixture, got {ids:?}"
    );
}

#[test]
fn stdio_sink_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/logging/stdio_sink")
        .expect("stdlib/logging/stdio_sink");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L06.02 targets both js and native"
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
fn surface_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/logging/surface"),
        "missing stdlib/logging/surface fixture, got {ids:?}"
    );
}

#[test]
fn surface_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/logging/surface")
        .expect("stdlib/logging/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "stdlib/logging/surface must target js and native"
    );
    for name in [
        "createLogger",
        "setLevel",
        "getLevel",
        "records",
        "stdio",
        "debug",
        "info",
        "warn",
        "error",
        "TypeError",
    ] {
        assert!(
            fixture.source.contains(name),
            "L06 surface must use {name} in one Program"
        );
    }
    assert_eq!(
        fixture.expect_js.stdout.as_deref(),
        Some("info i\ndebug d\n"),
        "L06 surface must sink debug/info to js stdout"
    );
    assert_eq!(
        fixture.expect_js.stderr.as_deref(),
        Some("warn w\nerror e\n"),
        "L06 surface must sink warn/error to js stderr"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("info i\ndebug d\ntrue\ntrue\ntrue\ntrue\n1\n1\n"),
        "L06 surface must observe stdio sink, level filter, isolation, and invalid-input errors"
    );
    assert_eq!(
        fixture.expect_native.stderr.as_deref(),
        Some("warn w\nerror e\n"),
        "L06 surface must sink warn/error to native stderr"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "L06 surface must terminate with exit 0"
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
