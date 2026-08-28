---
id: issues-25
created_at: "2026-08-28"
updated_at: "2026-08-28"
area: planning
domain: language
title: "Reference skeleton pages"
description: "Thin Reference pages for CLI, types, Dual-world rules, host I/O, and packages."
status: open
issue-type: feature-request
severity: medium
tags:
  - planning
  - issue
  - enhancement
  - docs
  - ready-for-agent
---

# Reference skeleton pages

## Parent

[[issues-19-language-docs-site]]

## Description / What to build

Reference is walkable while writing a Program. Thin written pages for CLI, types, Dual-world rules, host I/O, and packages. Same status-tag rules as Learn. Not a generated API dump.

## Blocked by

- [[issues-21-docs-learn-reference-nav]]
- [[issues-22-docs-markdown-subset]]

## Acceptance criteria

- [ ] Reference nav lists CLI, types, Dual-world rules, host I/O, packages
- [ ] Pages are written stubs with visible shipped or not-yet; no fences on not-yet
- [ ] Pipeline still generates the site

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
