---
id: issues-16
created_at: "2026-07-26"
updated_at: "2026-07-26"
area: planning
domain: language
title: "Expand Roadmap for end-to-end HTTP server"
description: "Add Roadmap rows so a Program can listen, accept, parse HTTP, and respond fully in Draconic (no C host)."
status: closed
issue-type: feature-request
severity: high
tags:
  - planning
  - issue
  - enhancement
  - closed
  - roadmap
  - native
  - runtime
  - http
  - phase-2
---

# Expand Roadmap for end-to-end HTTP server

## Description

Expand [`ROADMAP.md`](../../../ROADMAP.md) so the language product can host a real end-to-end HTTP server **written in Draconic** — listen on a port, accept connections, parse requests, run handler logic, write responses — on the native target (and portable policy for JS where applicable).

Today the flagship [[examples/todo]] ships a **C** static host (`server/main.c`) because the Runtime has no sockets/HTTP. Conformance is largely green; networking is not on the Roadmap at all.

## Affected

- `ROADMAP.md` (new section and/or N-children; Loop source of truth)
- `crates/draconic-runtime` (I/O, sockets, buffers, async integration)
- `crates/draconic-backend-llvm` / dual-worlds boundary
- JS backend policy for net APIs (`native-only` vs polyfill vs hard-error)
- Conformance / integration fixtures (new suite paths)
- `examples/todo` (eventual replace C host with Draconic server)
- Related planning: [[issues-3-native-depth]], [[issues-5-ship-real-program]], [[issues-7-new-roadmap-phase]]

## Observed

- `examples/todo/README.md`: *"The Draconic native backend does not yet expose sockets/HTTP in the Runtime, so the host is a small C server."*
- Roadmap sections B / E / T / N / U have no TCP, UDP, DNS, TLS, or HTTP rows.
- Native async job queue exists (N06.*); no non-blocking socket or host I/O bridge yet.
- No stdlib surface for `listen` / `accept` / byte streams / HTTP types.

## Impact

Without Roadmap atoms, **draconic-loop** cannot grow networking. The language cannot demonstrate a self-hosted server; product demos stay half-host/half-Draconic. Phase 2 “real program” work stays blocked on an undeclared stack.

## Proposed Fix

### 1. Human decisions (gate expansion)

1. **API shape (pick one primary):**
   - **A — Low-level sockets first:** TCP listen/accept/read/write + byte buffers; HTTP as userland or thin Runtime helpers later.
   - **B — HTTP server primitive first:** `serve(handler)` / request/response objects (Deno-like); sockets internal.
   - **C — Node-shaped `http`/`net`:** familiar surface; larger stdlib commitment.
2. **Targets:** `native` only for v1 | `both` with JS using host `http`/`net` or hard-error on unsupported.
3. **Scope of “done” for e2e:** HTTP/1.1 plaintext only | + chunked | + keep-alive | + TLS later as separate cluster.
4. **Success Program:** replace `examples/todo` C host **or** new `examples/http-echo` (or similar) built only via `draconic build --target native`.
5. **Phase placement:** new Roadmap section (e.g. **H — Host I/O & HTTP**) | N-children under native depth | seed under Phase 2 once [[issues-7-new-roadmap-phase]] lands.

### 2. Seed atomic Roadmap rows (after decisions)

Draft shape (adjust names/IDs to chosen API; each row one Loop; tests column required):

| Cluster | Example atoms (illustrative) |
|---------|------------------------------|
| **Buffers / bytes** | Uint8Array ↔ native buffer views usable at I/O boundary; concat/slice without full ES if already covered |
| **TCP** | bind/listen; accept → connection; read/write bytes; close; error surfaces |
| **Async I/O** | non-blocking or job-queue integration so `async` handlers don’t block the Runtime forever |
| **HTTP/1.1 server** | parse request line + headers + body (bounded); write status/headers/body; one connection one request minimum |
| **Handler API** | language-level server entry (per chosen shape A/B/C) |
| **E2E fixture** | conformance or integration: start server, client request, assert status/body, shutdown |
| **Example cutover** | Draconic static or API server replaces C in todo (or dedicated example) |

Optional later clusters (file as `todo` children, not v1 blockers): client `fetch`/TCP connect, TLS, HTTP/2, WebSockets, UDP, DNS.

### 3. Execution checklist (agent after human brief)

1. Add section + legend notes to `ROADMAP.md` without deleting B/E/T/N/U history.
2. Insert atomic `todo` rows with **Targets** and **Tests** paths (create empty fixture dirs only if harness already expects them; else name future paths).
3. ADR if API shape is non-obvious (link from issue Comments).
4. Point related issues: native depth, ship-real-program, Phase 2.
5. Do **not** implement sockets in the same change set as the Roadmap expansion unless a single pilot row is explicitly in scope.

## Human Brief

### Goal

Decide API shape, target policy, v1 HTTP scope, success demo, and Roadmap section home — then expand `ROADMAP.md` with Loop-sized `todo` rows that make a pure-Draconic e2e HTTP server achievable.

### Why human

Surface design (sockets vs `serve` vs Node-http), TLS timing, and whether JS must polyfill are product/architecture calls. Agents must not invent a permanent stdlib shape without this gate.

### Decisions needed

1. API shape: **A** sockets-first | **B** HTTP primitive | **C** Node-shaped (recommend **A** then thin HTTP helper, or **B** if demo speed matters more than layering).
2. Targets: native-only v1 vs both.
3. v1 protocol bar: plaintext HTTP/1.1 minimum features.
4. Success demo path and example home.
5. Roadmap home: **H** section vs N-children vs Phase 2.

### After decisions

- Agent expands `ROADMAP.md` with atomic rows + test paths.
- Optional ADR for chosen API.
- Loop implements rows test-first; example cutover when server cluster is green.

### Context (verified 2026-07-26)

- Todo example uses C HTTP/1.1 static host; client is Draconic→JS.
- N06 async/job queue on native is done; no socket ABI.
- [[issues-5-ship-real-program]] still human-gated on target choice — this issue **narrows** one concrete target (HTTP server) for Roadmap seeding.
- [[issues-7-new-roadmap-phase]] timing may affect section naming only.

### Out of scope

- Implementing the full server stack in one PR.
- Browser-only `fetch` without a server listen path.
- Claiming e2e done while any listen/accept/respond path still requires non-Draconic host code (except OS libc via Runtime).

## Comments

Related: [[issues-3-native-depth]], [[issues-5-ship-real-program]], [[issues-7-new-roadmap-phase]]

Evidence: `examples/todo/README.md` (C host rationale).

> **2026-07-26 closed:** Human gates locked and executed. Decisions: sockets-first (A), native first, plaintext HTTP/1.1 v1, success `examples/http-echo` + later todo C cutover, section **H**. Seeded Roadmap **H00–H17** (+ children); ADR-0008. Implementation of sockets is Loop work on those rows—not this issue.
