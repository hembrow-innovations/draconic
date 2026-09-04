---
id: "s-h16-workspace-timeout"
title: "H16 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T17:50:01Z"
updated_at: "2026-09-04T18:13:14Z"
claimed-by: 96126a57-6ff1-4d4e-8077-92803417844e
---

# H16 workspace tests finish

## Why

Review of [[s-h16]] left ROADMAP H16 unfinished: O1 (`host_os`) held, but O2 `cargo test --workspace` timed out at 120s. The host-io location still needs the H16 Loop to leave the workspace green, not only the host os conformance harness.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP H16 work. The host os conformance harness stays green. If the hang comes from the H16 change, fix that cwd get + chdir, hostname / OS type / arch, temp/home dir, and native sleep / yield surface so both checks hold. Mark H16 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-h16]]**: that slice stays sealed `failed`
- **H16.01**: cwd get + chdir (already `done`)
- **H16.02**: hostname / OS type / arch strings (already `done`)
- **H16.03**: temp dir + home dir paths (already `done`)
- **H16.04**: OS sleep / yield for timer tests (already `done`)
- **H01**: process args, env, exit
- **H03**: path helpers (string ops; no I/O)
- **H05**: time, clock, timers
- **H00**: host I/O surface policy
- js OS-misc APIs or a Node polyfill beyond the existing both-targets fixtures (native hang is the miss)

## Oracle checklist

- [x] O1: workspace tests finish after the H16 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-conformance --test host_os --offline && cargo test -p draconic-runtime --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=02d525e0c8331adc bytes=102364 at=2026-09-04T18:12:55.708Z

- [x] O2: H16 cwd, hostname/os/arch, and temp/home dir stay locked by the host os conformance tests
  CHECK: cargo test -p draconic-conformance --test host_os
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=547400617af825cf bytes=3101 at=2026-09-04T18:12:57.303Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[h16-workspace-timeout]]`

## See also

ROADMAP.md H16, `tests/conformance/tests/host_os.rs`, `tests/conformance/fixtures/host/os`, `crates/draconic-backend-llvm/src/host_os.rs`, `crates/draconic-runtime`, docs/adr/0008-host-io-sockets-first-http.md, CONTEXT.md, `.heio/planning/locations/host-io.md`, [[s-h16]], [[ticket-162-h16-workspace-timeout]].
