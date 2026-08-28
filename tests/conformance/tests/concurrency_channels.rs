//! ROADMAP C02.01: channel send/recv — scalars + strings.

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
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "{id} must target js and native"
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
fn channel_typeof_fixture_present() {
    assert_fixture_present("concurrency/channels/channel_typeof");
}

#[test]
fn channel_typeof_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/channels/channel_typeof");
}

#[test]
fn channel_number_fixture_present() {
    assert_fixture_present("concurrency/channels/channel_number");
}

#[test]
fn channel_number_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/channels/channel_number");
}

#[test]
fn channel_string_fixture_present() {
    assert_fixture_present("concurrency/channels/channel_string");
}

#[test]
fn channel_string_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/channels/channel_string");
}

#[test]
fn channel_bool_fixture_present() {
    assert_fixture_present("concurrency/channels/channel_bool");
}

#[test]
fn channel_bool_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/channels/channel_bool");
}

#[test]
fn channel_send_bad_fixture_present() {
    assert_fixture_present("concurrency/channels/channel_send_bad");
}

#[test]
fn channel_send_bad_runs_js_and_native() {
    assert_fixture_runs_js_and_native("concurrency/channels/channel_send_bad");
}
