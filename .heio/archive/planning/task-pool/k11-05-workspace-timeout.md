---
id: "k11-05-workspace-timeout"
title: "K11.05 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T19:35:22Z"
updated_at: "2026-09-04T20:03:27Z"
---

# K11.05 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11.05 work; the `draconic-pkg` yank tests stay green.

## Context

Roadmap ID **K11.05** (Yank/retract when advisory source configured). Review of [[s-k11-05]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-pkg` yank) stayed green. If the hang comes from the K11.05 change, fix that yank/retract surface so both the workspace check and those crate tests hold. Mark K11.05 `done` only when those tests are green. Not K11 post-v1 packaging umbrella, K11.01 private git auth, K11.02 `replace` directive, K11.03 multi-module monorepo, K11.04 module proxy/mirror, or K08 lock-hash integrity / tampered cache (already a v1 bar). Do not re-open [[s-k11-05]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline yank` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-pkg yank` still prints `test result: ok.` K11.05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11.05), `crates/draconic-pkg`, yank/retract surface as needed to unhang workspace tests after K11.05

## Links

[[s-k11-05-workspace-timeout]] [[ticket-177-k11-05-workspace-timeout]] [[s-k11-05]]
