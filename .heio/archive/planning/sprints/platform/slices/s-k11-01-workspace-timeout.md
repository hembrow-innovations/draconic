---
id: "s-k11-01-workspace-timeout"
title: "K11.01 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T19:04:38Z"
updated_at: "2026-09-04T19:19:41Z"
claimed-by: bb6f3688-e52b-4e19-b8ce-b01421364c43
---

# K11.01 workspace tests finish

## Why

Review of [[s-k11-01]] left ROADMAP K11.01 unfinished: O1 (`draconic-pkg` k11_01) and O2 (`draconic-cli` k11_01) held, but O3 `cargo test --workspace` timed out at 120s. The packages location still needs the K11.01 Loop to leave the workspace green, not only the private git HTTPS token / SSH crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K11.01 work. The `draconic-pkg` k11_01 private git HTTPS token / SSH auth tests and the `draconic-cli` k11_01 CLI surface tests stay green. If the hang comes from the K11.01 change, fix that private git auth surface so both the workspace check and those crate tests hold. Mark K11.01 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k11-01]]**: that slice stays sealed `failed`
- **K11**: Post-v1 packaging umbrella (not this child)
- **K11.02**: `replace` directive: fork/local override
- **K11.03**: Multi-module monorepo (subdir module paths)
- **K11.04**: Module proxy/mirror (git still canonical)
- **K11.05**: Yank/retract when advisory source configured
- **K03.02**: git clone/fetch into cache (HTTPS; fixture repos in tests) (already `done`; public/anonymous)

## Oracle checklist

- [x] O1: workspace tests finish after the K11.01 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --offline k11_01 && cargo test -p draconic-cli --test k11_01 --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=d983f4fb4c104bc3 bytes=95831 at=2026-09-04T19:19:17.227Z

- [x] O2: K11.01 private git HTTPS token / SSH auth stays locked by the draconic-pkg k11_01 tests
  CHECK: cargo test -p draconic-pkg k11_01
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=edbb6e5f3b570f73 bytes=1953 at=2026-09-04T19:19:17.466Z

- [x] O3: K11.01 private git auth CLI surface stays locked by the draconic-cli k11_01 tests
  CHECK: cargo test -p draconic-cli --test k11_01
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=46a7e9bbc4be2c32 bytes=2976 at=2026-09-04T19:19:18.019Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k11-01-workspace-timeout]]`

## See also

ROADMAP.md K11.01, `crates/draconic-pkg`, `crates/draconic-cli`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k11-01]], [[ticket-173-k11-01-workspace-timeout]].
