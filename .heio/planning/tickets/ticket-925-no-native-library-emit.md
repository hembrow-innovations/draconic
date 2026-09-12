---
id: "ticket-925-no-native-library-emit"
title: "No author path to emit a native library from Draconic"
kind: ticket
status: open
ticket_type: feature-request
tags: [packages, native, distribution]
blocked_by: []
created_at: "2026-09-13T12:00:00Z"
updated_at: "2026-09-13T12:00:00Z"
---

# No author path to emit a native library from Draconic

## Signal

`--library` with `--target native` is a parse error. Native build always produces an executable. `--link` consumes extra `.a` / dylibs; it does not emit a `.a`, `.so`, or rlib-like from Draconic exports. An author cannot publish a native library artifact for another Draconic or C program to link.

## Fit

Unknown until triage. Documented flag reject plus missing product. Fits [[location-224-distribution]]. Does not rewrite that destination.

## Notes

- `crates/draconic-cli/src/cmd/cmd_build.rs` and `docs/api/api-cli.md` reject native `--library`.
- Native emit calls `build_native_binary_with_lto` in `crates/draconic-backend-llvm/src/native_link.rs`.
- IR says LLVM ignores `named_exports`.
- F04–F05 / `crates/draconic-cli/tests/build_link.rs` are consume-side extra archives.

## Parent

[[location-224-distribution]]

## What to build

A Draconic library entry can be built to a reusable native artifact, or native `--library` stays rejected and that limit is on the public CLI/packages pages.

## Blocked by

none
