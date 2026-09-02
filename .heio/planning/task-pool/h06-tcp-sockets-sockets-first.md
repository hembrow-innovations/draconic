---
id: "h06-tcp-sockets-sockets-first"
title: "H06 TCP sockets (sockets-first)"
kind: task
status: ready
tags: []
created_at: "2026-09-02T13:26:03Z"
updated_at: "2026-09-02T13:26:03Z"
---

# H06 TCP sockets (sockets-first)

## Done

ROADMAP H06 is implemented test-first on native: listen (bind, backlog, close; port 0 → ephemeral + query local port), accept with peer address, connect (refused/timeout typed errors), read/write/partial/close/shutdown, loopback echo, and js listen/accept hard-error; `host/net/tcp` fixtures are green and H06 is `done`.

## Context

Roadmap ID **H06** (TCP sockets, sockets-first). H06.01–H06.06 already land listen/bind/backlog/ephemeral port, accept + peer address, connect with refused/timeout errors, read/write/partial/close/shutdown, loopback echo, and js listen/accept hard-error; this sitting unifies them as one honest TCP surface on native. Tests under `tests/conformance` fixtures `host/net/tcp`. Harness `tests/conformance/tests/host_tcp.rs`. Mark H06 `done` only when those tests are green. Not H07, H08, H09, H10, or H00.

## Verify

`cargo test -p draconic-conformance --test host_tcp` prints `test result: ok.` Workspace `cargo test --workspace` stays green. H06 is `done` on ROADMAP.md.

scope: `ROADMAP.md` (H06), `tests/conformance/fixtures/host/net/tcp`, `tests/conformance/tests/host_tcp.rs`, `crates/draconic-backend-llvm/src/host_tcp.rs`, `crates/draconic-runtime`, js/native TCP paths as needed for the parent surface

## Links

[[s-h06]] [[ticket-37-h06-tcp-sockets-sockets-first]]
