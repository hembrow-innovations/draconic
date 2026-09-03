//! ROADMAP H00 / H00.01 / H17.04: host I/O surface policy.
//! H00 parent locks free-identifier shape, HostError model, and js hard-error vs polyfill.
//! H17.04 lands the JS/Node bridge subset (TCP listen/accept, dnsLookup, HTTP helpers);
//! APIs outside that subset still hard-error on js (no silent polyfill).

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
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

fn assert_fixture_runs_js(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(fixture.targets.contains(&Target::Js), "{id} must target js");
    assert_fixture_runs(id);
}

fn assert_fixture_runs_both(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "{id} must target both js and native, got {:?}",
        fixture.targets
    );
    assert_fixture_runs(id);
}

#[test]
fn tcp_listen_js_bridge_fixture_present() {
    assert_fixture_present("host/policy/tcp_listen_js_bridge");
}

#[test]
fn tcp_listen_js_bridge_on_js() {
    assert_fixture_runs_js("host/policy/tcp_listen_js_bridge");
}

#[test]
fn tcp_accept_js_bridge_fixture_present() {
    assert_fixture_present("host/policy/tcp_accept_js_bridge");
}

#[test]
fn tcp_accept_js_bridge_on_js() {
    assert_fixture_runs_js("host/policy/tcp_accept_js_bridge");
}

#[test]
fn open_file_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/open_file_js_hard_error");
}

#[test]
fn open_file_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/open_file_js_hard_error");
}

#[test]
fn http_parse_request_js_bridge_fixture_present() {
    assert_fixture_present("host/policy/http_parse_request_js_bridge");
}

#[test]
fn http_parse_request_js_bridge_on_js() {
    assert_fixture_runs_js("host/policy/http_parse_request_js_bridge");
}

#[test]
fn http_write_response_js_bridge_fixture_present() {
    assert_fixture_present("host/policy/http_write_response_js_bridge");
}

#[test]
fn http_write_response_js_bridge_on_js() {
    assert_fixture_runs_js("host/policy/http_write_response_js_bridge");
}

#[test]
fn http_write_request_js_bridge_fixture_present() {
    assert_fixture_present("host/policy/http_write_request_js_bridge");
}

#[test]
fn http_write_request_js_bridge_on_js() {
    assert_fixture_runs_js("host/policy/http_write_request_js_bridge");
}

#[test]
fn tls_client_wrap_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/tls_client_wrap_js_hard_error");
}

#[test]
fn tls_server_wrap_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/tls_server_wrap_js_hard_error");
}

#[test]
fn tls_client_wrap_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/tls_client_wrap_js_hard_error");
}

#[test]
fn tls_server_wrap_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/tls_server_wrap_js_hard_error");
}

#[test]
fn on_signal_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/on_signal_js_hard_error");
}

#[test]
fn on_signal_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/on_signal_js_hard_error");
}

#[test]
fn raise_signal_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/raise_signal_js_hard_error");
}

#[test]
fn raise_signal_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/raise_signal_js_hard_error");
}

#[test]
fn ignore_signal_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/ignore_signal_js_hard_error");
}

#[test]
fn ignore_signal_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/ignore_signal_js_hard_error");
}

#[test]
fn restore_signal_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/restore_signal_js_hard_error");
}

#[test]
fn restore_signal_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/restore_signal_js_hard_error");
}

#[test]
fn dns_lookup_js_bridge_fixture_present() {
    assert_fixture_present("host/policy/dns_lookup_js_bridge");
}

#[test]
fn dns_lookup_js_bridge_on_js() {
    assert_fixture_runs_js("host/policy/dns_lookup_js_bridge");
}

#[test]
fn make_once_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/make_once_js_hard_error");
}

#[test]
fn make_once_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/make_once_js_hard_error");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("host/policy/surface");
}

#[test]
fn surface_free_identifier_shape_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/policy/surface")
        .expect("host/policy/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "host/policy/surface must target js and native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("a/b\n"),
        "H00 surface must observe free-identifier pathJoin on native"
    );
    assert_fixture_runs_both("host/policy/surface");
}

#[test]
fn host_error_js_fixture_present() {
    assert_fixture_present("host/policy/host_error_js");
}

#[test]
fn host_error_js_is_catchable_name_and_code() {
    assert_fixture_runs_js("host/policy/host_error_js");
}

#[test]
fn surface_locks_host_policy() {
    assert_fixture_runs_both("host/policy/surface");
    assert_fixture_runs_js("host/policy/host_error_js");
    assert_fixture_runs_js("host/policy/tcp_listen_js_bridge");
    assert_fixture_runs_js("host/policy/tcp_accept_js_bridge");
    assert_fixture_runs_js("host/policy/dns_lookup_js_bridge");
    assert_fixture_runs_js("host/policy/http_parse_request_js_bridge");
    assert_fixture_runs_js("host/policy/http_write_response_js_bridge");
    assert_fixture_runs_js("host/policy/http_write_request_js_bridge");
    assert_fixture_runs_js("host/policy/tls_client_wrap_js_hard_error");
}
