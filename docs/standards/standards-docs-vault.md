---
id: standards-docs-vault
title: Docs vault layout
kind: standard
description: Where committed toolchain knowledge lives, which files are source of truth, and what must not be filed in docs/.
domain: draconic
area: standards
tags: [standard, vault]
created_at: "2026-09-06"
updated_at: "2026-09-06"
---

# Docs vault layout

## Statement

`docs/` is the committed Obsidian vault for toolchain knowledge that should survive a clone. [[AGENTS]] plus the docs skill own layout. This checkout is a single-context language repo: glossary is [[CONTEXT]], locked decisions are `docs/adr/`, and completeness is archived [[ROADMAP]]. Specs are folders. Ignore `docs/99_scribble/`. Do not file tickets, slices, tasks, or rounds in `docs/`.

## Rationale

The docs skill default tree puts a glossary under `docs/overview/` and ADRs under `docs/decisions/adr/`. [[AGENTS]] and [[domain]] override that so the Loop, the glossary, and locked decisions stay one context. Mixing day-to-day tracker notes into the vault would duplicate `.heio/` and would compete with [[ROADMAP]] as completeness. Scratch in `99_scribble/` must never be treated as truth.

## Rules

- **Search first**: look under `docs/` before writing a new note. Skip `docs/99_scribble/`.
- **Kind folder**: the folder under `docs/` is the kind (`standards/`, `guides/`, `api/`, `non-functional/`, `architecture/`, `overview/`, `specs/`). Domain rides in frontmatter as `domain: draconic`. Do not invent `docs/draconic/` except under specs.
- **Glossary**: [[CONTEXT]] at the repo root (`CONTEXT.md`). Do not start a second glossary in the vault.
- **Locked decisions**: `docs/adr/` as `NNNN-<slug>.md`. Wikilink by stem, for example [[0006-mega-loop-roadmap-tests]]. Not `docs/decisions/adr/`.
- **Completeness**: archived [[ROADMAP]] (`docs/overview/ROADMAP.md`) with the Conformance suite. Not Heio occupancy. Root `ROADMAP.md` is a stub.
- **Specs**: folders under `docs/specs/<domain>/<area>/` (optional `<feature>/`) holding purpose, contract, and test notes. Specs are not flat files.
- **Ignore scribble**: `docs/99_scribble/` is scratch. Never a source of truth. Never cite it.
- **No tickets in docs/**: tickets, slices, tasks, rounds, and working reports live under `.heio/` (management skill). Do not copy them into `docs/` as a plan. Promote a finished outcome as an ADR, spec, architecture note, standard, or guide, then close the working file.
- **Public site is not the vault**: Learn and Reference sources live in `website/` ([[0010-public-docs-draconic-ssg]], [[0013-public-site-tanstack-start]], [[guides-public-docs]]). Agent and toolchain notes stay here.
- **Leftover tree**: `docs/reference/guides/` is leftover (for example [[issue-tracker]] and [[triage-labels]]). Do not move it. Do not treat it as this standard or as [[guides-toolchain]].
- **Links**: `[[wikilinks]]` only. See [[standards-markdown]].
- **Markdown**: never tables. See [[standards-markdown]] and [[AGENTS]].

Hub orientation: [[overview-vault]], [[overview-toolchain]], [[overview-completeness]].

## Examples

Correct:

- **New standard**: `docs/standards/standards-markdown.md` with `kind: standard` and `area: standards`.
- **New guide**: `docs/guides/guides-loop.md` describing one Roadmap atom ([[guides-loop]]).
- **ADR cite**: `[[0009-go-style-git-packages]]` for git-backed packages.
- **Working ticket**: file under `.heio/`, not under `docs/`.

Incorrect:

- **Second glossary**: `docs/overview/glossary.md` as a living duplicate of [[CONTEXT]].
- **ADR in the skill default path**: `docs/decisions/adr/` on this checkout.
- **Flat spec**: a single `docs/specs/foo.md` instead of a spec folder.
- **Ticket in the vault**: a slice or round copied into `docs/planning/` or `docs/reference/guides/`.
- **Citing scribble**: treating `docs/99_scribble/` as locked policy.

## Exceptions

- **Leftover `docs/reference/guides/`**: those files exist. They are not the layout. Do not edit or move them in this slice. Wikilink only to call them leftover.
- **Repo-root glossary**: [[CONTEXT]] sits next to the vault, not inside a kind folder. That is the [[AGENTS]] override, not a second vault.
- **Archived Roadmap**: [[ROADMAP]] lives at `docs/overview/ROADMAP.md`. Historical tables stay. Root `ROADMAP.md` is a stub.
- **`docs/agents/`**: agent orientation such as [[domain]] is allowed. It is not a place for tickets.
