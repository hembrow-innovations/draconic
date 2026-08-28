---
id: issues-20
created_at: "2026-08-28"
updated_at: "2026-08-29"
area: planning
domain: language
title: "Docs SSG: one page to HTML"
description: "A native Draconic Program reads one markdown page and emits one HTML file."
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

# Docs SSG: one page to HTML

## Parent

[[issues-19-language-docs-site]]

## Description / What to build

A visitor can generate a single HTML page from a single markdown source by running a native Draconic Program. Title from the page appears in the HTML. This is the first dogfood slice of the public site generator.

## Blocked by

- None — can start immediately

## Acceptance criteria

- [x] A native Program reads one markdown page and writes one HTML file
- [x] The website pipeline compiles that generator and runs it; the HTML contains the page title
- [x] No Node docs generator, no vault pages, no playground

## Agent Brief

### Goal

Land the smallest generator: one source page in, one HTML file out, proven by the website pipeline seam.

### Contract

- Generator is a Draconic Program, native target, host filesystem.
- Public tree is not the agent vault. ADR-0010.
- Test at pipeline height: compile generator, run, observe HTML. Same class as http-echo / todo integration.
- Markdown subset and two-section nav are later tickets.

### Out of scope

Nav, status tags, fence extraction, GitHub Pages, Learn/Reference content, playground.

## Comments

> Child of [[issues-19-language-docs-site]].

> **2026-08-29:** Landed. `website/generate.drac` reads `website/page.md` and writes `website/page.html`. Pipeline test: `tests/integration/tests/website_pipeline.rs`. Parent [[issues-19-language-docs-site]] stays open.
