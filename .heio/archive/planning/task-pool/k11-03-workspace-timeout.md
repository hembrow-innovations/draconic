---
id: "k11-03-workspace-timeout"
title: "K11.03 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T19:17:15Z"
updated_at: "2026-09-04T19:23:46Z"
---

# K11.03 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11.03 work; the `draconic-pkg` subdir module path tests stay green.

## Context

Roadmap ID **K11.03** (Multi-module monorepo (subdir module paths)). Review of [[s-k11-03]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-pkg` subdir) stayed green. If the hang comes from the K11.03 change, fix that subdir module-path surface so both the workspace check and those crate tests hold. Mark K11.03 `done` only when those tests are green. Not K11 post-v1 packaging umbrella, K11.01 private git auth, K11.02 `replace` directive, K11.04 module proxy/mirror, K11.05 yank/retract, K03 module cache layout / git clone at repo root (already landed), or K06 import resolve / package boundary for a single-module checkout. Do not re-open [[s-k11-03]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline subdir` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-pkg subdir` still prints `test result: ok.` K11.03 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11.03), `crates/draconic-pkg`, subdir module-path surface as needed to unhang workspace tests after K11.03

## Links

[[s-k11-03-workspace-timeout]] [[ticket-175-k11-03-workspace-timeout]] [[s-k11-03]]
