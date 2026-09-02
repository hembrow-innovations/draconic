---
id: "h02-stdio-stdout-stderr-stdin"
title: "H02 stdio stdout / stderr / stdin"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:25:28Z"
updated_at: "2026-09-02T13:25:28Z"
---

# H02 stdio stdout / stderr / stdin

## Done

ROADMAP H02 is implemented test-first on both targets: write a string plus newline and bytes via `Uint8Array` to stdout, write to stderr, and read stdin as a line or bounded bytes (v1 blocking ok on native); `host/stdio` fixtures are green and H02 is `done`.

## Context

Roadmap ID **H02** (Stdio: stdout / stderr / stdin). H02.01–H02.03 already land stdout write, stderr write, and stdin read; this sitting unifies them as one honest stdio surface on both targets. Tests under `tests/conformance` fixtures `host/stdio`. Harness `tests/conformance/tests/host_stdio.rs`. Mark H02 `done` only when those tests are green. Not H00, H01, or L06.

## Verify

`cargo test -p draconic-conformance --test host_stdio` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H02 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H02), `tests/conformance/fixtures/host/stdio`, `tests/conformance/tests/host_stdio.rs`, `crates/draconic-runtime`, `crates/draconic-backend-llvm/src/host_stdio.rs`, js/native stdio paths as needed for the parent surface

## Links

[[s-h02]] [[ticket-33-h02-stdio-stdout-stderr-stdin]]
