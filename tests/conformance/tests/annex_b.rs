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

#[test]
fn legacy_octal_string_fixture_present() {
    assert_fixture_present("es/annex-b/legacy_octal_string");
}

#[test]
fn legacy_octal_string_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/legacy_octal_string");
}

#[test]
fn legacy_octal_numeric_fixture_present() {
    assert_fixture_present("es/annex-b/legacy_octal_numeric");
}

#[test]
fn legacy_octal_numeric_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/legacy_octal_numeric");
}

#[test]
fn labelled_function_fixture_present() {
    assert_fixture_present("es/annex-b/labelled_function");
}

#[test]
fn labelled_function_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/labelled_function");
}

#[test]
fn if_function_fixture_present() {
    assert_fixture_present("es/annex-b/if_function");
}

#[test]
fn if_function_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/if_function");
}

#[test]
fn block_function_fixture_present() {
    assert_fixture_present("es/annex-b/block_function");
}

#[test]
fn block_function_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/block_function");
}

#[test]
fn var_decl_fixture_present() {
    assert_fixture_present("es/annex-b/var_decl");
}

#[test]
fn var_decl_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/var_decl");
}

#[test]
fn var_for_fixture_present() {
    assert_fixture_present("es/annex-b/var_for");
}

#[test]
fn var_for_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/var_for");
}

#[test]
fn regexp_statics_fixture_present() {
    assert_fixture_present("es/annex-b/regexp_statics");
}

#[test]
fn regexp_statics_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/regexp_statics");
}

#[test]
fn var_catch_fixture_present() {
    assert_fixture_present("es/annex-b/var_catch");
}

#[test]
fn var_catch_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/var_catch");
}

#[test]
fn regexp_literal_fixture_present() {
    assert_fixture_present("es/annex-b/regexp_literal");
}

#[test]
fn regexp_literal_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/regexp_literal");
}

#[test]
fn object_destructure_fixture_present() {
    assert_fixture_present("es/annex-b/object_destructure");
}

#[test]
fn object_destructure_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/object_destructure");
}

#[test]
fn destructure_defaults_fixture_present() {
    assert_fixture_present("es/annex-b/destructure_defaults");
}

#[test]
fn destructure_defaults_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/destructure_defaults");
}

#[test]
fn instanceof_fixture_present() {
    assert_fixture_present("es/annex-b/instanceof");
}

#[test]
fn instanceof_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/instanceof");
}

#[test]
fn accessors_fixture_present() {
    assert_fixture_present("es/annex-b/accessors");
}

#[test]
fn accessors_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/accessors");
}

#[test]
fn optional_chain_fixture_present() {
    assert_fixture_present("es/annex-b/optional_chain");
}

#[test]
fn optional_chain_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/optional_chain");
}

#[test]
fn arguments_object_fixture_present() {
    assert_fixture_present("es/annex-b/arguments_object");
}

#[test]
fn arguments_object_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/arguments_object");
}

#[test]
fn param_destructure_fixture_present() {
    assert_fixture_present("es/annex-b/param_destructure");
}

#[test]
fn param_destructure_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/param_destructure");
}

#[test]
fn class_fields_fixture_present() {
    assert_fixture_present("es/annex-b/class_fields");
}

#[test]
fn class_fields_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/class_fields");
}

#[test]
fn new_target_fixture_present() {
    assert_fixture_present("es/annex-b/new_target");
}

#[test]
fn new_target_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/new_target");
}

#[test]
fn object_spread_fixture_present() {
    assert_fixture_present("es/annex-b/object_spread");
}

#[test]
fn object_spread_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/object_spread");
}

#[test]
fn export_star_fixture_present() {
    assert_fixture_present("es/annex-b/export_star");
}

#[test]
fn export_star_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/export_star");
}

#[test]
fn export_named_from_fixture_present() {
    assert_fixture_present("es/annex-b/export_named_from");
}

#[test]
fn export_named_from_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/export_named_from");
}

#[test]
fn export_ns_from_fixture_present() {
    assert_fixture_present("es/annex-b/export_ns_from");
}

#[test]
fn export_ns_from_runs_js_and_native() {
    assert_fixture_runs_js_and_native("es/annex-b/export_ns_from");
}
