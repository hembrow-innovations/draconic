---
id: "k09-e2e-temp-git-dep-consumer"
title: "K09 E2E: temp git dep + consumer Program"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T13:20:51Z"
updated_at: "2026-09-04T13:59:48Z"
---

# K09 E2E: temp git dep + consumer Program

## Blocked by

None.

## Done

ROADMAP K09 is implemented test-first on the compiler target: a temp git dep (tagged lib) plus consumer manifest+lock resolve and fetch into cache, and a consumer Program builds while importing that module path from the fixture; packages tests `k09_01_resolve_fetch` and `k09_02_build_consumer` are green and K09 is `done`.

## Context

Roadmap ID **K09** (`E2E: temp git dep + consumer Program`). K09.01–K09.02 already land fixture resolve+fetch and consumer build/import; this sitting unifies them as one honest E2E package path on the compiler target. Tests in `tests/packages`. Harnesses `tests/packages/tests/k09_01_resolve_fetch.rs` and `tests/packages/tests/k09_02_build_consumer.rs`. Mark K09 `done` only when those tests are green. Not K09.01–K09.02 as separate atoms, K01–K08, K10, or K11.

## Verify

`cargo test -p draconic-packages-tests --test k09_01_resolve_fetch` prints `test result: ok.` `cargo test -p draconic-packages-tests --test k09_02_build_consumer` prints `test result: ok.` Workspace `cargo test --workspace` stays green. K09 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K09), `tests/packages/tests/k09_01_resolve_fetch.rs`, `tests/packages/tests/k09_02_build_consumer.rs`, compiler package E2E paths as needed for the parent surface

## Links

[[s-k09]] [[ticket-57-k09-e2e-temp-git-dep-consumer]]
