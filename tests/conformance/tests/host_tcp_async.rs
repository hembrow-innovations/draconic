//! ROADMAP H07 / H07.02–H07.03: async TCP Promises; concurrent connections.
//! H07 parent locks the combined async TCP surface in one Program.

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

#[test]
fn tcp_async_accept_connect_runs_native() {
    assert_fixture_present("host/net/async/tcp_async_accept_connect");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/async/tcp_async_accept_connect")
        .expect("host/net/async/tcp_async_accept_connect");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/async/tcp_async_accept_connect");
}

#[test]
fn tcp_async_read_write_runs_native() {
    assert_fixture_present("host/net/async/tcp_async_read_write");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/async/tcp_async_read_write")
        .expect("host/net/async/tcp_async_read_write");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/async/tcp_async_read_write");
}

#[test]
fn tcp_async_cancel_close_runs_native() {
    assert_fixture_present("host/net/async/tcp_async_cancel_close");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/async/tcp_async_cancel_close")
        .expect("host/net/async/tcp_async_cancel_close");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/async/tcp_async_cancel_close");
}

#[test]
fn tcp_async_concurrent_runs_native() {
    assert_fixture_present("host/net/async/tcp_async_concurrent");
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/async/tcp_async_concurrent")
        .expect("host/net/async/tcp_async_concurrent");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_fixture_runs("host/net/async/tcp_async_concurrent");
}

#[test]
fn surface_fixture_present() {
    assert_fixture_present("host/net/async/surface");
}

#[test]
fn surface_runs_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "host/net/async/surface")
        .expect("host/net/async/surface");
    assert!(
        fixture.targets.contains(&Target::Native),
        "must target native"
    );
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("1\n0\n1\n1\n4\n4\n2\n2\n2\n2\nfunction\nfunction\nfunction\nfunction\n"),
        "H07 surface must observe cancel/close, async accept/connect, async r/w, concurrent, and API typeof"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "H07 surface must terminate with exit 0"
    );
    assert_fixture_runs("host/net/async/surface");
}
