---
id: "contract"
title: "Packages — Contract"
kind: contract
description: "Durable promises for git module paths, manifest, lock, cache, integrity, and offline build."
status: active
domain: draconic
area: packages
tags: [contract]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Packages — Contract

A promise with a `test:` pointer is locked. One without is asserted. Purpose: [[Packages purpose]]. Coverage map: [[Packages tests]].

## Behaviour

- `packages.manifest:module-path-deps`: `draconic.toml` parses and writes a Go-like module path, a deps map (path → version req), and an optional path→git URL map; default git URL is `https://{module_path}.git`.
  test: parse_module_and_deps
  test: k01_combined_manifest_parse_write_validate_and_url_map
  test: default_git_url_derives_https_module_path_git
- `packages.lock:pin-oid-hash`: `draconic.lock` stores path, concrete version, git URL, commit OID, and SHA-256 content hash; serialize is sorted and byte-identical when unchanged.
  test: k02_combined_lockfile_resolved_pins
  test: k02_03_rewrite_unchanged_is_byte_identical
- `packages.cache:clone-checkout`: Cache layout is keyed by module path and commit OID; git clone/fetch fills the VCS store; checkout of a pinned OID hits cache without network.
  test: k03_combined_layout_clone_checkout_hash
  test: checkout_cache_hit_skips_network
- `packages.resolve:semver-fail-closed`: Version resolve picks the highest matching semver git tag and fails closed on no match, non-semver-only tags, or empty tags; v1 lock fill is direct deps only.
  test: k04_combined_semver_tag_to_oid_fail_closed_direct_pins
  test: no_match_returns_error
- `packages.cli:get-and-tidy`: `draconic get <module_path>@<ver>` fetches and updates manifest, lock, and cache; `draconic mod tidy` aligns lock to manifest, fetches missing, and prunes unused.
  test: get_fetches_and_writes_manifest_lock
  test: mod_tidy_writes_lock_from_manifest
  test: k05_combined_get_and_mod_tidy_cli
- `packages.import:module-path-via-linker`: A module-path import (plus subpath) resolves to a file under the cached package root; relative ESM imports still work; path escape outside the package root is rejected.
  test: looks_like_module_path_accepts_go_paths
  test: resolve_package_root_index
  test: resolve_rejects_escape_outside_package_root
  test: entry_mixes_relative_and_module_path
  test: frontend_compile_mixed_relative_and_module_path
- `packages.build:auto-fetch-offline`: `draconic build` auto-fetches missing locked cache entries; `--offline` is cache only and fails closed on miss; a present lock does not float versions.
  test: build_auto_fetches_missing_locked_cache
  test: build_offline_fails_when_cache_missing
  test: build_offline_succeeds_when_cache_present
  test: build_prefers_lock_pins_does_not_float
  test: k07_combined_build_auto_fetch_offline_lock_pins
- `packages.integrity:refuse-tamper`: Recompute tree SHA-256 against the lock; mismatched hash or checkout OID hard-fails; no silent wrong tree.
  test: k08_combined_verify_lock_hashes_refuse_tampered_cache
  test: compile_path_hard_fails_lock_hash_mismatch
  test: compile_path_refuses_tampered_module_cache
- `packages.e2e:temp-git-consumer`: A tagged temp-git lib plus consumer Program resolves, fetches, and builds an import of that module path.
  test: k09_e2e_temp_git_dep_consumer_program
  test: k09_02_build_consumer_importing_module_path
- `packages.later:k11-opt-in-not-v1-bar`: Private git auth, replace, monorepo subdir, proxy/mirror, and yank are opt-in later knobs; they are not the silent v1 fetch/resolve path.
  test: k11_v1_surface_does_not_silently_ship_later_features
- `packages.forbid-central-registry`: v1 package identity is git-backed module paths; there is no npm-like central registry as the primary source.
