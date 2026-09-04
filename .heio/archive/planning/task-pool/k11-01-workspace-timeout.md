---
id: "k11-01-workspace-timeout"
title: "K11.01 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T19:07:28Z"
updated_at: "2026-09-04T19:15:33Z"
---

# K11.01 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11.01 work; the `draconic-pkg` k11_01 private git HTTPS token / SSH auth tests and the `draconic-cli` k11_01 CLI surface tests stay green.

## Context

Roadmap ID **K11.01** (Private git auth (HTTPS token / SSH)). Review of [[s-k11-01]] left O3 unmet: `cargo test --workspace` timed out at 120s while O1 (`draconic-pkg` k11_01) and O2 (`draconic-cli` k11_01) stayed green. If the hang comes from the K11.01 change, fix that private git auth surface so both the workspace check and those crate tests hold. Mark K11.01 `done` only when those tests are green. Not K11 post-v1 packaging umbrella, K11.02 `replace` directive, K11.03 multi-module monorepo, K11.04 module proxy/mirror, K11.05 yank/retract, or K03.02 git clone/fetch into cache (already `done`; public/anonymous). Do not re-open [[s-k11-01]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline k11_01 && cargo test -p draconic-cli --test k11_01 --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-pkg k11_01` still prints `test result: ok.` `cargo test -p draconic-cli --test k11_01` still prints `test result: ok.` K11.01 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (K11.01), `crates/draconic-pkg`, `crates/draconic-cli`, private git HTTPS token / SSH auth surface as needed to unhang workspace tests after K11.01

## Links

[[s-k11-01-workspace-timeout]] [[ticket-173-k11-01-workspace-timeout]] [[s-k11-01]]
