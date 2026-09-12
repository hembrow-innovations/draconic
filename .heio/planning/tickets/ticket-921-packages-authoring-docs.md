---
id: "ticket-921-packages-authoring-docs"
title: "Public packages pages skip creating and tagging a library"
kind: ticket
status: open
ticket_type: observation
tags: [packages, website, docs]
blocked_by: []
created_at: "2026-09-13T12:00:00Z"
updated_at: "2026-09-13T12:00:00Z"
---

# Public packages pages skip creating and tagging a library

## Signal

Shipped Learn and Reference packages chapters teach consuming (`draconic get`) more than creating. They never show `git init`, commit, a semver tag, or push so `https://{module_path}.git` works. Copy-the-examples still needs a local git dance that those pages omit. CONTEXT never defines package, library, or module path.

## Fit

Unknown until triage. Fits product / [[location-220-packages]]. Does not rewrite the location destination.

## Notes

- `draconic-web/content/packages.md` and `draconic-web/content/reference-packages.md` jump from one-line `module = "github.com/org/pkg"` to consumer get.
- Resolve in `crates/draconic-pkg/src/resolve.rs` only accepts semver tags.
- `examples/pkg-lib` is not a published remote. Consumer and flagship READMEs require temp git, tag `v0.1.0`, then `get --url`.
- Package-root fences compile as single Programs. Consumer `from "github.com/org/pkg"` fences are not `drac`, so they never compile.
- Direct-deps-only and private-git env (`DRACONIC_GIT_TOKEN`) exist in the vault and are absent from public pages.
- `draconic-web/content/cli.md` omits `--library` and `--offline`. `print_usage` omits `--offline`.
- Distinct from archived [[ticket-831-learn-packages-ship]] (chapter shipped) and from [[ticket-920-no-package-init]] (CLI).

## Parent

[[location-220-packages]]

## What to build

An author following Learn or Reference can create, tag, and consume a library without reading vault architecture notes.

## Blocked by

none
