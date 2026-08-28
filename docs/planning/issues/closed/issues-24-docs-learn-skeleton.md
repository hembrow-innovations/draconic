---
id: issues-24
created_at: "2026-08-28"
updated_at: "2026-08-29"
area: planning
domain: language
title: "Learn skeleton pages"
description: "Install, two landings, Dual worlds, modules, native types, host I/O, packages — with honest status tags."
status: closed
issue-type: feature-request
severity: medium
tags:
  - planning
  - issue
  - enhancement
  - docs
  - closed
---

# Learn skeleton pages

## Parent

[[issues-19-language-docs-site]]

## Description / What to build

Learn is walkable. Install; landing from JavaScript; landing from systems; Dual worlds as the join; then modules, native types, host I/O, and packages. Each page has an honest shipped or not-yet tag. Fences only on shipped pages.

## Blocked by

- [[issues-21-docs-learn-reference-nav]]
- [[issues-22-docs-markdown-subset]]

## Acceptance criteria

- [x] Learn nav lists Install, from JavaScript, from systems, Dual worlds, modules, native types, host I/O, packages
- [x] Landings assume JS/TS and systems programmers; they join at Dual worlds
- [x] Every page has a visible status tag; not-yet pages have no fences
- [x] Pipeline still generates the site (fence compile may land in [[issues-23-docs-fence-pipeline]])

## Agent Brief

### Goal

Write the P03 Learn skeleton with two landings, not a beginner programming book.

### Contract

- Vocabulary: Dual worlds, native types, Program, Learn.
- Designed-plus-status-tags. Prose may describe the designed language; samples only if they build.
- Do not duplicate flagship examples as the book. Small fences only.
- Not a full ECMAScript tour.

### Out of scope

Reference stubs ([[issues-25-docs-reference-skeleton]]). Playground. GitHub Pages.

## Comments

> Child of [[issues-19-language-docs-site]]. Parallel with [[issues-25-docs-reference-skeleton]].
>
> **2026-08-29:** Landed. Learn pages under `website/` (Install, from JavaScript, from systems, Dual worlds, modules, native types, host I/O, packages). Generator emits Learn chapter nav on `section: learn` pages. Install carries a small shipped `drac` fence; not-yet pages are prose only. Pipeline test: `website_pipeline_learn_skeleton_is_walkable`. Parent [[issues-19-language-docs-site]] stays open.
