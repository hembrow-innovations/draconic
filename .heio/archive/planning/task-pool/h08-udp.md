---
id: "h08-udp"
title: "H08 UDP"
kind: task
status: completed
tags: []
created_at: "2026-09-02T13:27:17Z"
updated_at: "2026-09-03T05:16:34Z"
---

# H08 UDP

## Done

ROADMAP H08 is implemented test-first on native: bind a UDP socket (port 0 → ephemeral + query local port), sendto/recvfrom bytes, close the handle, and complete a loopback send + echo; `host/net/udp` fixtures are green and H08 is `done`.

## Context

Roadmap ID **H08** (UDP). H08.01–H08.02 already land bind/sendto/recvfrom/close and loopback e2e; this sitting unifies them as one honest UDP surface on native. Tests under `tests/conformance` fixtures `host/net/udp`. Harness `tests/conformance/tests/host_udp.rs`. Mark H08 `done` only when those tests are green. Not H06, H07, H09, H10, or H00.

## Verify

`cargo test -p draconic-conformance --test host_udp` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H08 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H08), `tests/conformance/fixtures/host/net/udp`, `tests/conformance/tests/host_udp.rs`, `crates/draconic-backend-llvm/src/host_udp.rs`, `crates/draconic-runtime`, native UDP paths as needed for the parent surface

## Links

[[s-h08]] [[ticket-39-h08-udp]]
