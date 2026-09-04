---
id: "s-k09-workspace-timeout"
title: "K09 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T19:00:46Z"
updated_at: "2026-09-04T19:17:20Z"
claimed-by: e84017cf-8e27-4796-be8c-46d92a0268be
---

# K09 workspace tests finish

## Why

Review of [[s-k09]] left ROADMAP K09 unfinished: O1 (`k09_01_resolve_fetch`) and O2 (`k09_02_build_consumer`) held, but O3 `cargo test --workspace` timed out at 120s. The packages location still needs the K09 Loop to leave the workspace green, not only the temp git dep + consumer Program package tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K09 work. The `draconic-packages-tests` k09_01 resolve+fetch and k09_02 consumer build/import tests stay green. If the hang comes from the K09 change, fix that temp git dep + consumer Program E2E surface so both the workspace check and those package tests hold. Mark K09 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k09]]**: that slice stays sealed `failed`
- **K09.01**: Fixture temp git lib (tagged); consumer manifest+lock; resolve+fetch (already `done`)
- **K09.02**: E2E build consumer importing module path from fixture (already `done`)
- **K01–K08**: Manifest, lock, cache, resolve, CLI, build integration, integrity
- **K10**: Demo package published for copy-paste
- **K11**: Post-v1 packaging (auth, replace, monorepo, proxy, yank)

## Oracle checklist

- [x] O1: workspace tests finish after the K09 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-packages-tests --test k09_01_resolve_fetch --offline && cargo test -p draconic-packages-tests --test k09_02_build_consumer --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=64996a284c300cb9 bytes=95076 at=2026-09-04T19:16:51.374Z

- [x] O2: K09.01 temp git lib resolve+fetch stays locked by the packages k09_01 test
  CHECK: cargo test -p draconic-packages-tests --test k09_01_resolve_fetch
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=2724f2399b71caf8 bytes=2085 at=2026-09-04T19:16:51.678Z

- [x] O3: K09.02 consumer Program import/build stays locked by the packages k09_02 test
  CHECK: cargo test -p draconic-packages-tests --test k09_02_build_consumer
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=12ae3974b38b24bd bytes=2089 at=2026-09-04T19:16:51.965Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k09-workspace-timeout]]`

## See also

ROADMAP.md K09, `tests/packages/tests/k09_01_resolve_fetch.rs`, `tests/packages/tests/k09_02_build_consumer.rs`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k09]], [[ticket-172-k09-workspace-timeout]].
