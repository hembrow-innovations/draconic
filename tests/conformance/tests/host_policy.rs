//! ROADMAP H00.01 / H06.06 / H09.03 / H10.07: host API registry — js unsupported → hard diagnostic.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

fn assert_fixture_present(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|x| *x == id),
        "missing {id} fixture, got {ids:?}"
    );
}

fn assert_fixture_runs_js(id: &str) {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures.iter().find(|f| f.id == id).expect(id);
    assert!(
        fixture.targets.contains(&Target::Js),
        "{id} must target js"
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
fn tcp_listen_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/tcp_listen_js_hard_error");
}

#[test]
fn tcp_listen_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/tcp_listen_js_hard_error");
}

#[test]
fn tcp_accept_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/tcp_accept_js_hard_error");
}

#[test]
fn tcp_accept_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/tcp_accept_js_hard_error");
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
fn http_parse_request_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/http_parse_request_js_hard_error");
}

#[test]
fn http_parse_request_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/http_parse_request_js_hard_error");
}

#[test]
fn http_write_response_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/http_write_response_js_hard_error");
}

#[test]
fn http_write_response_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/http_write_response_js_hard_error");
}

#[test]
fn http_request_header_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/http_request_header_js_hard_error");
}

#[test]
fn http_request_header_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/http_request_header_js_hard_error");
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
fn dns_lookup_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/dns_lookup_js_hard_error");
}

#[test]
fn dns_lookup_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/dns_lookup_js_hard_error");
}

#[test]
fn make_once_js_hard_error_fixture_present() {
    assert_fixture_present("host/policy/make_once_js_hard_error");
}

#[test]
fn make_once_js_hard_error_on_js() {
    assert_fixture_runs_js("host/policy/make_once_js_hard_error");
}
