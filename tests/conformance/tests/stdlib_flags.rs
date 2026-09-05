//! ROADMAP L07 / L07.01 / L07.02: flags parse + typed options and help text.
//! L07 parent locks the combined argv → typed options/positionals surface in one Program.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn parse_long_short_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/flags/parse_long_short"),
        "missing stdlib/flags/parse_long_short fixture, got {ids:?}"
    );
}

#[test]
fn parse_long_short_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/flags/parse_long_short")
        .expect("stdlib/flags/parse_long_short");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L07.01 targets both js and native"
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
fn typed_options_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/flags/typed_options"),
        "missing stdlib/flags/typed_options fixture, got {ids:?}"
    );
}

#[test]
fn typed_options_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/flags/typed_options")
        .expect("stdlib/flags/typed_options");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L07.02 targets both js and native"
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
        ids.iter().any(|id| *id == "stdlib/flags/surface"),
        "missing stdlib/flags/surface fixture, got {ids:?}"
    );
}

#[test]
fn surface_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/flags/surface")
        .expect("stdlib/flags/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "stdlib/flags/surface must target js and native"
    );
    for name in [
        "parseFlags",
        "flagHelp",
        "positionals",
        "boolean",
        "string",
        "number",
        "--still",
    ] {
        assert!(
            fixture.source.contains(name),
            "L07 surface must use {name} in one Program"
        );
    }
    assert!(
        fixture.source.contains("parseFlags(["),
        "L07 surface must parse argv without a spec (schema-free long/short + positionals)"
    );
    assert!(
        fixture.source.contains(", spec)"),
        "L07 surface must parse argv with a typed spec"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("true\ntrue\ntrue\ntrue\ntrue\n"),
        "L07 surface must observe schema-free parse, typed options, help text, and leftover positionals"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "L07 surface must terminate with exit 0"
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
