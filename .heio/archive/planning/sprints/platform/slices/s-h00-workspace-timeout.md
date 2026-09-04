---
id: "s-h00-workspace-timeout"
title: "H00 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T15:46:23Z"
updated_at: "2026-09-04T16:32:42Z"
claimed-by: 502ac06f-4cb7-453c-b0d2-f419c5be3de1
---

# H00 workspace tests finish

## Why

Review of [[s-h00]] left ROADMAP H00 unfinished: O1 (`host_policy`) and O2 (`draconic-runtime` lib) held, but O3 `cargo test --workspace` timed out at 120s. The host-io location still needs the H00 Loop to leave the workspace green, not only the host policy harness and Runtime ABI tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H00 work. The host policy conformance harness and the runtime crate tests stay green. If the hang comes from the H00 change, fix that host I/O surface policy so both checks hold. Mark H00 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h00]]**: that slice stays sealed `failed`
- **H00.01**: host API registry and js unsupported hard diagnostic (already `done`)
- **H00.02**: Runtime ABI scaffold (already `done`)
- **H00.03**: I/O bytes boundary (already `done`)
- **H01–H16**: concrete host ops (process, stdio, fs, sockets, HTTP)
- **H17.04**: optional JS/Node bridge for subset host APIs
- **R02**: permission grant/deny model

## Oracle checklist

- [x] O1: workspace tests finish after the H00 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_policy --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=e73db70d3a3230df bytes=103459 at=2026-09-04T16:32:16.230Z

- [x] O2: H00 js hard-error vs polyfill matrix stays locked by the host policy conformance tests
  CHECK: cargo test -p draconic-conformance --test host_policy
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=83c1d770dc947185 bytes=4397 at=2026-09-04T16:32:17.956Z

- [x] O3: H00 host error model and Runtime ABI stay locked by the runtime crate tests
  CHECK: cargo test -p draconic-runtime --lib
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=59500484046c5bf9 bytes=8305 at=2026-09-04T16:32:27.934Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h00-workspace-timeout]]`

## See also

ROADMAP.md H00, `tests/conformance/tests/host_policy.rs`, `tests/conformance/fixtures/host/policy`, `crates/draconic-runtime`, `crates/draconic-check/src/host_api.rs`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h00]], [[ticket-146-h00-workspace-timeout]].
