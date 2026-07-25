//! ROADMAP N01: native integer types i8–i64, u8–u64.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs_native(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Native),
        "{id} must target native"
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
fn arith_i32_fixture_present() {
    assert_fixture_present("native/ints/arith_i32");
}

#[test]
fn arith_i32_runs_native() {
    assert_fixture_runs_native("native/ints/arith_i32");
}

#[test]
fn widths_fixture_present() {
    assert_fixture_present("native/ints/widths");
}

#[test]
fn widths_runs_native() {
    assert_fixture_runs_native("native/ints/widths");
}

#[test]
fn bitwise_fixture_present() {
    assert_fixture_present("native/ints/bitwise");
}

#[test]
fn bitwise_runs_native() {
    assert_fixture_runs_native("native/ints/bitwise");
}

#[test]
fn wrap_i8_fixture_present() {
    assert_fixture_present("native/ints/wrap_i8");
}

#[test]
fn wrap_i8_runs_native() {
    assert_fixture_runs_native("native/ints/wrap_i8");
}

#[test]
fn fn_call_fixture_present() {
    assert_fixture_present("native/ints/fn_call");
}

#[test]
fn fn_call_runs_native() {
    assert_fixture_runs_native("native/ints/fn_call");
}

#[test]
fn compare_update_fixture_present() {
    assert_fixture_present("native/ints/compare_update");
}

#[test]
fn compare_update_runs_native() {
    assert_fixture_runs_native("native/ints/compare_update");
}
