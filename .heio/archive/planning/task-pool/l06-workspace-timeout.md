---
id: "l06-workspace-timeout"
title: "L06 workspace tests finish"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-04T21:03:02Z"
updated_at: "2026-09-04T23:15:26Z"
---

# L06 workspace tests finish

## Blocked by

None.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP L06 work; the stdlib logging conformance tests stay green.

## Context

Roadmap ID **L06** (Logging: leveled logger; stderr/stdout sink). Review of [[s-l06]] left O2 unmet: `cargo test --workspace` timed out at 120s while O1 (`stdlib_logging`) stayed green. The stdlib location still needs the L06 Loop to leave the workspace green, not only the leveled-logger and stderr/stdout sink fixtures. If the hang comes from the L06 change, fix that leveled-logger / stdio-sink surface so the workspace check and those fixtures hold. Mark L06 `done` only when those tests are green. Not L06.01–L06.02 (already `done`), H02 host stdio, L05 testing, structured JSON logs, file sinks, syslog, or a Node `console` identity surface. Do not re-open [[s-l06]].

## Verify

`cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test stdlib_logging --offline` prints `test result: ok.` and finishes (does not hang). `cargo test -p draconic-conformance --test stdlib_logging` still prints `test result: ok.` L06 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L06), `tests/conformance/tests/stdlib_logging.rs`, `tests/conformance/fixtures/stdlib/logging`, `crates/draconic-backend-llvm/src/es_logging.rs`, `crates/draconic-runtime/src/logging.rs`, leveled-logger / stdio-sink surface as needed to unhang workspace tests after L06

## Links

[[s-l06-workspace-timeout]] [[ticket-184-l06-workspace-timeout]] [[s-l06]]
