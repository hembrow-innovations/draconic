---
id: "ticket-920-no-package-init"
title: "No CLI writes a first package manifest"
kind: ticket
status: open
ticket_type: feature-request
tags: [packages, cli]
blocked_by: []
created_at: "2026-09-13T12:00:00Z"
updated_at: "2026-09-13T12:00:00Z"
---

# No CLI writes a first package manifest

## Signal

Creating a Draconic library from an empty directory is outside the toolchain. There is no `init`, `new`, or `mod init`. `draconic get` errors `MissingManifest` unless `draconic.toml` already exists, then only inserts into `[dependencies]`.

## Fit

Unknown until triage. Fits [[location-220-packages]], not an active slice. Does not rewrite that location destination.

## Notes

- `print_usage` in `crates/draconic-cli/src/main.rs` and `docs/api/api-cli.md` list get and mod tidy, not init.
- `cmd_mod.rs` accepts only `tidy`.
- Known manifest keys are `module`, `dependencies`, `urls`, `replace`, `toolchain`. There is no package `version` or `exports` field.
- `examples/pkg-lib/draconic.toml` is a one-line `module = "github.com/draconic-lang/pkg-lib"` that authors must type by hand.
- Distinct from [[ticket-921-packages-authoring-docs]] (public recipe) and from closed [[ticket-17-roadmap-git-package-manager]] (consume path).

## Parent

[[location-220-packages]]

## What to build

An author can start a package root from the CLI so `get` and tidy have a manifest to write into.

## Blocked by

none
