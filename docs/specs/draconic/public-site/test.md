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

Existing cases in `tests/integration/tests/website_pipeline.rs` lock `public-site.markdown:subset`, `public-site.ia:learn-walkable`, `public-site.ia:reference-walkable`, `public-site.fences:shipped-must-build`, `public-site.fences:forbid-not-yet-fences`, and `public-site.nav:learn-reference-status`. Related fence cases in the same file support those locks. `website/src/tests/learn-hub-nav.test.ts` also locks `public-site.ia:learn-walkable` for the Start Learn hub and aside sequence, `public-site.chrome:current-page` for Learn chapter current-page chrome, and `public-site.a11y:distinct-nav-names` for host I/O and packages accessible names. `website/src/tests/learn-pages.test.ts` locks `public-site.ia:learn-walkable` for one Start route per Learn chapter, teaching copy, frontmatter Badge status, and `.html` hrefs rewritten to app routes. `website/src/tests/learn-prev-next.test.ts` locks `public-site.ia:learn-walkable` for article-footer prev/next that follows `learn.md`, with both landings continuing to Dual worlds. `website/src/tests/reference-hub-pages.test.ts` locks `public-site.ia:reference-walkable` for the Start Reference hub, aside sequence, and working-page routes from `website/content/*.md`, `public-site.chrome:current-page` for Reference chapter current-page chrome, and `public-site.a11y:distinct-nav-names` for host I/O and packages accessible names. `website/src/tests/reference-prev-next.test.ts` locks `public-site.ia:reference-walkable` for article-footer prev/next that follows CLI, types, Dual-world rules, host I/O, and packages, and `public-site.chrome:docs-article-order` for the related-link footer on Reference working pages. `website/src/tests/mobile-a11y.test.ts` locks `public-site.a11y:keyboard-small` for stacked wrap, visible focus, and skip-link order. `website/src/tests/docs-shell.test.ts` locks `public-site.chrome:docs-sidebar` for handbook aside, article, and status Badge chrome that is not the home hero, `public-site.chrome:docs-article-order` for kicker then heading then badge then on-page outline then related-link footer filled by LearnPage and ReferencePage, and `public-site.chrome:on-page-toc` for in-article heading links. `website/src/tests/search.test.ts` locks `public-site.search:titles-headings` for a title and heading index whose Dual worlds query reaches the Learn chapter, `public-site.search:keyboard-live` for Arrow Down, Arrow Up, Enter, and live announcement of hits or No matching pages, and `public-site.search:session` for query and result reset on navigation and Escape so leftover hits cannot keep aria-current. `website/src/tests/site-header-primary-nav.test.ts` locks `public-site.chrome:primary-nav` and `public-site.chrome:odm-shell` for sticky side-nav wordmark, Learn, Reference, and GitHub, `public-site.chrome:current-page` for hub current-page chrome, and `public-site.chrome:favicon` for a served `/favicon.ico` plus a document icon link. `website/src/tests/site-footer-and-skip-link.test.ts` locks `public-site.chrome:odm-shell` for skip-to-content, sticky side nav, and main column. Asserted with no test yet: `public-site.home:landing`, `public-site.forbid-vault-as-site`, `public-site.forbid-playground`.

`website/src/tests/document-title.test.ts` locks `public-site.chrome:document-title` for per-page document titles. Home may remain Draconic. `website/src/tests/not-found.test.ts` locks `public-site.chrome:not-found` for unknown-URL recovery. `website/src/tests/code-fence-copy.test.ts` locks `public-site.fences:copy` for copy-the-text and `public-site.fences:copy-announce` for distinct names, live confirmation, and clearing Copied. `website/src/tests/typography-and-badge.test.ts` locks `public-site.nav:learn-reference-status` for shipped-chip 4.5:1 contrast at 14px so the 4.37 light pair cannot pass.

## Tests

- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_renders_markdown_subset`
  - **How:** Fixture Learn markdown with heading, paragraph, list, fence, and link renders those HTML elements.
  - **Why:** Locks `public-site.markdown:subset`.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_learn_skeleton_is_walkable`
  - **How:** Repo Learn pages generate; each chapter is linked; from JavaScript and from systems join at Dual worlds.
  - **Why:** Locks `public-site.ia:learn-walkable`.
