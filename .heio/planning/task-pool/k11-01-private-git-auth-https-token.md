---
id: "k11-01-private-git-auth-https-token"
title: "K11.01 Private git auth (HTTPS token / SSH)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:25:04Z"
updated_at: "2026-09-02T22:25:04Z"
---

# K11.01 Private git auth (HTTPS token / SSH)

## Done

ROADMAP K11.01 is implemented test-first on the compiler target: package fetch authenticates to a private git host with an HTTPS token or SSH (as designed); missing or rejected credentials fail closed with a clear diagnostic and do not write secrets into `draconic.toml` or `draconic.lock`; `draconic-pkg` and `draconic-cli` k11_01 tests are green and K11.01 is `done`.

## Context

Roadmap ID **K11.01** (Private git auth (HTTPS token / SSH)). Post-v1 packaging on the compiler target (ADR-0009): fetch of a private module may use an HTTPS token or SSH credentials so clone/fetch is not limited to anonymous HTTPS. Tests in `crates/draconic-pkg` and `crates/draconic-cli` lock that surface. Mark K11.01 `done` only when those tests are green. Not K11 umbrella, K11.02 replace, K11.03 monorepo subdir paths, K11.04 proxy/mirror, K11.05 yank, K03.02 anonymous HTTPS clone/fetch, or npm registry tokens / crates.io API keys as the primary auth shape.

## Verify

`cargo test -p draconic-pkg k11_01` prints `test result: ok.` `cargo test -p draconic-cli k11_01` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K11.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11.01), `crates/draconic-pkg`, `crates/draconic-cli`

## Links

[[s-k11-01]] [[ticket-59-k11-01-private-git-auth-https-token]]
