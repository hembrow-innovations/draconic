---
id: "k09-workspace-timeout"
title: "K09 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T19:03:09Z"
updated_at: "2026-09-04T19:11:36Z"
---

# K09 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K09 work; the `draconic-packages-tests` k09_01 resolve+fetch and k09_02 consumer build/import tests stay green.

## Context

Roadmap ID **K09** (E2E: temp git dep + consumer Program). Review of [[s-k09]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`k09_01_resolve_fetch`) and O2 (`k09_02_build_consumer`) stayed green. If the hang comes from the K09 change, fix that temp git dep + consumer Program E2E surface so both the workspace check and those package tests hold. Mark K09 `done` only when those tests are green. Not K09.01 fixture temp git lib / consumer manifest+lock / resolve+fetch, K09.02 E2E build consumer importing module path from fixture, K01–K08 manifest/lock/cache/resolve/CLI/build/integrity, K10 demo package, or K11 post-v1 packaging. Do not re-open [[s-k09]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-packages-tests --test k09_01_resolve_fetch --offline && cargo test -p draconic-packages-tests --test k09_02_build_consumer --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-packages-tests --test k09_01_resolve_fetch` still prints `test result: ok.` `cargo test -p draconic-packages-tests --test k09_02_build_consumer` still prints `test result: ok.` K09 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K09), `tests/packages/tests/k09_01_resolve_fetch.rs`, `tests/packages/tests/k09_02_build_consumer.rs`, temp git dep + consumer Program E2E surface as needed to unhang workspace tests after K09

## Links

[[s-k09-workspace-timeout]] [[ticket-172-k09-workspace-timeout]] [[s-k09]]