- **website/src/tests/learn-hub-nav.test.ts** — `learn hub nav`
  - **How:** Learn hub file route loads `website/content/learn.md` in DocsShell; aside lists Install through packages in hub order; the two landings join at Dual worlds; current chapter chrome uses an existing token with aria-current, not muted ink alone, and current-page ink meets 4.5:1 against the canvas so the 3:1 accent-2-on-canvas pair cannot pass; host I/O and packages links have distinct accessible names from the matching Reference links (`Learn ·` versus `Reference ·`) while visible CONTEXT terms stay.
  - **Why:** Locks `public-site.ia:learn-walkable` for the Start hub and aside, `public-site.chrome:current-page` for Learn chapter links, and `public-site.a11y:distinct-nav-names` for shared CONTEXT terms.
- **website/src/tests/learn-pages.test.ts** — `learn pages`
    - **How:** Each existing Learn markdown file has a Start file route that loads teaching copy in LearnPage and DocsShell; Badge status matches frontmatter; rendered in-site links are app routes, not `.html`. Install names `node` on PATH before the first `draconic run` command. from JavaScript also locks the unresolved `console` bind, a second compiling `greet` fence, a GitHub FizzBuzz example link, and a Todo heading with a compiling `globalThis.document`/`globalThis.localStorage` fence, `draconic build --target js todo.drac -o todo.js`, a `todo.js` script tag, `stdoutWrite`/`tcpListen` as the machine path not the DOM path, and a GitHub Todo example link. from systems locks a compiling hello Program, a compiling `i32` `add` fence, native build commands, and a GitHub HTTP echo example link. Dual worlds locks a compiling `as i32` boundary fence plus `draconic build --target native boundary.drac`. modules is shipped and locks a compiling named `export function greet` fence, `draconic check`, a relative `./greet.drac` import, a complete `main.drac` entry that logs `greet("from the entry")`, and `draconic check main.drac` plus `draconic run main.drac`. native types is shipped and locks compiling `i32`/`i64`, `i8`/`u8`/`f32`/`bool`, fixed-struct, and fixed-array fences plus `draconic build --target native width.drac`, a Pointers heading with `*i32` address-of as a non-`drac` fence, and a GitHub examples/types link. host I/O is shipped and locks compiling `stdoutWrite` and write-then-read `writeFileText`/`readFileText` fences, a compiling `processArgs` fence with `envGet`/`stdinReadLine` lookup, `draconic check args.drac`, a compiling `pathJoin` fence with `pathNormalize`/`pathDirname`/`pathBasename`/`pathExtname`/`pathIsAbsolute`/`pathResolve` lookup and `draconic check join.drac`, a compiling `mkdir`/`exists` fence with `mkdirAll`/`readdir`/`rmdir`/`removeFile`/`renameFile`/`copyFile` lookup and `draconic check dirs.drac`, a compiling `tcpListen` fence with `draconic check listen.drac` and native build commands, a compiling HTTP echo fence with `tcpAccept`, `tcpRead(a, 65536)`, `httpParseRequest`, `req.path`, and `httpWriteResponse`, named `httpParseRequest`, a GitHub HTTP echo example link, and a GitHub Flagship service example link. packages is shipped and locks a compiling package-root fence, get/tidy argv, GitHub pkg-lib and pkg-consumer links, and a Flagship service heading with a GitHub Flagship service example link.
  - **Why:** Locks `public-site.ia:learn-walkable` for chapter routes.
- **website/src/tests/learn-prev-next.test.ts** — `learn prev next`
  - **How:** Sequence helper neighbors match `website/content/learn.md`; both landings next to Dual worlds; LearnPage article footer renders LearnPager; no extra stops.
  - **Why:** Locks `public-site.ia:learn-walkable` for walkable prev/next.
- **tests/integration/tests/website_pipeline.rs** — `website_pipeline_reference_skeleton_is_walkable`
  - **How:** Repo Reference pages generate; CLI, types, Dual-world rules, host I/O, and packages are linked.
  - **Why:** Locks `public-site.ia:reference-walkable`.
