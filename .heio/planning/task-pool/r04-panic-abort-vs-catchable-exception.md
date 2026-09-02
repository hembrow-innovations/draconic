---
id: "r04-panic-abort-vs-catchable-exception"
title: "R04 Panic/abort vs catchable exception policy; fixtures per class"
kind: task
status: ready
tags: []
created_at: "2026-09-02T22:34:01Z"
updated_at: "2026-09-02T22:34:01Z"
---

# R04 Panic/abort vs catchable exception policy; fixtures per class

## Done

ROADMAP R04 is implemented test-first on the native target: fixtures under `tests/conformance/fixtures/security/panic_policy` lock catchable vs abort per class, Runtime tests lock that `draconic_rt_abort` and invariant failures abort the process, and R04 is `done`.

## Context

Roadmap ID **R04** (Panic/abort vs catchable exception policy; fixtures per class). Runtime-hardening location: the parent row that the combined panic/abort vs catchable exception policy surface is honest on native. R04.01 and R04.02 already land the two classes (ADR-0011); this sitting unifies them. Tests under `tests/conformance` fixtures `security/panic_policy` and `crates/draconic-runtime`. Harnesses `cargo test -p draconic-runtime --lib abort_policy` and `cargo test -p draconic-conformance --test panic_policy`. Mark R04 `done` only when those tests are green. Not R04.01–R04.02 as separate atoms, R01 embed limits, R05 fuzz/stress, R06 panic backtraces, or N09 GC stress.

## Verify

`cargo test -p draconic-runtime --lib abort_policy` prints `test result: ok.` `cargo test -p draconic-conformance --test panic_policy` prints `test result: ok.` Workspace `cargo test --workspace` stays green. R04 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (R04), `crates/draconic-runtime`, `tests/conformance/fixtures/security/panic_policy`, `tests/conformance/tests/panic_policy.rs`

## Links

[[s-r04]] [[ticket-113-r04-panic-abort-vs-catchable-exception]]
