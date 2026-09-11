---
id: "ticket-807-llvm-walker-native-date-stdlib"
title: "LLVM walker drops native Date and stdlib programs"
kind: ticket
status: open
ticket_type: bug
tags: []
blocked_by: []
created_at: "2026-09-11T21:44:22Z"
updated_at: "2026-09-11T21:44:22Z"
---

# LLVM walker drops native Date and stdlib programs

## Signal

`cargo test --workspace --no-fail-fast` is red. Eight native Conformance runs fail at `emit_llvm_ir` with unsupported IR, while ROADMAP marks the rows `done`.

- **es/builtins/date**: `date_runs` (`tests/conformance/tests/builtins.rs:161`). N08.14.06.
- **stdlib/flags**: `parse_long_short_runs_both_targets`, `typed_options_runs_both_targets`, `surface_runs_js_and_native`. L07.
- **stdlib/url**: `parse_basics_runs_both_targets`, `query_roundtrip_runs_both_targets`, `surface_runs_js_and_native`. L08.
- **stdlib/compression/invalid**: `invalid_runs_both_targets` (`stdlib_compression.rs:85`). L04.

`Date.now()` sets `has_date_now` in `crates/draconic-backend-llvm/src/es_expr/walk.rs:930` and steal-routes at `:62` / `:208` into host time. Compression invalid has `try` and steal-routes at `:465` into exceptions. `parseFlags` / `parseUrl` / `gzip` are not in the `es_kind` ident gates. `walk.rs` is 1075 lines and has no `#[test]`. `es_builtins.rs:376` `date_classifies_and_emits` calls `emit_es_builtins` directly and stays green.

Closed [[ticket-802-llvm-host-emit-bodies]] deleted dead `walk_host_*` wrappers. This sitting is leftover dispatch through the crate interface, not those wrappers.

## Fit

this project, later slice

## Notes

- Command: `cargo test --workspace --no-fail-fast` (exit non-zero). Four targets: `-p draconic-conformance --test builtins|stdlib_compression|stdlib_flags|stdlib_url`.
- ROADMAP.md N08.14.06, L04, L07, L08 are `done` with `targets: both` / native observations.
- Fixture metas claim `targets: js,native` (`tests/conformance/fixtures/es/builtins/date.meta`).
- Do not mark E17.02 or E18.44 done.
