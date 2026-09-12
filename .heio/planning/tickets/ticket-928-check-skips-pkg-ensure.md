---
id: "ticket-928-check-skips-pkg-ensure"
title: "draconic check does not fetch locked packages"
kind: ticket
status: open
ticket_type: bug
tags: [packages, cli]
blocked_by: []
created_at: "2026-09-13T12:00:00Z"
updated_at: "2026-09-13T12:00:00Z"
---

# draconic check does not fetch locked packages

## Signal

Only `build` calls `ensure_locked_for_entry`. `draconic check` goes through `check_path` → `link_entry`, which discovers `draconic.lock` but does not clone. A cold cache after clone-with-lock fails at import resolve even though `build` would auto-fetch. `run` is fine because it uses `build_program`.

## Fit

Unknown until triage. Linker correctly does not fetch. The CLI check surface still typechecks a consumer.

## Notes

- `crates/draconic-cli/src/cmd/cmd_build.rs` (`build_program`) calls `ensure_locked_for_entry`.
- `cmd_check.rs` does not.
- architecture-linker states fetch is not the linker’s job.

## Parent

[[location-220-packages]]

## What to build

`draconic check` on a consumer with a lock either materialises pins like build, or the miss diagnostic tells the author to get, tidy, or build first.

## Blocked by

none
