---
id: "l06-logging-leveled-logger-stderr-stdout-sink"
title: "L06 Logging: leveled logger; stderr/stdout sink"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:47:19Z"
updated_at: "2026-09-04T20:55:53Z"
---

# L06 Logging: leveled logger; stderr/stdout sink

## Done

ROADMAP L06 is implemented test-first on both targets: a Program can create a leveled logger, filter by level, and sink formatted lines to stderr/stdout as designed; `stdlib/logging` fixtures lock that combined surface and L06 is `done`.

## Context

Roadmap ID **L06** (Logging: leveled logger; stderr/stdout sink). Stdlib location: honest portable libs a simple service needs. L06.01 and L06.02 already land `createLogger` with error/warn/info/debug plus level filter, and the string-format stdio sink (debug/info → stdout, warn/error → stderr); this sitting unifies them as one logging library a Program can use. Tests under `tests/conformance` fixtures `stdlib/logging`. Harness `tests/conformance/tests/stdlib_logging.rs`. Mark L06 `done` only when those tests are green. Not L06.01, L06.02, H02 host stdio, L05 testing, structured JSON logs, file sinks, syslog, or a Node `console` identity surface.

## Verify

`cargo test -p draconic-conformance --test stdlib_logging` prints `test result: ok.` Workspace `cargo test --workspace` stays green. L06 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (L06), `tests/conformance/fixtures/stdlib/logging`, `tests/conformance/tests/stdlib_logging.rs`, `crates/draconic-backend-llvm/src/es_logging.rs`, `crates/draconic-runtime/src/logging.rs`, stdlib logging surface as needed for both targets

## Links

[[s-l06]] [[ticket-84-l06-logging-leveled-logger-stderr-stdout]]
