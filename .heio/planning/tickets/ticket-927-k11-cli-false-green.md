---
id: "ticket-927-k11-cli-false-green"
title: "K11 marked done while several knobs never hit get or tidy"
kind: ticket
status: open
ticket_type: observation
tags: [packages, roadmap]
blocked_by: []
created_at: "2026-09-13T12:00:00Z"
updated_at: "2026-09-13T12:00:00Z"
---

# K11 marked done while several knobs never hit get or tidy

## Signal

`docs/overview/ROADMAP.md` marks K11 and K11.01–K11.05 `done`. architecture-pkg and `later.rs` say those knobs are not the v1 bar. Authors reading ROADMAP would think private git, replace, monorepo subdir, proxy, and yank are usable from the CLI.

## Fit

Unknown until triage. Completeness honesty, not a location rewrite. K11.01 auth is actually wired (`GitAuth::from_env` on get). Do not reopen archived K11 timeout tickets.

## Notes

- K11.04: `clone_or_fetch_with_proxy` exists. `get_package` uses `clone_or_fetch_with_auth`. Tidy and ensure use `clone_or_fetch`. `DRACONIC_PROXY` does nothing on the CLI. Architecture already admits this.
- `k11_v1_surface_does_not_silently_ship_later_features` only classifies empty env. `later.rs` is not called from get, tidy, or ensure.
- K11.03: default URL for `github.com/org/mono/pkg/foo` is still `https://github.com/org/mono/pkg/foo.git`. Local and `file://` URLs cannot derive a subdir. No CLI or `tests/packages` coverage.
- K11.05: `get_package` refuses yanked versions when `DRACONIC_ADVISORY` is set. `mod tidy` has no advisory check. There is no author retract command.
- K11.02 replace git/module works; relative path is [[ticket-926-replace-relative-clone]].

## Parent

[[location-220-packages]]

## What to build

ROADMAP K11 rows match what `draconic get` and `draconic mod tidy` actually honor, and remaining unwired knobs are not `done`.

## Blocked by

none
