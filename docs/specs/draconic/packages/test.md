---
id: "test"
title: "Packages tests"
kind: test
description: "Which tests cover git modules, manifest, lock, cache, integrity, and offline build."
status: active
domain: draconic
area: packages
tags: [test]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Packages tests

Purpose: [[Packages purpose]]. Contract: [[Packages — Contract]].

## Coverage

These tests lock `packages.manifest:module-path-deps`, `packages.lock:pin-oid-hash`, `packages.cache:clone-checkout`, `packages.resolve:semver-fail-closed`, `packages.cli:get-and-tidy`, `packages.import:module-path-via-linker`, `packages.build:auto-fetch-offline`, `packages.integrity:refuse-tamper`, `packages.e2e:temp-git-consumer`, and `packages.later:k11-opt-in-not-v1-bar`. Asserted: `packages.forbid-central-registry`.

## Tests

- **crates/draconic-pkg/src/lib.rs** — `parse_module_and_deps` / `k01_combined_manifest_parse_write_validate_and_url_map` / `default_git_url_derives_https_module_path_git`
  - **How:** Parse/write `draconic.toml`; default URL is `https://{module_path}.git`.
  - **Why:** Locks `packages.manifest:module-path-deps` (K01).
- **crates/draconic-pkg/src/lock.rs** — `k02_combined_lockfile_resolved_pins` / `k02_03_rewrite_unchanged_is_byte_identical`
  - **How:** Lock entry fields round-trip; unchanged rewrite is byte-identical.
  - **Why:** Locks `packages.lock:pin-oid-hash` (K02).
- **crates/draconic-pkg/src/cache.rs** — `k03_combined_layout_clone_checkout_hash` / `checkout_cache_hit_skips_network`
  - **How:** Fixture git clone, checkout by OID, second checkout does not network.
  - **Why:** Locks `packages.cache:clone-checkout` (K03).
- **crates/draconic-pkg/src/resolve.rs** — `k04_combined_semver_tag_to_oid_fail_closed_direct_pins` / `no_match_returns_error`
  - **How:** Highest matching tag; no match is an error.
  - **Why:** Locks `packages.resolve:semver-fail-closed` (K04).
- **crates/draconic-cli/tests/get.rs** — `get_fetches_and_writes_manifest_lock`
  - **How:** `draconic get` updates manifest and lock from a tagged fixture.
  - **Why:** Locks `packages.cli:get-and-tidy` get path (K05.01).
- **crates/draconic-cli/tests/mod_tidy.rs** — `mod_tidy_writes_lock_from_manifest`
  - **How:** `draconic mod tidy` writes lock from manifest.
  - **Why:** Same promise, tidy path (K05.02).
- **crates/draconic-pkg/src/import_resolve.rs** — `looks_like_module_path_accepts_go_paths` / `resolve_rejects_escape_outside_package_root`
  - **How:** Go-like specifiers resolve; `..` escape is rejected.
  - **Why:** Locks `packages.import:module-path-via-linker` (K06).
- **tests/packages/tests/k06_03_coexist_relative.rs** — `entry_mixes_relative_and_module_path`
  - **How:** One Program mixes `./` imports and module-path imports.
  - **Why:** Coexist with E11 relative imports (K06.03).
- **crates/draconic-cli/tests/build.rs** — `build_auto_fetches_missing_locked_cache` / `build_offline_fails_when_cache_missing` / `build_offline_succeeds_when_cache_present` / `build_prefers_lock_pins_does_not_float`
  - **How:** Build fetches missing pins; `--offline` miss fails; warm cache succeeds; lock pins win.
  - **Why:** Locks `packages.build:auto-fetch-offline` (K07).
- **crates/draconic-pkg/src/hash.rs** — `k08_combined_verify_lock_hashes_refuse_tampered_cache`
  - **How:** Hash mismatch and OID mismatch refuse the tree.
  - **Why:** Locks `packages.integrity:refuse-tamper` (K08).
- **tests/integration/tests/supply_chain_lock_hash_mismatch.rs** — `compile_path_hard_fails_lock_hash_mismatch`
  - **How:** Frontend compile hard-fails when lock hash does not match.
  - **Why:** Same integrity promise at compile.
- **tests/integration/tests/supply_chain_tampered_cache.rs** — `compile_path_refuses_tampered_module_cache`
  - **How:** Tampered cache contents are refused.
  - **Why:** No silent wrong tree.
- **tests/packages/tests/k09.rs** — `k09_e2e_temp_git_dep_consumer_program`
  - **How:** Temp git lib + consumer resolve and fetch.
  - **Why:** Locks `packages.e2e:temp-git-consumer` (K09).
- **tests/packages/tests/k09_02_build_consumer.rs** — `k09_02_build_consumer_importing_module_path`
  - **How:** Build the consumer importing the fixture module path.
  - **Why:** v1 done bar includes K09.02.
- **crates/draconic-pkg/src/later.rs** — `k11_v1_surface_does_not_silently_ship_later_features`
  - **How:** Default classify is the v1 bar (no later knobs).
  - **Why:** Locks `packages.later:k11-opt-in-not-v1-bar`.

Support tests: K11.01 auth, replace, subdir, proxy, yank exist as opt-in later tests. They do not move the v1 bar.

## Gaps

- No test yet for promise `packages.forbid-central-registry`. The rejection is [[0009-go-style-git-packages]]; default HTTPS git URL tests imply git identity but do not name a registry forbid.
- Offline is opt-in on `build --offline` with a pre-warmed cache. There is no fully offline package ecosystem without cache or git. That is not an unimplemented v1 fetch path; it is the designed cache-only miss.
