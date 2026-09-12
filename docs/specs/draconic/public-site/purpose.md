---
id: "purpose"
title: "Public site purpose"
kind: purpose
description: "Product brief: job, scope, and fences for the public language homepage, Learn path, and Reference."
status: active
domain: draconic
area: public-site
tags: [purpose]
created_at: "2026-09-06"
updated_at: "2026-09-12"
---

# Public site purpose

## Job

Give someone writing a Program a public language homepage, a Learn path, and working Reference at modern language-site quality, without using the agent vault as the site.

## In scope

- **Home landing**: A language homepage, not a Learn chapter dump as the index.
- **Learn path**: Install, from JavaScript, from systems, Dual worlds, modules, native types, host I/O, packages.
- **Reference working pages**: CLI, types, Dual-world rules, host I/O, packages.
- **Shipped or not-yet badges**: Each teaching page shows its status.
- **In-site search**: A visitor finds a Learn or Reference page by title or heading.
- **Site chrome**: Two-column shell on every page: skip link, sticky side nav, main column. Side nav has wordmark, Learn, Reference, and GitHub. Search stays in that nav. The origin serves a favicon so the browser tab shows a site icon. Each page's document title names the open page so tabs distinguish destinations. Home may remain Draconic. An unknown URL keeps that shell; main names the miss and offers a way back to a real page.
- **Docs article**: Learn and Reference pages share that shell. The article column has a section kicker, heading, status badge, an on-page outline of section headings when those headings exist, remaining markdown, and a related-link footer. Side nav groups list the Learn path and Reference pages.
- **Keyboard and small-viewport use**: Primary nav and article reading remain usable. At a small viewport the side nav stacks and wraps; no separate marketing top bar.
- **Copyable fences**: A visitor can copy the text of a rendered code fence on a Learn or Reference page.
- **Heading permalinks**: A section heading on a Learn or Reference page links to its fragment id.
- **On-page outline**: A Learn or Reference page with section headings below the title lists those headings as in-article links to the heading fragment ids.

## Out of scope

- **Playground and in-page runners**: Not this area.
- **Publishing `docs/` as the site**: The agent vault is not the public site.
- **Language semantics, CLI, or ROADMAP completeness**: This area does not change those.
- **mdBook, Starlight, VitePress, Next.js**: Not the public site product.
- **Fences on not-yet pages**: Not-yet pages stay without copy-paste samples.
- **Copy-paste samples that do not compile on shipped pages**: Shipped fences must build.

## Surfaces

- **Public GitHub Pages site**: Where a visitor reads the language homepage, Learn, and Reference.
- **Markdown sources in `website/content/`**: Teaching source with `title`, `section`, and `status`.

## Authority

- Behaviour: [[Public site — Contract]]
- Tests: [[Public site tests]]
- Decisions: [[0013-public-site-tanstack-start]], [[0010-public-docs-draconic-ssg]]
- Glossary: [[CONTEXT]] (Learn and Reference terms)

## Open product questions

- (none)
