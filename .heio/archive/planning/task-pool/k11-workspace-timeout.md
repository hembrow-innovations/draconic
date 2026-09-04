---
id: "k11-workspace-timeout"
title: "K11 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T19:39:17Z"
updated_at: "2026-09-04T20:06:48Z"
---

# K11 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11 work; the `draconic-pkg` k11 tests stay green.

## Context

Roadmap ID **K11** (Post-v1 packaging (not v1 bar)). Review of [[s-k11]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-pkg` k11) stayed green. If the hang comes from the K11 change, fix that later-not-v1 packaging surface so both the workspace check and those crate tests hold. Mark K11 `done` only when those tests are green. Not K11.01 private git auth, K11.02 `replace` directive, K11.03 multi-module monorepo, K11.04 module proxy/mirror, K11.05 yank/retract, or K01–K10 (v1 bar and demo). Do not re-open [[s-k11]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline k11` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-pkg k11` still prints `test result: ok.` K11 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11), `crates/draconic-pkg`, later-not-v1 packaging surface as needed to unhang workspace tests after K11

## Links

[[s-k11-workspace-timeout]] [[ticket-178-k11-workspace-timeout]] [[s-k11]]
