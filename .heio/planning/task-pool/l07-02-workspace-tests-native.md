---
id: "l07-02-workspace-tests-native"
title: "L07.02 workspace tests pass (native runtime C)"
kind: task
status: completed
mode: afk
blocked-by: []
tags: []
created_at: "2026-09-05T20:10:00Z"
updated_at: "2026-09-05T20:29:12Z"
---

# L07.02 workspace tests pass (native runtime C)

## Blocked by

None.

## Done

`cargo test --workspace` and `cargo test -p draconic-conformance --test stdlib_flags` both print `test result: ok.` Native `build_runtime_static_lib` succeeds when compile-time `CARGO_MANIFEST_DIR` C sources are missing by writing embedded sources.

## Context

Reviewer miss on [[s-l07-02-workspace-tests]]. `build_native_binary` failed with `build runtime static lib failed: runtime C source missing` at a stale pi-worktree path. File is present at `crates/draconic-runtime/src/draconic_rt.c` in this checkout. Same miss hits `stdlib_flags` native runs and workspace native suites. Current: `c_runtime_path()` is `env!("CARGO_MANIFEST_DIR")` and `build_runtime_static_lib_with_lto` errors if that path is gone. Desired: prefer disk files; if missing, materialize `include_str!` sources and headers into `out_dir` and clang those. Out of scope: re-opening L07.02 as a language atom; restoring worktrees.

## Verify

`cargo test --workspace` exit 0 with `test result: ok.` `cargo test -p draconic-conformance --test stdlib_flags` exit 0 with `test result: ok.` Unit coverage that missing disk C paths still build the static lib.

scope: `crates/draconic-runtime/src/lib.rs`, `crates/draconic-runtime/src/draconic_rt.c`, `crates/draconic-runtime/src/draconic_rt_host.c`, headers as needed for clang `-I`

## Links

[[s-l07-02-workspace-tests-native]] [[ticket-190-l07-02-workspace-tests-native]]

## Gauntlet

- **round 1**: `cargo test -p draconic-runtime --lib resolve_runtime_c_file`, `cargo test -p draconic-runtime --lib from_missing_tree_paths`, `cargo test -p draconic-conformance --test stdlib_flags`, `cargo test --workspace` — win. Missing-tree C paths materialize embedded sources and build `libdraconic_rt.a`. stdlib_flags 6 passed. Workspace 161 `test result: ok.` lines, exit 0. ROADMAP L07.02 remains `done`.
