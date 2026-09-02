---
id: "ticket-25-docs-reference-skeleton"
title: "Reference skeleton pages"
kind: ticket
status: closed
tags: []
created_at: "2026-08-28T00:00:00Z"
updated_at: "2026-08-29T00:00:00Z"
---

# Reference skeleton pages

Archived from `docs/planning/issues/closed/issues-25-docs-reference-skeleton.md`.

# Reference skeleton pages

## Parent

[[issues-19-language-docs-site]]

## Description / What to build

Reference is walkable while writing a Program. Thin written pages for CLI, types, Dual-world rules, host I/O, and packages. Same status-tag rules as Learn. Not a generated API dump.

## Blocked by

- [[issues-21-docs-learn-reference-nav]]
- [[issues-22-docs-markdown-subset]]

## Acceptance criteria

- [x] Reference nav lists CLI, types, Dual-world rules, host I/O, packages
- [x] Pages are written stubs with visible shipped or not-yet; no fences on not-yet
- [x] Pipeline still generates the site

## Agent Brief

### Goal

Stub the working Reference so both doors exist on day one.

### Contract

- Vocabulary: **Reference**, Dual worlds, Program.
- Written pages, not symbol extraction from the compiler.
- Same sample rule as Learn.

### Out of scope

Learn chapters. Generated API. Playground. GitHub Pages.

## Comments

> Child of [[issues-19-language-docs-site]]. Parallel with [[issues-24-docs-learn-skeleton]].
>
> **2026-08-29:** Landed. Reference pages under `website/` (CLI, types, Dual-world rules, host I/O, packages). Generator emits Reference page nav on `section: reference` pages. CLI is shipped; other stubs are not-yet prose with no fences. Pipeline test: `website_pipeline_reference_skeleton_is_walkable`. Parent [[issues-19-language-docs-site]] stays open.
