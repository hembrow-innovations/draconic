---
id: "h05-time-clock-timers-job-queue"
title: "H05 Time, clock, timers (job-queue integrated)"
kind: task
status: completed
tags: []
created_at: "2026-09-02T22:16:51Z"
updated_at: "2026-09-03T05:16:34Z"
---

# H05 Time, clock, timers (job-queue integrated)

## Done

ROADMAP H05 is implemented test-first on both targets: a Program can read wall-clock time (real OS time on native via `Date.now` / host `now_ms`), measure durations with a monotonic clock, schedule and cancel `setTimeout` / `clearTimeout` through the job queue, and schedule and cancel `setInterval` / `clearInterval`, with the native run loop waiting for due timers instead of busy-spinning forever; `host/time` fixtures are green and H05 is `done`.

## Context

Roadmap ID **H05** (Time, clock, timers (job-queue integrated)). H05.01–H05.05 already land `Date.now` / `now_ms`, monotonic durations, `setTimeout` / `clearTimeout`, `setInterval` / `clearInterval`, and a run loop that waits for due timers; this sitting unifies them as one honest time surface on both targets. Tests under `tests/conformance/host/time`. Harness `tests/conformance/tests/host_time.rs`. Mark H05 `done` only when those tests are green. Not H05.01, H05.02, H05.03, H05.04, H05.05, H06, C05, or H00.

## Verify

`cargo test -p draconic-conformance --test host_time` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H05 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H05), `tests/conformance/fixtures/host/time`, `tests/conformance/tests/host_time.rs`, `crates/draconic-backend-llvm/src/host_time.rs`, `crates/draconic-backend-llvm/src/host_timers.rs`, `crates/draconic-runtime`, js/native time / timer paths as needed for the parent surface

## Links

[[s-h05]] [[ticket-36-h05-time-clock-timers-job-queue]]
