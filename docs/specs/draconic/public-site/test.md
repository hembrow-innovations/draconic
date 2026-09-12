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
updated_at: "2026-09-12"
---

# Public site tests

Purpose: [[Public site purpose]]. Contract: [[Public site — Contract]].

## Coverage

Existing cases in `tests/integration/tests/website_pipeline.rs` lock `public-site.markdown:subset`, `public-site.ia:learn-walkable`, `public-site.ia:reference-walkable`, `public-site.fences:shipped-must-build`, `public-site.fences:forbid-not-yet-fences`, and `public-site.nav:learn-reference-status`. Related fence cases in the same file support those locks. `website/src/tests/learn-hub-nav.test.ts` also locks `public-site.ia:learn-walkable` for the Start Learn hub and aside sequence, and `public-site.chrome:current-page` for Learn chapter current-page chrome. `website/src/tests/learn-pages.test.ts` locks `public-site.ia:learn-walkable` for one Start route per Learn chapter, teaching copy, frontmatter Badge status, and `.html` hrefs rewritten to app routes. `website/src/tests/learn-prev-next.test.ts` locks `public-site.ia:learn-walkable` for article-footer prev/next that follows `learn.md`, with both landings continuing to Dual worlds. `website/src/tests/reference-hub-pages.test.ts` locks `public-site.ia:reference-walkable` for the Start Reference hub, aside sequence, and working-page routes from `website/*.md`, and `public-site.chrome:current-page` for Reference chapter current-page chrome. `website/src/tests/mobile-a11y.test.ts` locks `public-site.a11y:keyboard-small` for stacked wrap, visible focus, and skip-link order. `website/src/tests/docs-shell.test.ts` locks `public-site.chrome:docs-sidebar` for handbook aside, article, and status Badge chrome that is not the home hero, and `public-site.chrome:docs-article-order` for kicker then heading then badge. `website/src/tests/search.test.ts` locks `public-site.search:titles-headings` for a title and heading index whose Dual worlds query reaches the Learn chapter. `website/src/tests/site-header-primary-nav.test.ts` locks `public-site.chrome:primary-nav` and `public-site.chrome:odm-shell` for sticky side-nav wordmark, Learn, Reference, and GitHub, and `public-site.chrome:current-page` for hub current-page chrome. `website/src/tests/site-footer-and-skip-link.test.ts` locks `public-site.chrome:odm-shell` for skip-to-content, sticky side nav, and main column. Asserted with no test yet: `public-site.home:landing`, `public-site.forbid-vault-as-site`, `public-site.forbid-playground`.

## Tests

- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_renders_markdown_subset`
  - **How:** Fixture Learn markdown with heading, paragraph, list, fence, and link renders those HTML elements.
  - **Why:** Locks `public-site.markdown:subset`.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_learn_skeleton_is_walkable`
  - **How:** Repo Learn pages generate; each chapter is linked; from JavaScript and from systems join at Dual worlds.
  - **Why:** Locks `public-site.ia:learn-walkable`.
- **website/src/tests/learn-hub-nav.test.ts** — `learn hub nav`
  - **How:** Learn hub file route loads `website/learn.md` in DocsShell; aside lists Install through packages in hub order; the two landings join at Dual worlds; current chapter chrome uses an existing token with aria-current, not muted ink alone.
  - **Why:** Locks `public-site.ia:learn-walkable` for the Start hub and aside, and `public-site.chrome:current-page` for Learn chapter links.
- **website/src/tests/learn-pages.test.ts** — `learn pages`
   - **How:** Each existing Learn markdown file has a Start file route that loads teaching copy in LearnPage and DocsShell; Badge status matches frontmatter; rendered in-site links are app routes, not `.html`. Install names `node` on PATH before the first `draconic run` command. from JavaScript also locks the unresolved `console` bind, a second compiling `greet` fence, and a GitHub FizzBuzz example link. from systems locks a compiling hello Program, a compiling `i32` `add` fence, native build commands, and a GitHub HTTP echo example link. Dual worlds locks a compiling `as i32` boundary fence. modules is shipped and locks a compiling named `export function greet` fence, `draconic check`, and a relative `./greet.drac` import in prose. native types is shipped and locks compiling `i32`/`i64` and fixed-struct fences.
  - **Why:** Locks `public-site.ia:learn-walkable` for chapter routes.
- **website/src/tests/learn-prev-next.test.ts** — `learn prev next`
  - **How:** Sequence helper neighbors match `website/learn.md`; both landings next to Dual worlds; LearnPage article footer renders LearnPager; no extra stops.
  - **Why:** Locks `public-site.ia:learn-walkable` for walkable prev/next.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_reference_skeleton_is_walkable`
  - **How:** Repo Reference pages generate; CLI, types, Dual-world rules, host I/O, and packages are linked.
  - **Why:** Locks `public-site.ia:reference-walkable`.
- **website/src/tests/reference-hub-pages.test.ts** — `reference hub pages`
   - **How:** Reference hub file route loads `website/reference.md` in DocsShell; aside lists CLI, types, Dual-world rules, host I/O, and packages in hub order; each working markdown file has a Start file route that loads teaching copy in ReferencePage; Badge status matches frontmatter; rendered in-site links are app routes, not `.html`; current working-page chrome uses an existing token with aria-current, not muted ink alone. types is shipped and locks compiling JS-value, `i32`, and `as` fences plus `draconic check`. Dual-world rules is shipped and locks compiling `as` and `try`/`catch` fences plus `draconic check`.
  - **Why:** Locks `public-site.ia:reference-walkable` for the Start hub, aside, and working pages, and `public-site.chrome:current-page` for Reference chapter links.
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
  - **How:** Small-viewport shell is one column with the side nav stacked and wrapping; Wordmark, Learn, Reference, and GitHub stay keyboard-reachable without a Menu button; focus rings use tokens; skip link stays first in the root layout.
  - **Why:** Locks `public-site.a11y:keyboard-small`.
- **website/src/tests/docs-shell.test.ts** — `docs shell`
  - **How:** DocsShell is aside plus article plus Badge from shipped or not-yet; article children are kicker, then heading, then Badge, then remaining markdown, then footer; home and root do not use that chrome; tokens and CVA, no hex.
  - **Why:** Locks `public-site.chrome:docs-sidebar` and `public-site.chrome:docs-article-order`.
- **website/src/tests/search.test.ts** — `search`
   - **How:** Static index of routed `website/*.md` titles and headings; query Dual worlds hits `/dual-worlds`; query Fixed structs hits `/native-types`; body-only and vault phrases miss; SiteSearch in site chrome links to Start routes.
  - **Why:** Locks `public-site.search:titles-headings`.
- **website/src/tests/site-header-primary-nav.test.ts** — `site header primary nav`
  - **How:** Root layout is skip then sticky side nav then main; side nav source has wordmark, Learn, Reference, GitHub, search, and theme toggle; home and Learn routes do not remount that chrome; current hub chrome uses an existing token with aria-current, not muted ink alone.
  - **Why:** Locks `public-site.chrome:primary-nav`, `public-site.chrome:odm-shell`, and `public-site.chrome:current-page`.
- **website/src/tests/site-footer-and-skip-link.test.ts** — `site footer and skip link`
  - **How:** Skip link is first and targets `#main`; root wraps sticky `aside` plus `main`; slim footer stays inside main.
  - **Why:** Locks `public-site.chrome:odm-shell`.

## Gaps

No test yet for `public-site.home:landing`. `public-site.forbid-vault-as-site` and `public-site.forbid-playground` are also asserted.
