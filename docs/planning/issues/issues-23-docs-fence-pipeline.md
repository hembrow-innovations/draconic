---
id: issues-23
created_at: "2026-08-28"
updated_at: "2026-08-28"
area: planning
domain: language
title: "Docs pipeline: shipped fences must build"
description: "Website pipeline extracts shipped drac fences and compiles them; not-yet pages with fences fail."
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

# Docs pipeline: shipped fences must build

## Parent

[[issues-19-language-docs-site]]

## Description / What to build

The website pipeline gives the must-build rule teeth. Shipped pages' Draconic fences are extracted and compiled. A not-yet page that contains a fence fails the pipeline. Fixture pages are enough; full Learn prose is not required.

## Blocked by

- [[issues-21-docs-learn-reference-nav]]
- [[issues-22-docs-markdown-subset]]

## Acceptance criteria

- [ ] Pipeline extracts `drac` fences from shipped pages and `draconic build` succeeds on them
- [ ] A not-yet page with a fence fails the pipeline
- [ ] A not-yet page with no fence still generates

## Agent Brief

### Goal

Fold fence extraction and compile into the one website pipeline seam.

### Contract

- Copy-paste code only on shipped pages. ADR-0010.
- Use fixture pages if Learn content is not in yet.
- Observe build success/failure, not parser internals.
- Do not add a playground.

### Out of scope

Writing the real Learn/Reference chapters. GitHub Pages. Expanding the markdown subset.

## Comments

> Child of [[issues-19-language-docs-site]].
