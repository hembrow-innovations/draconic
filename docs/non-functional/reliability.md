---
id: reliability
title: Reliability
kind: non-functional
description: Fail-closed package resolve, native-only hard-error, workspace oracle, and no silent dual-backend divergence.
status: active
domain: draconic
area: non-functional
tags: [non-functional, reliability]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Reliability

## Overview

Reliability here means fail closed instead of silent wrong code: package version resolve and integrity, native-only / JS-only diagnostics, the workspace test oracle, and Dual backends sharing one IR. Locked shape: [[0002-shared-ir-dual-backends]], [[0009-go-style-git-packages]], [[0012-oracle-check-timeout]], [[CONTEXT]] (Portable program, Native-only / JS-only). Architecture: [[architecture-pkg]], [[architecture-cli]], [[architecture-embed]].

## Requirements

- **Fail-closed package resolve (K04.02)**: semver git tags resolve to a commit OID, or a diagnostic. No match, non-semver-only tags, empty tags, or invalid req must not silently float a pin (`crates/draconic-pkg` `resolve.rs`). Direct deps become lock pins (OID + content hash).
- **Integrity refuse (K08)**: mismatched tree SHA-256 or checkout OID is a hard fail at ensure and import resolve. No silent wrong tree. Symlinks in the hashed tree fail closed. See [[security]].
- **Offline miss is a hard error (K07.02)**: `draconic build --offline` uses cache only; a miss is a fixit, not a network fetch.
- **Native-only hard-error**: a feature valid on exactly one backend must diagnostic on the other. Never emit silent wrong code ([[CONTEXT]], JS `reject_native_only`, host listen APIs on js until a bridge row, `extern "C"` on js). [[0008-host-io-sockets-first-http]] native-first listen; JS hard-errors unsupported host APIs until an explicit bridge.
- **Workspace oracle**: keep `cargo test --workspace` when that is the promise. Do not treat a timeout with green `EXPECT` already in the output as a product hang ([[0012-oracle-check-timeout]], [[performance]]).
- **Dual backends must not silently diverge**: both consume shared IR after Frontend ([[0002-shared-ir-dual-backends]]). Portable programs need equivalent observable behavior. Native-only / JS-only must hard-error, not skip a lower.

## Approach

Packages: resolve tags → pin → cache checkout → verify OID + SHA-256 before link. CLI `get` / `mod tidy` / `build` share that ensure path ([[api-cli]], [[architecture-pkg]]).

Backends: Frontend lowers once. JS emit rejects native pointers, pointer ops, and `extern "C"`. Host policy fixtures under Conformance pin js hard-error versus native success.

Loop: a `both` Roadmap row is not done until both targets are tested or explicitly split ([[guides-loop]], [[0006-mega-loop-roadmap-tests]]).

Catchable exceptions versus abort stay split ([[0011-catchable-exceptions-vs-abort]], [[security]]): language errors are catchable; invariant and abort are process death.

## Risks

- **JS host bridges**: until an explicit bridge row, js must keep hard-erroring listen/server APIs. A polyfill that silently no-ops would be divergence.
- **Lockless or tampered cache**: refuse, do not run the wrong tree. Operators may see hard fails where other ecosystems float.
- **Oracle budget miss**: filing workspace timeout as a product defect produced false tickets ([[0012-oracle-check-timeout]]). Raise the budget or wait; do not drop conformance.
- **Embed subset**: Embed eval supports a limited IR interpreter ([[api-embed]]). Unsupported constructs diagnostic. That is fail closed, not “JS eval on native.”
