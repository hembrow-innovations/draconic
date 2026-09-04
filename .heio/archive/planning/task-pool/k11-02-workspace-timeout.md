---
id: "k11-02-workspace-timeout"
title: "K11.02 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T19:10:34Z"
updated_at: "2026-09-04T19:19:41Z"
---

# K11.02 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11.02 work; the `draconic-pkg` replace fork and local override tests stay green.

## Context

Roadmap ID **K11.02** (`replace` directive: fork/local override). Review of [[s-k11-02]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-pkg` replace) stayed green. If the hang comes from the K11.02 change, fix that `replace` directive surface so both the workspace check and those crate tests hold. Mark K11.02 `done` only when those tests are green. Not K11 post-v1 packaging umbrella, K11.01 private git auth, K11.03 multi-module monorepo, K11.04 module proxy/mirror, K11.05 yank/retract, or K01 manifest parse/write/url-map (already landed). Do not re-open [[s-k11-02]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline replace` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-pkg replace` still prints `test result: ok.` K11.02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11.02), `crates/draconic-pkg`, `replace` directive fork/local override surface as needed to unhang workspace tests after K11.02

## Links

[[s-k11-02-workspace-timeout]] [[ticket-174-k11-02-workspace-timeout]] [[s-k11-02]]
