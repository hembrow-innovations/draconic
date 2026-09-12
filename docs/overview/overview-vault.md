---
id: overview-vault
title: Committed vault
kind: overview
domain: draconic
area: overview
tags: [overview]
created_at: "2026-09-06"
updated_at: "2026-09-06"
description: How committed toolchain knowledge is stored in the docs vault, including kind folders, ADRs, specs, and what is not completeness.
---

# Committed vault

## Overview

`docs/` is the committed Obsidian vault for toolchain knowledge that should survive a clone. Orientation starts here, at [[overview-toolchain]], and at [[overview-completeness]]. Search this vault first. Ignore `docs/99_scribble/`.

[[domain]] names the single-context layout. `AGENTS.md` wins over the default docs-skill tree: glossary is [[CONTEXT]] (`CONTEXT.md` at repo root), locked decisions live in `docs/adr/` (not `docs/decisions/adr/`), and completeness is archived [[ROADMAP]] (`docs/overview/ROADMAP.md`).

## Context

This vault documents the language Toolchain as it is locked: glossary, ADRs, architecture, specs, APIs, standards, guides, and non-functionals. It does not invent product rules. It does not replace [[ROADMAP]] or the Conformance suite.

Day-to-day agent working files under `.heio/` are not language completeness. Do not treat Heio tickets, slices, or rounds as the Loop source of truth.

Public Learn and Reference sources live in `website/`, not this vault ([[0010-public-docs-draconic-ssg]], [[0013-public-site-tanstack-start]], [[guides-public-docs]]). Agent and toolchain notes stay in this vault.

## Design

### Sources of truth

- **Glossary**: [[CONTEXT]] — do not duplicate terms here.
- **Locked decisions**: `docs/adr/` — wikilink by filename stem, for example [[0001-rust-host-compiler]]. Do not invent ADRs in overview notes.
- **Completeness**: archived [[ROADMAP]] with the Conformance suite — see [[overview-completeness]]. Root `ROADMAP.md` is a stub.
- **Layout note**: [[domain]]
- **Vault standard**: [[standards-docs-vault]]
- **Language purpose**: [[purpose]] under `docs/specs/draconic/<area>/`

### Kind folders

Kind is the folder under `docs/`. Domain rides in frontmatter (`domain: draconic`), except specs nest by domain path. `AGENTS.md` overrides ADR placement.

- **overview/**: this hub (`overview-<slug>.md`).
- **architecture/**: [[architecture-pipeline]] style notes and [[system-design-compile-pipeline]] style notes.
- **adr/**: locked ADRs `NNNN-<slug>.md` (this repo; not `docs/decisions/adr/`).
- **specs/draconic/<area>/**: spec folders (purpose, contract, test). Specs are folders, not flat files. Start from [[purpose]].
- **standards/**: [[standards-docs-vault]] and other `standards-<slug>` notes.
- **guides/**: [[guides-toolchain]], [[guides-public-docs]].
- **api/**: [[api-cli]], [[api-embed]].
- **non-functional/**: [[security]], [[performance]], [[reliability]].
- **agents/**: agent orientation such as [[domain]].
- **style/**: style notes when they exist (`style-<slug>.md`).

### Architecture and system-design notes

Sibling notes this swarm will write (wikilink even if they land in parallel):

- **architecture-pipeline**: [[architecture-pipeline]]
- **system-design-compile-pipeline**: [[system-design-compile-pipeline]]
- **architecture-frontend**: [[architecture-frontend]]
- **architecture-lexer**: [[architecture-lexer]]
- **architecture-parser**: [[architecture-parser]]
- **architecture-ast**: [[architecture-ast]]
- **architecture-check**: [[architecture-check]]
- **architecture-diagnostics**: [[architecture-diagnostics]]
- **architecture-ir**: [[architecture-ir]]
- **architecture-backend-js**: [[architecture-backend-js]]
- **architecture-backend-llvm**: [[architecture-backend-llvm]]
- **system-design-dual-backends**: [[system-design-dual-backends]]
- **architecture-runtime**: [[architecture-runtime]]
- **architecture-embed**: [[architecture-embed]]
- **architecture-dual-worlds**: [[architecture-dual-worlds]]
- **system-design-gc-runtime**: [[system-design-gc-runtime]]
- **architecture-cli**: [[architecture-cli]]
- **architecture-lsp**: [[architecture-lsp]]
- **architecture-pkg**: [[architecture-pkg]]
- **architecture-linker**: [[architecture-linker]]
- **architecture-editors**: [[architecture-editors]]

### Linking

Note to note uses `[[wikilinks]]` only. Never relative `.md` paths. Filename stem is the link target (`[[0006-mega-loop-roadmap-tests]]`, [[purpose]], [[domain]]).

### Leftover and out of band

- **docs/99_scribble/**: scratch. Never source of truth. Ignore.
- **docs/reference/guides/**: leftover tree. Not source of truth. Do not move it; do not treat it as [[guides-toolchain]] or [[guides-public-docs]].
- **.heio/**: local working memory (tickets, slices, tasks, rounds). Not language completeness. [[ROADMAP]] is the archived Loop checklist.
- **website/**: public site sources, not this vault ([[0010-public-docs-draconic-ssg]], [[0013-public-site-tanstack-start]]).

## Trade-offs

The docs skill default puts ADRs under `docs/decisions/adr/` and a glossary under `docs/overview/glossary.md`. This checkout does not. [[domain]] and `AGENTS.md` keep a single glossary at [[CONTEXT]] and locked decisions at `docs/adr/` so the language Loop and the vault stay one context.

## Related notes

- **overview-toolchain**: [[overview-toolchain]]
- **overview-completeness**: [[overview-completeness]]
- **CONTEXT**: [[CONTEXT]]
- **ROADMAP**: [[ROADMAP]]
- **domain**: [[domain]]
- **purpose**: [[purpose]]
- **standards-docs-vault**: [[standards-docs-vault]]
- **guides-toolchain**: [[guides-toolchain]]
- **guides-public-docs**: [[guides-public-docs]]
- **api-cli**: [[api-cli]]
- **api-embed**: [[api-embed]]
- **security**: [[security]]
- **performance**: [[performance]]
- **reliability**: [[reliability]]
