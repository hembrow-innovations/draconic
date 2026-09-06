---
title: Permissive default permissions
impact: MEDIUM
impactDescription: Deno deny-by-default contradicts R02.04
tags: [host, permissions]
---

## Permissive default permissions

Host filesystem and TCP succeed when a Program runs with no explicit grant subset. `--allow-fs-read` / `--allow-fs-write` / `--allow-net-listen` / `--allow-net-connect` install an opt-in subset as `DRACONIC_PERMISSIONS`.

**Incorrect:** denying fs/net when no flags are passed.

**Correct:** no flags → permissive. Flags → subset. Deny diagnostics when a grant is missing are R02.02 (not the v1 default).

**Notes.** ADR-0008 / R02.04. Not Deno locked-down. Fixture meta may set `grants`. See `host-sockets-first`.
