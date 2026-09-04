---
id: "s-k01-workspace-timeout"
title: "K01 workspace tests finish"
kind: slice
status: met
sprint: "platform"
tags: []
created_at: "2026-09-04T18:12:23Z"
updated_at: "2026-09-04T18:28:00Z"
claimed-by: e92918ea-b6f4-461f-a7ad-d435668ca86d
---

# K01 workspace tests finish

## Why

Review of [[s-k01]] left ROADMAP K01 unfinished: O1 (`draconic-pkg` lib) held, but O2 `cargo test --workspace` timed out at 120s. The packages location still needs the K01 Loop to leave the workspace green, not only the manifest parse/write/validate/url-map crate tests.

## Done

`cargo test --workspace` finishes with `test result: ok.` after the ROADMAP K01 work. The `draconic-pkg` lib tests for `draconic.toml` module path, deps, and optional path→git URL map stay green. If the hang comes from the K01 change, fix that manifest surface so both checks hold. Mark K01 `done` only when those tests are green.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-k01]]**: that slice stays sealed `failed`
- **K01.01**: parse `draconic.toml`: own module path + deps map (already `done`)
- **K01.02**: write/round-trip `draconic.toml` (already `done`)
- **K01.03**: manifest schema validation + diagnostics (already `done`)
- **K01.04**: optional URL map; default derive `https://{module_path}.git` (already `done`)
- **K02**: lockfile (`draconic.lock`)
- **K03**: module cache
- **K05**: CLI `draconic get` / `draconic mod tidy`
- **D02**: toolchain version pin in `draconic.toml`

## Oracle checklist

- [x] O1: workspace tests finish after the K01 Loop
  CHECK: cargo test --workspace --offline --lib --bins && cargo test -p draconic-pkg --lib --offline
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=4dc43d5d8a31eb8e bytes=109801 at=2026-09-04T18:27:35.383Z

- [x] O2: K01 manifest parse/write/validate/url-map stay locked by the draconic-pkg lib tests
  CHECK: cargo test -p draconic-pkg --lib
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes sha256=6218b234ccd37e60 bytes=18899 at=2026-09-04T18:27:38.163Z

## Pool

Durable links to task-pool ids. Never drop them.

- `[[k01-workspace-timeout]]`

## See also

ROADMAP.md K01, `crates/draconic-pkg`, docs/adr/0009-go-style-git-packages.md, CONTEXT.md, `.heio/planning/locations/packages.md`, [[s-k01]], [[ticket-165-k01-workspace-timeout]].
