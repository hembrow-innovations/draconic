---
id: "purpose"
title: "Host I/O purpose"
kind: purpose
description: "Product brief: job, scope, and fences for process, stdio, fs, sockets, and thin HTTP."
status: active
domain: draconic
area: host-io
tags: [purpose]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Host I/O purpose

## Job

Let a Program talk to the process, filesystem, and network on the targets that already expose those host APIs, with sockets first and thin HTTP helpers on those sockets.

## In scope

- **Process, stdio, path, filesystem**: args, env, exit, stdout/stderr/stdin, path string ops, read/write/dirs.
- **Sockets then HTTP**: native TCP listen/accept/connect/read/write, then plaintext HTTP/1.1 request/response helpers on those sockets.
- **Target policy**: native first for listen/server paths; JS hard-errors unsupported host APIs or uses an explicit bridge where that row exists.
- **Default permission policy**: permissive. A Program with no explicit grant subset may use fs and TCP already exposed.
- **Opt-in grants**: when `--allow-fs-read` / `--allow-fs-write` / `--allow-net-listen` / `--allow-net-connect` are present on `draconic run`, that subset is the grant; missing grants deny with a diagnostic.

## Out of scope

- **Full browser**: no browser engine, DOM, or page runtime as a host surface.
- **Deno-shaped permission UX as the v1 default**: locked-down deny-by-default is not the designed default ([[0008-host-io-sockets-first-http]], [[CONTEXT]] Default permission policy).
- **Language semantics**: ECMA-262 and Dual worlds are not this folder.
- **Package identity and fetch**: git modules live under [[Packages purpose]].
- **CLI parse/check/fmt as product**: those live under [[Toolchain purpose]]; this folder only owns run allow-flags that install grants.

## Surfaces

- **Host identifiers in a Program**: process, stdio, fs, path, TCP, HTTP helpers.
- **`draconic run` allow flags**: grant subset forwarded as `DRACONIC_PERMISSIONS`.
- **Success Program**: `examples/http-echo` as pure Draconic native HTTP/1.1.

## Authority

- Behaviour: [[Host I/O — Contract]]
- Tests: [[Host I/O tests]]
- Glossary: [[CONTEXT]]
- Decisions: [[0008-host-io-sockets-first-http]]
- Completeness track: [[ROADMAP]] (H)

## Open product questions

- (none)
