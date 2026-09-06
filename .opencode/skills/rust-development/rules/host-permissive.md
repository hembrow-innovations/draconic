---
title: Default host grants are permissive
impact: HIGH
impactDescription: Deno-style deny breaks designed v1
tags: [host, permissions]
---

## Default host grants are permissive

Filesystem and TCP succeed when a Program runs with no explicit grant subset. Opt-in grant/deny is Roadmap R02. CLI `--allow-fs-read` / `--allow-fs-write` / `--allow-net-listen` / `--allow-net-connect` install a subset.

**Incorrect:** failing `readFile` / `listen` with a permission error when the user passed no `--allow-*`.

**Correct:** host I/O succeeds with no flags. Flags narrow the set when present (R02.03 / R02.04).

**Notes.** ADR-0008. Vocabulary: **Default permission policy** in `CONTEXT.md`. Sockets-first host I/O. JS hard-errors until a bridge row. See `host-registry`.
