---
id: "k11-04-module-proxy-mirror-git-still"
title: "K11.04 Module proxy/mirror (git still canonical)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:27:22Z"
updated_at: "2026-09-02T22:27:22Z"
---

# K11.04 Module proxy/mirror (git still canonical)

## Done

ROADMAP K11.04 is implemented test-first on the compiler target: a configured module proxy/mirror can serve fetch while module path identity, lock OID, and tree hash stay git-canonical; a missing or failing proxy does not rewrite identity to the mirror URL; `draconic-pkg` k11_04 tests are green and K11.04 is `done`.

## Context

Roadmap ID **K11.04** (Module proxy/mirror (git still canonical)). Post-v1 packaging on the compiler target (ADR-0009): fetch may go through a configured proxy or mirror (Athens/GOPROXY-shaped) while git remains the canonical module identity and source of truth — lock pins and content hashes still name the git tree, not the mirror. Tests in `crates/draconic-pkg` lock that split. Mark K11.04 `done` only when those tests are green. Not K11 umbrella, K11.01 private git auth, K11.02 `replace`, K11.03 monorepo subdir paths, K11.05 yank, K01.04 path→git URL map, or a central npm/crates.io registry as canonical source.

## Verify

`cargo test -p draconic-pkg k11_04` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K11.04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11.04), `crates/draconic-pkg`

## Links

[[s-k11-04]] [[ticket-62-k11-04-module-proxy-mirror-git-still]]
