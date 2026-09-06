---
id: "contract"
title: "Public site — Contract"
kind: contract
description: "Durable promises for public Learn and Reference IA, fences, chrome, search, and vault and playground forbids."
status: active
domain: draconic
area: public-site
tags: [contract]
created_at: "2026-09-06"
updated_at: "2026-09-07"
---

# Public site — Contract

A promise with a `test:` pointer is locked. One without is asserted. Purpose: [[Public site purpose]]. Coverage map: [[Public site tests]].

## Behaviour

- `public-site.markdown:subset`: Public site pages render the markdown subset of headings, paragraphs, lists, code fences, and links from `website/` sources.
  test: website_pipeline_renders_markdown_subset
- `public-site.ia:learn-walkable`: Learn is walkable as Install, from JavaScript, from systems, Dual worlds, modules, native types, host I/O, and packages, and the two landings join at Dual worlds.
  test: website_pipeline_learn_skeleton_is_walkable
  test: website/src/tests/learn-hub-nav.test.ts
  test: website/src/tests/learn-pages.test.ts
  test: website/src/tests/learn-prev-next.test.ts
- `public-site.ia:reference-walkable`: Reference is walkable as CLI, types, Dual-world rules, host I/O, and packages.
  test: website_pipeline_reference_skeleton_is_walkable
  test: website/src/tests/reference-hub-pages.test.ts
- `public-site.fences:shipped-must-build`: A shipped page's copy-paste Draconic fence builds.
  test: website_pipeline_shipped_drac_fence_builds
- `public-site.fences:forbid-not-yet-fences`: A not-yet page that contains a code fence fails the site pipeline.
  test: website_pipeline_not_yet_page_with_fence_fails
- `public-site.nav:learn-reference-status`: Learn and Reference pages include Learn and Reference navigation and a visible shipped or not-yet status.
  test: website_pipeline_learn_and_reference_nav_and_status
- `public-site.home:landing`: `/` is a language homepage with pitch and Get-started CTA, not the Learn chapter dump.
- `public-site.chrome:primary-nav`: Every page has wordmark plus Learn, Reference, and GitHub.
- `public-site.chrome:docs-sidebar`: Learn and Reference article pages have a section sidebar and a shipped or not-yet badge from frontmatter.
  test: website/src/tests/docs-shell.test.ts
- `public-site.search:titles-headings`: A visitor can find a Learn or Reference page by title or heading text.
  test: website/src/tests/search.test.ts
- `public-site.a11y:keyboard-small`: Primary nav and article reading work with keyboard and at a small viewport.
  test: website/src/tests/mobile-a11y.test.ts
- `public-site.forbid-vault-as-site`: The public site does not publish `docs/` notes.
- `public-site.forbid-playground`: This area does not ship a playground or in-page runner.