- **website/src/tests/reference-hub-pages.test.ts** — `reference hub pages`
    - **How:** Reference hub file route loads `website/content/reference.md` in DocsShell; aside lists CLI, types, Dual-world rules, host I/O, and packages in hub order; each working markdown file has a Start file route that loads teaching copy in ReferencePage; Badge status matches frontmatter; rendered in-site links are app routes, not `.html`; current working-page chrome uses an existing token with aria-current, not muted ink alone, and current-page ink meets 4.5:1 against the canvas so the 3:1 accent-2-on-canvas pair cannot pass; host I/O and packages links have distinct accessible names from the matching Learn links (`Learn ·` versus `Reference ·`) while visible CONTEXT terms stay. CLI is shipped and locks extract, doc, and bindgen headings, a compiling doc-comment fence, default `{stem}.out.js` output, `--coverage`, `.exit`, and leftover run args as `processArgs`. types is shipped and locks compiling JS-value, object-type, union, generic, `i32`, `i8`/`u8`/`f32`/`bool`, fixed-struct, fixed-array, and `as` fences plus `draconic check` and a Pointers heading. Dual-world rules is shipped and locks compiling `as` and `try`/`catch` fences plus `draconic check`. host I/O is shipped and locks write-time call shapes (`processArgs()`, `envGet(key)`, `pathJoin(...)`, `pathNormalize(path)`, `exists(path)`, `mkdir(path)`, `mkdirAll(path)`, `readdir(path)`, `rmdir(path)`, `removeFile(path)`, `renameFile(from, to)`, `copyFile(from, to)`, `stat(path)`, `tcpListen(port)`, `tcpRead(connection, maxLen)`, `httpParseRequest(raw)` with `method`/`path`/`version`/`body`, `httpWriteResponse(status, reason, headers, body)`) plus compiling `processArgs`, `pathJoin`, `mkdir`/`exists`, and HTTP echo fences.
  - **Why:** Locks `public-site.ia:reference-walkable` for the Start hub, aside, and working pages, `public-site.chrome:current-page` for Reference chapter links, and `public-site.a11y:distinct-nav-names` for shared CONTEXT terms.
- **website/src/tests/reference-prev-next.test.ts** — `reference prev next`
  - **How:** Sequence helper neighbors match CLI, types, Dual-world rules, host I/O, and packages; first stop has no previous and last stop has no next; ReferencePage article footer renders ReferencePager; hub keeps cards; LearnPager is not the mechanism.
  - **Why:** Locks `public-site.ia:reference-walkable` for walkable prev/next and `public-site.chrome:docs-article-order` for the related-link footer on Reference working pages.
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
  - **Why:** Locks `public-site.nav:learn-reference-status` for visible status.
- **website/src/tests/typography-and-badge.test.ts** — `typography and badge`
  - **How:** Shipped Badge CVA keeps accent fill and accent-foreground text at text-mono; light theme is not the 4.37 pair of rgb(248, 251, 255) on rgb(49, 120, 198); light and dark pairs meet 4.5:1; no hex in the Badge component.
  - **Why:** Locks `public-site.nav:learn-reference-status` for readable shipped-chip contrast.
- **website/src/tests/mobile-a11y.test.ts** — `mobile a11y`
  - **How:** Small-viewport shell is one column with the side nav stacked and wrapping; Wordmark, Learn, Reference, and GitHub stay keyboard-reachable without a Menu button; focus rings use tokens; skip link stays first in the root layout.
  - **Why:** Locks `public-site.a11y:keyboard-small`.
- **website/src/tests/docs-shell.test.ts** — `docs shell`
  - **How:** DocsShell is aside plus article plus Badge from shipped or not-yet; article children are kicker, then heading, then Badge, then OnThisPage, then remaining markdown, then footer; LearnPage and ReferencePage pass pager children into that footer; home and root do not use that chrome; tokens and CVA, no hex.
  - **Why:** Locks `public-site.chrome:docs-sidebar`, `public-site.chrome:docs-article-order`, and `public-site.chrome:on-page-toc`.
- **website/src/tests/docs-shell.test.ts** — `page outline`
  - **How:** Install outline ids match heading permalinks; Dual worlds has no section headings so the outline is empty; CLI outline names parse through Shebang.
  - **Why:** Locks `public-site.chrome:on-page-toc` for fragment ids that match rendered headings.
