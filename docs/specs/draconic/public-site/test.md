---
id: "test"
title: "Public site tests"
kind: test
description: "Which tests cover the public site spec folder, how, and why."
status: active
domain: draconic
area: public-site
tags: [test]
created_at: "2026-09-06"
updated_at: "2026-09-07"
---

# Public site tests

Purpose: [[Public site purpose]]. Contract: [[Public site — Contract]].

## Coverage

Existing cases in `tests/integration/tests/website_pipeline.rs` lock `public-site.markdown:subset`, `public-site.ia:learn-walkable`, `public-site.ia:reference-walkable`, `public-site.fences:shipped-must-build`, `public-site.fences:forbid-not-yet-fences`, and `public-site.nav:learn-reference-status`. Related fence cases in the same file support those locks. `website/src/tests/learn-hub-nav.test.ts` also locks `public-site.ia:learn-walkable` for the Start Learn hub and aside sequence. `website/src/tests/learn-pages.test.ts` locks the same promise for one Start route per Learn chapter, teaching copy, frontmatter Badge status, and `.html` hrefs rewritten to app routes. `website/src/tests/learn-prev-next.test.ts` locks the same promise for article-footer prev/next that follows `learn.md`, with both landings continuing to Dual worlds. `website/src/tests/reference-hub-pages.test.ts` locks `public-site.ia:reference-walkable` for the Start Reference hub, aside sequence, and working-page routes from `website/*.md`. `website/src/tests/mobile-a11y.test.ts` locks `public-site.a11y:keyboard-small` for primary-nav disclosure, visible focus, and skip-link order. `website/src/tests/docs-shell.test.ts` locks `public-site.chrome:docs-sidebar` for handbook aside, article, and status Badge chrome that is not the home hero. Asserted with no test yet: `public-site.home:landing`, `public-site.chrome:primary-nav`, `public-site.search:titles-headings`, `public-site.forbid-vault-as-site`, `public-site.forbid-playground`.

## Tests

- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_renders_markdown_subset`
  - **How:** Fixture Learn markdown with heading, paragraph, list, fence, and link renders those HTML elements.
  - **Why:** Locks `public-site.markdown:subset`.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_learn_skeleton_is_walkable`
  - **How:** Repo Learn pages generate; each chapter is linked; from JavaScript and from systems join at Dual worlds.
  - **Why:** Locks `public-site.ia:learn-walkable`.
- **website/src/tests/learn-hub-nav.test.ts** — `learn hub nav`
  - **How:** Learn hub file route loads `website/learn.md` in DocsShell; aside lists Install through packages in hub order; the two landings join at Dual worlds.
  - **Why:** Locks `public-site.ia:learn-walkable` for the Start hub and aside.
- **website/src/tests/learn-pages.test.ts** — `learn pages`
  - **How:** Each existing Learn markdown file has a Start file route that loads teaching copy in LearnPage and DocsShell; Badge status matches frontmatter; rendered in-site links are app routes, not `.html`.
  - **Why:** Locks `public-site.ia:learn-walkable` for chapter routes.
- **website/src/tests/learn-prev-next.test.ts** — `learn prev next`
  - **How:** Sequence helper neighbors match `website/learn.md`; both landings next to Dual worlds; LearnPage article footer renders LearnPager; no extra stops.
  - **Why:** Locks `public-site.ia:learn-walkable` for walkable prev/next.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_reference_skeleton_is_walkable`
  - **How:** Repo Reference pages generate; CLI, types, Dual-world rules, host I/O, and packages are linked.
  - **Why:** Locks `public-site.ia:reference-walkable`.
- **website/src/tests/reference-hub-pages.test.ts** — `reference hub pages`
  - **How:** Reference hub file route loads `website/reference.md` in DocsShell; aside lists CLI, types, Dual-world rules, host I/O, and packages in hub order; each working markdown file has a Start file route that loads teaching copy in ReferencePage; Badge status matches frontmatter; rendered in-site links are app routes, not `.html`.
  - **Why:** Locks `public-site.ia:reference-walkable` for the Start hub, aside, and working pages.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_shipped_drac_fence_builds`
  - **How:** A shipped `drac` fence is extracted and `draconic build` writes an artifact.
  - **Why:** Locks `public-site.fences:shipped-must-build`.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_shipped_invalid_drac_fence_fails`
  - **How:** An invalid shipped `drac` fence fails the pipeline with a build error.
  - **Why:** Same shipped-fence risk; support case, not a new promise.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_not_yet_page_with_fence_fails`
  - **How:** A not-yet page that includes a fence fails the pipeline.
  - **Why:** Locks `public-site.fences:forbid-not-yet-fences`.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_not_yet_page_without_fence_generates`
  - **How:** A not-yet page with no fence still generates.
  - **Why:** Same forbid; prose-only not-yet pages are allowed.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_learn_and_reference_nav_and_status`
  - **How:** Learn and Reference HTML include those nav links and a visible shipped or not-yet status.
  - **Why:** Locks `public-site.nav:learn-reference-status`.
- **website/src/tests/mobile-a11y.test.ts** — `mobile a11y`
  - **How:** Header source is a keyboard disclosure (button plus panel) of Wordmark, Learn, Reference, and GitHub; focus rings use tokens; skip link stays first in the root layout.
  - **Why:** Locks `public-site.a11y:keyboard-small`.
- **website/src/tests/docs-shell.test.ts** — `docs shell`
  - **How:** DocsShell is aside plus article plus Badge from shipped or not-yet; home and root do not use that chrome; tokens and CVA, no hex.
  - **Why:** Locks `public-site.chrome:docs-sidebar`.

## Gaps

No test yet for `public-site.home:landing`, `public-site.search:titles-headings`, or `public-site.chrome:primary-nav`. Those promises stay asserted. `public-site.forbid-vault-as-site` and `public-site.forbid-playground` are also asserted.
