---
id: "ticket-190-l07-02-workspace-tests-native"
title: "L07.02 workspace tests did not pass (O1 O2 native runtime C)"
kind: ticket
status: ready-for-agent
labels: bug
tags: []
sprint: platform
created_at: "2026-09-05T12:44:16Z"
updated_at: "2026-09-05T12:44:16Z"
caused-by: s-l07-02-workspace-tests
failed: true
intent: fix
---

# L07.02 workspace tests did not pass (O1 O2 native runtime C)

Reviewer miss on [[s-l07-02-workspace-tests]]. Not a new ROADMAP atom. L07.02 stays `done` on ROADMAP.md. Next Builder must beat the failed oracles, not re-open the language row.

- **caused-by**: s-l07-02-workspace-tests
- **failed oracle**: O1 and O2
- **O1 CHECK**: cargo test --workspace
- **O1 EXPECT**: test result: ok.
- **O1 EVIDENCE**: unmet exit=101 match=yes bytes=77974 at=2026-09-05T12:38:10.339Z
- **O2 CHECK**: cargo test -p draconic-conformance --test stdlib_flags
- **O2 EXPECT**: test result: ok.
- **O2 EVIDENCE**: unmet exit=101 match=no bytes=4281 at=2026-09-05T12:38:11.464Z
- **gap**: native `build_native_binary` fails with `build runtime static lib failed: runtime C source missing` at stale path `/private/var/folders/8n/_by5jpf16x34d49_qwgf_9wr0000gn/T/pi-worktree-b944d90e-ee30-408c-a94c-851a1a71b3b7-0/crates/draconic-runtime/src/draconic_rt.c`. File is present at `crates/draconic-runtime/src/draconic_rt.c` in this checkout. Same miss hits `stdlib_flags` native runs (`typed_options_runs_both_targets`, `parse_long_short_runs_both_targets`, `surface_runs_js_and_native`) and workspace native suites (`cross_compile`, `ffi_link_dynamic`, `ffi_link_static`, `http_echo`, `native_debug`).
- **beat**: `cargo test --workspace` and `cargo test -p draconic-conformance --test stdlib_flags` both exit 0 and print `test result: ok.`
- **Roadmap ID**: L07.02 (already `done`; do not mint a new language atom)
- **Item**: Typed options (bool/string/number); help text as designed
- **Tests**: `tests/conformance` fixtures `stdlib/flags`
- **Targets**: both
