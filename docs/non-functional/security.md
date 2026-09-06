---
id: security
title: Security
kind: non-functional
description: Host I/O permission default, optional run grants, package integrity hashes, git secret handling, and catchable versus abort.
status: active
domain: draconic
area: non-functional
tags: [non-functional, security]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Security

## Overview

This note covers host I/O permissions, package integrity, private git credentials, and the catchable-exception versus process-abort split. It describes the Toolchain as designed. It does not claim a Deno-style deny-by-default sandbox. Surfaces: [[architecture-cli]], [[architecture-pkg]], [[architecture-embed]], [[0008-host-io-sockets-first-http]], [[0009-go-style-git-packages]], [[0011-catchable-exceptions-vs-abort]].

## Requirements

- **Default host I/O is permissive (R02.04)**: a Program with no explicit grant subset may read and write the filesystem and listen/connect TCP on targets that already expose those host APIs. Unset or empty `DRACONIC_PERMISSIONS` allows those ops (`draconic_rt_host.c` `host_permissions_allows`).
- **Optional grant subset (R02.01 / R02.03)**: `draconic run` accepts `--allow-fs-read`, `--allow-fs-write`, `--allow-net-listen`, and `--allow-net-connect`. Non-empty grants are forwarded as `DRACONIC_PERMISSIONS` (comma tokens) to the child process. When that env is set, only listed tokens are allowed.
- **Not Deno sandbox**: do not document a locked-down default. Rejected in [[0008-host-io-sockets-first-http]].
- **Package integrity (K08)**: lockfile `content_hash` is SHA-256 of the canonical package tree. Resolve and build recompute the hash and check checkout OID; mismatch hard-fails. No silent wrong tree ([[0009-go-style-git-packages]], `crates/draconic-pkg` hash/ensure/import_resolve).
- **Never persist git secrets (K11.01)**: HTTPS token or SSH identity may authenticate clone/fetch from the environment (`DRACONIC_GIT_TOKEN`, `DRACONIC_GIT_TOKEN_USER`, `DRACONIC_GIT_SSH_KEY`). Credentials are never read from or written to `draconic.toml` or `draconic.lock`. Stored URLs strip userinfo.
- **Catchable versus abort ([[0011-catchable-exceptions-vs-abort]])**: user `throw` and ECMA errors are catchable. `draconic_rt_abort`, Runtime invariant failure, and resource-budget exhaustion are not JS values. Embed oversize source and alloc/time budget exhaustion fail closed with a diagnostic, not a catchable exception ([[architecture-embed]], [[api-embed]]).

## Approach

Host: default allow. Opt-in grants tighten the subset when flags are present; they do not turn an unflagged run into a sandbox. See [[api-cli]] `run`.

Packages: `draconic.lock` pins commit OID plus tree SHA-256. `verify_content_hash` / `verify_package_integrity` refuse tamper, OID mismatch, and symlinks in the hashed tree. Version resolve fails closed on empty or non-matching tags ([[reliability]]).

Auth: `GitAuth` from env or an explicit value at fetch time. `sanitize_stored_git_url` / redaction keep tokens out of manifests, locks, and diagnostics.

Failures: catchable exceptions continue after `catch`. Abort prints `draconic_rt: abort` and dies. Budget miss is fail-closed at the C ABI or Embed diagnostic.

## Risks

- **Permissive default**: a Program can use host fs and TCP without flags on targets that expose those APIs. Treat that as designed, not a missed sandbox.
- **Partial grants**: passing one `--allow-*` sets `DRACONIC_PERMISSIONS` and therefore denies tokens not listed. Unflagged runs stay permissive.
- **Integrity depends on the lock**: floating builds without a lock are outside the K08 pin. `--offline` still refuses a cache miss rather than fetching ([[0009-go-style-git-packages]]).
- **Private git is opt-in later surface (K11.01)**: missing or rejected credentials fail closed; success must still not write secrets.
- **Abort is unrecoverable**: do not wrap `draconic_rt_abort` in `try`/`catch`. See [[0011-catchable-exceptions-vs-abort]].
