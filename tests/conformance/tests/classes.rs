//! ROADMAP E05: class fixtures on declared targets.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture};

#[test]
fn class_basic_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/classes/class_basic"),
        "missing es/classes/class_basic fixture, got {ids:?}"
    );
}

#[test]
fn class_basic_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_basic")
        .expect("es/classes/class_basic");
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
fn class_extends_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/classes/class_extends"),
        "missing es/classes/class_extends fixture, got {ids:?}"
    );
}

#[test]
fn class_extends_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_extends")
        .expect("es/classes/class_extends");
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
fn class_static_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/classes/class_static"),
        "missing es/classes/class_static fixture, got {ids:?}"
    );
}

#[test]
fn class_static_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_static")
        .expect("es/classes/class_static");
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
fn class_super_access_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/classes/class_super_access"),
        "missing es/classes/class_super_access fixture, got {ids:?}"
    );
}

#[test]
fn class_super_access_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_super_access")
        .expect("es/classes/class_super_access");
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
fn class_computed_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "es/classes/class_computed"),
        "missing es/classes/class_computed fixture, got {ids:?}"
    );
}

#[test]
fn class_computed_runs() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_computed")
        .expect("es/classes/class_computed");
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
fn class_super_assign_eval_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/classes/class_super_assign_eval"),
        "missing es/classes/class_super_assign_eval fixture, got {ids:?}"
    );
}

#[test]
fn class_super_assign_eval_runs() {
    // E19.72: super assign/compound/eval/null-proto on class methods.
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_super_assign_eval")
        .expect("es/classes/class_super_assign_eval");
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
fn class_heritage_is_constructor_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/classes/class_heritage_is_constructor"),
        "missing es/classes/class_heritage_is_constructor fixture, got {ids:?}"
    );
}

#[test]
fn class_heritage_is_constructor_runs() {
    // E19.82.02: extends non-constructor / invalid prototype → TypeError.
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_heritage_is_constructor")
        .expect("es/classes/class_heritage_is_constructor");
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
fn class_derived_ctor_this_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/classes/class_derived_ctor_this"),
        "missing es/classes/class_derived_ctor_this fixture, got {ids:?}"
    );
}

#[test]
fn class_derived_ctor_this_runs() {
    // E19.82.03: derived ctor this TDZ, super via Reflect.construct, return override.
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_derived_ctor_this")
        .expect("es/classes/class_derived_ctor_this");
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
fn class_static_fields_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/classes/class_static_fields"),
        "missing es/classes/class_static_fields fixture, got {ids:?}"
    );
}

#[test]
fn class_static_fields_runs() {
    // E19.82.04: static field this, NamedEvaluation, intercalated keys, static name.
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_static_fields")
        .expect("es/classes/class_static_fields");
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
fn class_arrow_super_fields_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/classes/class_arrow_super_fields"),
        "missing es/classes/class_arrow_super_fields fixture, got {ids:?}"
    );
}

#[test]
fn class_arrow_super_fields_runs() {
    // E19.82.05: arrow SuperCall in derived ctor; SuperProperty in field inits; .prototype.
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_arrow_super_fields")
        .expect("es/classes/class_arrow_super_fields");
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
fn class_field_init_eval_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/classes/class_field_init_eval"),
        "missing es/classes/class_field_init_eval fixture, got {ids:?}"
    );
}

#[test]
fn class_field_init_eval_runs() {
    // E19.82.06: field-init direct eval SuperProperty/new.target; arguments → SyntaxError.
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_field_init_eval")
        .expect("es/classes/class_field_init_eval");
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
fn class_private_nested_shadow_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/classes/class_private_nested_shadow"),
        "missing es/classes/class_private_nested_shadow fixture, got {ids:?}"
    );
}

#[test]
fn class_private_nested_shadow_runs() {
    // E19.82.07: nested class private name shadows outer same-name of any kind.
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_private_nested_shadow")
        .expect("es/classes/class_private_nested_shadow");
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
fn class_private_eval_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/classes/class_private_eval"),
        "missing es/classes/class_private_eval fixture, got {ids:?}"
    );
}

#[test]
fn class_private_eval_runs() {
    // E19.82.08: private names visible to direct eval in methods and field inits.
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_private_eval")
        .expect("es/classes/class_private_eval");
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
fn class_private_add_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "es/classes/class_private_add"),
        "missing es/classes/class_private_add fixture, got {ids:?}"
    );
}

#[test]
fn class_private_add_runs() {
    // E19.82.09: PrivateFieldAdd / PrivateMethodOrAccessorAdd TypeError.
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "es/classes/class_private_add")
        .expect("es/classes/class_private_add");
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
