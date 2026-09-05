//! ROADMAP L03 / L03.01 / L03.02 / L10 / L10.01 / L10.02: SHA-256 digest, OS
//! CSPRNG bytes, HMAC-SHA256, and AES-256-GCM AEAD. L03 parent locks the
//! combined digest+CSPRNG surface in one Program; L10.01 locks HMAC-SHA256;
//! L10.02 locks AEAD encrypt/decrypt separately; L10 parent locks HMAC + AEAD
//! as one later crypto library in one Program.

use draconic_conformance::{fixtures_dir, load_fixtures, run_fixture, Target};

#[test]
fn sha256_vectors_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/crypto/sha256_vectors"),
        "missing stdlib/crypto/sha256_vectors fixture, got {ids:?}"
    );
}

#[test]
fn sha256_vectors_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/sha256_vectors")
        .expect("stdlib/crypto/sha256_vectors");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L03.01 targets both js and native"
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
fn sha256_invalid_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/crypto/sha256_invalid"),
        "missing stdlib/crypto/sha256_invalid fixture, got {ids:?}"
    );
}

#[test]
fn sha256_invalid_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/sha256_invalid")
        .expect("stdlib/crypto/sha256_invalid");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L03.01 targets both js and native"
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
fn random_bytes_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/crypto/random_bytes"),
        "missing stdlib/crypto/random_bytes fixture, got {ids:?}"
    );
}

#[test]
fn random_bytes_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/random_bytes")
        .expect("stdlib/crypto/random_bytes");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L03.02 targets both js and native"
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
fn random_bytes_invalid_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/crypto/random_bytes_invalid"),
        "missing stdlib/crypto/random_bytes_invalid fixture, got {ids:?}"
    );
}

#[test]
fn random_bytes_invalid_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/random_bytes_invalid")
        .expect("stdlib/crypto/random_bytes_invalid");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L03.02 targets both js and native"
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
        ids.iter().any(|id| *id == "stdlib/crypto/surface"),
        "missing stdlib/crypto/surface fixture, got {ids:?}"
    );
}

#[test]
fn surface_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/surface")
        .expect("stdlib/crypto/surface");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "stdlib/crypto/surface must target js and native"
    );
    for name in ["sha256", "randomBytes", "TypeError", "RangeError"] {
        assert!(
            fixture.source.contains(name),
            "L03 surface must use {name} in one Program"
        );
    }
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("true\ntrue\ntrue\ntrue\ntrue\n1\n1\n1\n"),
        "L03 surface must observe SHA-256 vector, CSPRNG length, composed digest, and invalid-input errors"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "L03 surface must terminate with exit 0"
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
fn hmac_sha256_vectors_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/crypto/hmac_sha256_vectors"),
        "missing stdlib/crypto/hmac_sha256_vectors fixture, got {ids:?}"
    );
}

#[test]
fn hmac_sha256_vectors_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/hmac_sha256_vectors")
        .expect("stdlib/crypto/hmac_sha256_vectors");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L10.01 targets both js and native"
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
fn hmac_sha256_invalid_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter()
            .any(|id| *id == "stdlib/crypto/hmac_sha256_invalid"),
        "missing stdlib/crypto/hmac_sha256_invalid fixture, got {ids:?}"
    );
}

#[test]
fn hmac_sha256_invalid_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/hmac_sha256_invalid")
        .expect("stdlib/crypto/hmac_sha256_invalid");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L10.01 targets both js and native"
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
fn aead_vectors_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/crypto/aead_vectors"),
        "missing stdlib/crypto/aead_vectors fixture, got {ids:?}"
    );
}

#[test]
fn aead_vectors_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/aead_vectors")
        .expect("stdlib/crypto/aead_vectors");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L10.02 targets both js and native"
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
fn aead_roundtrip_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/crypto/aead_roundtrip"),
        "missing stdlib/crypto/aead_roundtrip fixture, got {ids:?}"
    );
}

#[test]
fn aead_roundtrip_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/aead_roundtrip")
        .expect("stdlib/crypto/aead_roundtrip");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L10.02 targets both js and native"
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
fn aead_invalid_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/crypto/aead_invalid"),
        "missing stdlib/crypto/aead_invalid fixture, got {ids:?}"
    );
}

#[test]
fn aead_invalid_runs_both_targets() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/aead_invalid")
        .expect("stdlib/crypto/aead_invalid");
    assert_eq!(
        fixture.targets.len(),
        2,
        "L10.02 targets both js and native"
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
fn hmac_aead_fixture_present() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load fixtures");
    let ids: Vec<_> = fixtures.iter().map(|f| f.id.as_str()).collect();
    assert!(
        ids.iter().any(|id| *id == "stdlib/crypto/hmac_aead"),
        "missing stdlib/crypto/hmac_aead fixture, got {ids:?}"
    );
}

#[test]
fn hmac_aead_runs_js_and_native() {
    let fixtures = load_fixtures(&fixtures_dir()).expect("load");
    let fixture = fixtures
        .iter()
        .find(|f| f.id == "stdlib/crypto/hmac_aead")
        .expect("stdlib/crypto/hmac_aead");
    assert!(
        fixture.targets.contains(&Target::Js) && fixture.targets.contains(&Target::Native),
        "stdlib/crypto/hmac_aead must target js and native"
    );
    for name in [
        "hmacSha256",
        "aeadEncrypt",
        "aeadDecrypt",
        "TypeError",
        "RangeError",
    ] {
        assert!(
            fixture.source.contains(name),
            "L10 combined surface must use {name} in one Program"
        );
    }
    assert_eq!(
        fixture.expect_native.stdout.as_deref(),
        Some("true\ntrue\ntrue\ntrue\ntrue\n1\n1\n1\n"),
        "L10 surface must observe HMAC vector, AEAD roundtrip, and invalid-input errors"
    );
    assert_eq!(
        fixture.expect_native.exit, 0,
        "L10 surface must terminate with exit 0"
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
