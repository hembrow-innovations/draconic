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
updated_at: "2026-09-12"
---

# Public site — Contract

A promise with a `test:` pointer is locked. One without is asserted. Purpose: [[Public site purpose]]. Coverage map: [[Public site tests]].

## Behaviour

- `public-site.markdown:subset`: Public site pages render the markdown subset of headings, paragraphs, lists, code fences, and links from `website/content/` sources.
  test: website_pipeline_renders_markdown_subset
- `public-site.markdown:heading-permalinks`: A heading below the title on a Learn or Reference page is a permalink to that heading's fragment id.
  test: markdown render heading permalinks
  test: website/src/tests/docs-shell.test.ts
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
- `public-site.fences:copy`: A visitor can copy the text of a rendered code fence on a Learn or Reference page.
  test: website/src/tests/code-fence-copy.test.ts
- `public-site.nav:learn-reference-status`: Learn and Reference pages include Learn and Reference navigation and a visible shipped or not-yet status.
  test: website_pipeline_learn_and_reference_nav_and_status
- `public-site.home:landing`: `/` is a language homepage with pitch and Get-started CTA, not the Learn chapter dump.
  test: website/src/tests/home-hero-and-cta.test.ts
  test: website/src/tests/home-sample.test.ts
  test: website/src/tests/home-features.test.ts
- `public-site.chrome:odm-shell`: Every page, including home, uses skip-to-content plus a sticky side nav plus a main column.
  test: website/src/tests/site-footer-and-skip-link.test.ts
  test: website/src/tests/site-header-primary-nav.test.ts
- `public-site.chrome:favicon`: The origin serves a favicon so the browser tab shows a site icon and `/favicon.ico` is not a 404.
  test: website/src/tests/site-header-primary-nav.test.ts
- `public-site.chrome:primary-nav`: Every page's side nav has wordmark plus Learn, Reference, and GitHub.
  test: website/src/tests/site-header-primary-nav.test.ts
- `public-site.chrome:current-page`: The side-nav item for the open page is visually distinct from sibling links using an existing semantic token, and still uses aria-current="page".
  test: website/src/tests/site-header-primary-nav.test.ts
  test: website/src/tests/learn-hub-nav.test.ts
  test: website/src/tests/reference-hub-pages.test.ts
- `public-site.chrome:docs-sidebar`: Learn and Reference article pages have a section sidebar and a shipped or not-yet badge from frontmatter.
  test: website/src/tests/docs-shell.test.ts
- `public-site.chrome:docs-article-order`: Learn and Reference article columns render a section kicker, then the page heading, then a shipped or not-yet badge, then an on-page outline when the body has section headings, then remaining markdown, then a related-link footer.
  test: website/src/tests/docs-shell.test.ts
- `public-site.chrome:on-page-toc`: A Learn or Reference page with section headings below the title lists those headings as in-article links to the heading fragment ids.
  test: docs shell
  test: page outline
- `public-site.search:titles-headings`: A visitor can find a Learn or Reference page by title or heading text.
  test: website/src/tests/search.test.ts
- `public-site.a11y:keyboard-small`: Primary nav and article reading work with keyboard and at a small viewport.
  test: website/src/tests/mobile-a11y.test.ts
- `public-site.forbid-vault-as-site`: The public site does not publish `docs/` notes.
- `public-site.forbid-playground`: This area does not ship a playground or in-page runner.