- **website/src/tests/search.test.ts** — `search`
    - **How:** Static index of routed `website/content/*.md` titles and headings; query Dual worlds hits `/dual-worlds`; query Todo hits `/from-javascript#todo` and labels Learn · from JavaScript · Todo; query Entry hits `/modules#entry` and labels Learn · modules · Entry; query Fixed structs hits `/native-types#fixed-structs` and `/types#fixed-structs` and labels Learn · native types · Fixed structs and Reference · types · Fixed structs; query Object types, Unions and intersections, and Generics hit `/types` with section hashes; query i32 hits `/native-types#i32-and-i64`; query i8 hits `/native-types#i8-u8-f32-and-bool` and `/types#i8-u8-f32-and-bool`; query Fixed arrays hits `/native-types#fixed-arrays` and `/types#fixed-arrays`; query Pointers hits `/native-types#pointers` and `/types#pointers`; query packages labels Learn · packages and Reference · packages; query Flagship service hits `/packages#flagship-service` and labels Learn · packages · Flagship service; query extract, doc, and bindgen hit `/cli` with section hashes; query processArgs hits `/host-io#processargs` and `/reference-host-io#processargs`; query pathJoin hits `/host-io#pathjoin` and `/reference-host-io#pathjoin`; query mkdir hits `/host-io#mkdir` and `/reference-host-io#mkdir`; query exists hits `/reference-host-io#exists`; query HTTP echo hits `/host-io#http-echo` and `/reference-host-io#http-echo`; body-only and vault phrases miss; a miss shows No matching pages; SiteSearch in site chrome links to Start routes, with heading hits carrying the section hash; SiteSearch is a combobox: Arrow Down and Arrow Up move through visible hits, Enter follows the selected hit including the first hit, and a live region announces hits or No matching pages; SiteSearch resets the query on location change and Escape so leftover hits cannot keep aria-current.
   - **Why:** Locks `public-site.search:titles-headings`, `public-site.search:keyboard-live`, and `public-site.search:session`.
- **website/src/tests/markdown-render.test.ts** — `markdown render install subset`
   - **How:** Install h1 stays without an id so DocsShell can peel it; Reproducibility renders as h2 with id reproducibility; native types h2s are i32-and-i64, i8-u8-f32-and-bool, fixed-structs, fixed-arrays, and pointers.
  - **Why:** Supports `public-site.search:titles-headings` heading fragments on the markdown subset.
- **website/src/tests/site-header-primary-nav.test.ts** — `site header primary nav`
  - **How:** Root layout is skip then sticky side nav then main; side nav source has wordmark, Learn, Reference, GitHub, search, and theme toggle; home and Learn routes do not remount that chrome; current hub chrome uses an existing token with aria-current, not muted ink alone, and current-page ink meets 4.5:1 against the canvas so the 3:1 accent-2-on-canvas pair cannot pass; root links `/favicon.ico` and `website/public/favicon.ico` is a non-empty ICO.
  - **Why:** Locks `public-site.chrome:primary-nav`, `public-site.chrome:odm-shell`, `public-site.chrome:current-page`, and `public-site.chrome:favicon`.
- **website/src/tests/site-footer-and-skip-link.test.ts** — `site footer and skip link`
  - **How:** Skip link is first and targets `#main`; root wraps sticky `aside` plus `main`; slim footer stays inside main.
  - **Why:** Locks `public-site.chrome:odm-shell`.
- **website/src/tests/home-hero-and-cta.test.ts** — `home hero and cta`
  - **How:** `/` hero uses Learn pitch language and CTAs to Install and Learn; the Learn hub article stays off the index.
  - **Why:** Locks `public-site.home:landing`.
- **website/src/tests/home-sample.test.ts** — `home sample`
  - **How:** `/` shows static hello.drac, typed greet.drac with `draconic check` and a types doorway, and native width.drac with `i32`/`i64`, Dual-world `as`, `--target native`, and a native-types doorway. Not a playground.
  - **Why:** Locks `public-site.home:landing`.
- **website/src/tests/home-features.test.ts** — `home features`
  - **How:** `/` feature grid names JavaScript, LLVM, and Dual worlds. No extra product claims.
  - **Why:** Locks `public-site.home:landing`.
- **website/src/tests/document-title.test.ts** — `document title`
  - **How:** Learn, Reference, and an article route do not all share only the title Draconic; each names the open page from existing markdown title; home may remain Draconic; route files do not rewrite page h1 copy.
  - **Why:** Locks `public-site.chrome:document-title`.
- **website/src/tests/not-found.test.ts** — `not found`
  - **How:** Root sets `notFoundComponent`; recovery main is a heading that names the miss plus an in-site Home or Learn link, not the router default Not Found paragraph; document title names the miss; skip-link, sticky side nav, and footer stay on root; no splat route that would 200; no `aria-current=page` on the recovery view.
  - **Why:** Locks `public-site.chrome:not-found`.
- **website/src/tests/code-fence-copy.test.ts** — `code fence copy`
  - **How:** DocsShell splits article HTML into fences; CodeFence copies fence text; each control has a distinct accessible name rather than a shared Copy; a live region announces a successful copy; Copied clears on a timer.
  - **Why:** Locks `public-site.fences:copy` and `public-site.fences:copy-announce`.

## Gaps

`public-site.forbid-vault-as-site` and `public-site.forbid-playground` are asserted.
