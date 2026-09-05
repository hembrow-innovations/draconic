---
id: "s-l07-02-workspace-tests-native"
title: "L07.02 workspace tests pass (native runtime C)"
kind: slice
status: failed
sprint: "platform"
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-05T21:15:00Z"
---

# L07.02 workspace tests pass (native runtime C)

## Why

Review of [[s-l07-02-workspace-tests]] left O1 and O2 unmet: native `build_native_binary` failed with `runtime C source missing` at a stale pi-worktree `CARGO_MANIFEST_DIR`. L07.02 stays `done` on ROADMAP.md. This cut beats those oracles without re-opening the language row.

## Done

`cargo test --workspace` and `cargo test -p draconic-conformance --test stdlib_flags` both exit 0 and print `test result: ok.` Native runtime C compile uses tree files when present and embedded sources when the compile-time path is gone.

## Blocked by

None.

## Non-goals

- **Re-opening [[s-l07-02-workspace-tests]] / [[s-l07-02]]**: those slices stay sealed
- **Minting a new L07.02 Loop atom**: ROADMAP L07.02 stays `done`
- **L07 / L07.01** remainder rows
- Restoring deleted pi-worktrees

## Oracle checklist

- [ ] O1: workspace tests pass after the native runtime C fix
  CHECK: cargo test --workspace
  EXPECT: test result: ok.
  EVIDENCE: unmet exit=101 match=yes bytes=132849
  ABANDON: leftover after --reverify; CHECK failed (exit=101 match=yes bytes=132849); cargo test --workspace exited 101 → [[ticket-205-l07-02-workspace-tests]]

- [x] O2: L07.02 typed-options fixtures stay locked by stdlib flags on both targets
  CHECK: cargo test -p draconic-conformance --test stdlib_flags
  EXPECT: test result: ok.
  EVIDENCE: met exit=0 match=yes bytes=2697

## Pool

- `[[l07-02-workspace-tests-native]]`

## See also

ROADMAP.md L07.02, `crates/draconic-runtime/src/draconic_rt.c`, `crates/draconic-runtime/src/lib.rs`, `tests/conformance/tests/stdlib_flags.rs`, [[ticket-190-l07-02-workspace-tests-native]], [[s-l07-02-workspace-tests]].
