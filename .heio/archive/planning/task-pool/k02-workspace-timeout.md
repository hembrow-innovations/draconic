---
id: "k02-workspace-timeout"
title: "K02 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T18:20:24Z"
updated_at: "2026-09-04T18:26:31Z"
---

# K02 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K02 work; the `draconic-pkg` lock tests for `draconic.lock` resolved pins stay green.

## Context

Roadmap ID **K02** (Lockfile (`draconic.lock`): resolved pins). Review of [[s-k02]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-pkg` lock) stayed green. If the hang comes from the K02 change, fix that lockfile surface so both the workspace check and the `draconic-pkg` lock tests for `draconic.lock` resolved pins (path + version + git URL + commit OID + content hash SHA-256, parse/write reject-malformed, stable serialize) hold. Mark K02 `done` only when those tests are green. Not K02.01 lock entry fields, K02.02 parse/write reject-malformed, K02.03 stable lock serialize, K01 manifest (`draconic.toml`), K03 module cache layout / git clone, K04 version resolve, or K08 integrity verify. Do not re-open [[s-k02]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline lock` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-pkg lock` still prints `test result: ok.` K02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K02), `crates/draconic-pkg`, `crates/draconic-pkg/src/lock.rs`, lockfile parse/write/serialize surface as needed to unhang workspace tests after K02

## Links

[[s-k02-workspace-timeout]] [[ticket-166-k02-workspace-timeout]] [[s-k02]]
