---
id: "s-h09-workspace-timeout"
title: "H09 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T17:05:32Z"
updated_at: "2026-09-04T17:26:14Z"
claimed-by: ced75cbd-81c0-4ccf-b64d-d49b75af07af
---

# H09 workspace tests finish

## Why

Review of [[s-h09]] left ROADMAP H09 unfinished: O1 (`host_dns`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H09 Loop to leave the workspace green, not only the host DNS conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H09 work. The host DNS conformance harness stays green. If the hang comes from the H09 change, fix that DNS lookup and connect-by-name surface so both checks hold. Mark H09 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h09]]**: that slice stays sealed `failed`
- **H09.01**: DNS lookup hostname → addresses; failure errors (already `done`)
- **H09.02**: Connect-by-name (H09.01 + H06.03) (already `done`)
- **H09.03**: DNS on js hard-error / deferred Node polyfill (already `done`)
- **H06**: TCP listen/accept/connect/read/write
- **H08**: UDP
- **H10**: HTTP/1.1 thin helpers
- **H17.04**: optional JS/Node bridge for subset host APIs

## Oracle checklist

- [x] O1: workspace tests finish after the H09 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_dns --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=8f629538e5b99891 bytes=102343 at=2026-09-04T17:25:39.350Z

- [x] O2: H09 DNS lookup and connect-by-name stay locked by the host dns conformance tests
  CHECK: cargo test -p draconic-conformance --test host_dns
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=e0c6697476e2b45c bytes=3080 at=2026-09-04T17:25:40.288Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h09-workspace-timeout]]`

## See also

ROADMAP.md H09, `tests/conformance/tests/host_dns.rs`, `tests/conformance/fixtures/host/net/dns`, `crates/draconic-backend-llvm/src/host_dns.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h09]], [[ticket-155-h09-workspace-timeout]].
