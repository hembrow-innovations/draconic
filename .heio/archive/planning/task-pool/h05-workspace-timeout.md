---
id: "h05-workspace-timeout"
title: "H05 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T16:30:27Z"
updated_at: "2026-09-04T16:55:57Z"
---

# H05 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H05 work; the host time conformance harness stays green.

## Context

Roadmap ID **H05** (Time, clock, timers (job-queue integrated)). Review of [[s-h05]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`host_time`) stayed green. If the hang comes from the H05 change, fix that time, clock, and job-queue timer surface so both the workspace check and the host time harness hold. Mark H05 `done` only when those tests are green. Not H05.01 wall clock `Date.now` / host `now_ms`, H05.02 monotonic clock for durations, H05.03 `setTimeout` / `clearTimeout` via job queue, H05.04 `setInterval` / `clearInterval`, H05.05 run loop waits for due timers, H06 TCP sockets, C05 structured cancellation / timeout helpers, or H00 host I/O surface policy. Do not re-open [[s-h05]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_time --offline && cargo test -p draconic-runtime --lib --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test host_time` still prints `test result: ok.` H05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H05), `tests/conformance/tests/host_time.rs`, `tests/conformance/fixtures/host/time`, `crates/draconic-backend-llvm/src/host_time.rs`, `crates/draconic-backend-llvm/src/host_timers.rs`, `crates/draconic-runtime`, time / clock / job-queue timer surface as needed to unhang workspace tests after H05

## Links

[[s-h05-workspace-timeout]] [[ticket-151-h05-workspace-timeout]] [[s-h05]]
