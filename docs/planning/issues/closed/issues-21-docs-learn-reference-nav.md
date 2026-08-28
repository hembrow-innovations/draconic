---
id: issues-21
created_at: "2026-08-28"
updated_at: "2026-08-29"
area: planning
domain: language
title: "Docs site: Learn and Reference nav plus status"
description: "Generator wraps pages with Learn and Reference nav; frontmatter drives section and shipped or not-yet."
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

# Docs site: Learn and Reference nav plus status

## Parent

[[issues-19-language-docs-site]]

## Description / What to build

The generated site has two sections, Learn and Reference. Each page declares its section and shipped or not-yet in frontmatter. The HTML shows that status and links both sections.

## Blocked by

- [[issues-20-docs-ssg-one-page]]

## Acceptance criteria

- [x] Frontmatter carries title, section (`learn` | `reference`), and status (`shipped` | `not-yet`)
- [x] Generated HTML includes nav to Learn and to Reference
- [x] The page's status is visible in the HTML
- [x] Pipeline still compiles the generator and asserts this structure

## Agent Brief

### Goal

Turn the one-page generator into a two-section site skeleton with visible status tags.

### Contract

- Vocabulary: **Learn**, **Reference**, shipped, not-yet.
- One site, two sections. No third top-level product.
- Pipeline seam only. Do not add a playground or a Node generator.

### Out of scope

Full markdown subset rendering, fence compile, chapter prose, GitHub Pages.

## Comments

> Child of [[issues-19-language-docs-site]].

> **2026-08-29:** Landed. `website/generate.drac` reads `website/learn.md` and `website/reference.md` (frontmatter: title, section, status) and writes HTML with Learn/Reference nav. Native host_fs cannot parse YAML yet, so the page body including frontmatter is appended; status is visible that way. Pipeline test: `tests/integration/tests/website_pipeline.rs`. Parent [[issues-19-language-docs-site]] stays open.
