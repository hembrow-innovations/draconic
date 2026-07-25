//! ROADMAP E18.01+: Annex B fixtures on js + native.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs_js_and_native(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
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
fn escape_unescape_fixture_present() {
    assert_fixture_present("es/annex-b/escape_unescape");
}

#[test]
fn escape_unescape_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/escape_unescape");
}

#[test]
fn object_proto_fixture_present() {
    assert_fixture_present("es/annex-b/object_proto");
}

#[test]
fn object_proto_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/object_proto");
}

#[test]
fn string_proto_annex_fixture_present() {
    assert_fixture_present("es/annex-b/string_proto_annex");
}

#[test]
fn string_proto_annex_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/string_proto_annex");
}

#[test]
fn date_proto_annex_fixture_present() {
    assert_fixture_present("es/annex-b/date_proto_annex");
}

#[test]
fn date_proto_annex_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/date_proto_annex");
}

#[test]
fn regexp_compile_fixture_present() {
    assert_fixture_present("es/annex-b/regexp_compile");
}

#[test]
fn regexp_compile_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/regexp_compile");
}

#[test]
fn string_trim_left_right_fixture_present() {
    assert_fixture_present("es/annex-b/string_trim_left_right");
}

#[test]
fn string_trim_left_right_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/string_trim_left_right");
}

#[test]
fn object_accessor_legacy_fixture_present() {
    assert_fixture_present("es/annex-b/object_accessor_legacy");
}

#[test]
fn object_accessor_legacy_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/object_accessor_legacy");
}

#[test]
fn html_comments_fixture_present() {
    assert_fixture_present("es/annex-b/html_comments");
}

#[test]
fn html_comments_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/html_comments");
}
