//! ROADMAP E11.01–E11.04: named, default, namespace, and cyclic import fixtures on declared targets.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn named_export_import_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/modules/named_export_import"),
        "missing es/modules/named_export_import fixture, got {ids:?}"
    );
    // Dependency module must not be a separate fixture (no .meta).
    assert!(
        !ids.iter().any(|id| id.contains("named_lib")),
        "named_lib should not be a fixture entry, got {ids:?}"
    );
}

#[test]
fn named_export_import_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/modules/named_export_import")
        .expect("es/modules/named_export_import");
    assert!(!fixture.targets.is_empty());
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
fn default_export_import_fixtures_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    for want in [
        "es/modules/default_export_import",
        "es/modules/default_expr_import",
    ] {
        assert!(
            ids.iter().any(|id| *id == want),
            "missing {want} fixture, got {ids:?}"
        );
    }
    assert!(
        !ids.iter().any(|id| id.contains("default_lib") || id.contains("default_expr_lib")),
        "default lib modules should not be fixture entries, got {ids:?}"
    );
}

#[test]
fn default_export_import_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    for id in [
        "es/modules/default_export_import",
        "es/modules/default_expr_import",
    ] {
        let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
        assert!(!fixture.targets.is_empty());
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
}

#[test]
fn namespace_import_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/modules/namespace_import"),
        "missing es/modules/namespace_import fixture, got {ids:?}"
    );
    assert!(
        !ids.iter().any(|id| id.contains("ns_lib")),
        "ns_lib should not be a fixture entry, got {ids:?}"
    );
}

#[test]
fn namespace_import_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/modules/namespace_import")
        .expect("es/modules/namespace_import");
    assert!(!fixture.targets.is_empty());
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
fn cyclic_module_fixtures_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    for want in ["es/modules/cyclic_functions", "es/modules/cyclic_live"] {
        assert!(
            ids.iter().any(|id| *id == want),
            "missing {want} fixture, got {ids:?}"
        );
    }
    assert!(
        !ids.iter().any(|id| {
            id.contains("cyclic_a")
                || id.contains("cyclic_b")
                || id.contains("cyclic_live_a")
                || id.contains("cyclic_live_b")
        }),
        "cycle dependency modules should not be fixture entries, got {ids:?}"
    );
}

#[test]
fn cyclic_modules_run() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    for id in ["es/modules/cyclic_functions", "es/modules/cyclic_live"] {
        let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
        assert!(!fixture.targets.is_empty());
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
}
