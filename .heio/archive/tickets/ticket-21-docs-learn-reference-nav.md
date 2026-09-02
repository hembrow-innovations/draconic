---
id: "ticket-21-docs-learn-reference-nav"
title: "Docs site: Learn and Reference nav plus status"
kind: ticket
status: closed
tags: []
created_at: "2026-08-28T00:00:00Z"
updated_at: "2026-08-29T00:00:00Z"
---

# Docs site: Learn and Reference nav plus status

Archived from `docs/planning/issues/closed/issues-21-docs-learn-reference-nav.md`.

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
