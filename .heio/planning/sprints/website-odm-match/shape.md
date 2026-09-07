---
id: "website-odm-match"
title: "Public site ODM chrome"
kind: sprint
status: active
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T23:15:00Z"
---

# Public site ODM chrome

## Grouping

Location [[location-589-public-site]]. Vertical cuts to restyle the existing TanStack Start app so chrome, tokens, and docs shell match the ODM public site (two-column sticky side nav, dark navy tokens, grouped docs nav). One thin slice per sitting. Language ROADMAP.md loop is out.

## Slices in

- [[slice-629-odm-tokens]]: dark navy semantic tokens. blocked_by: none
- [[slice-630-site-shell]]: skip plus sticky side nav plus main on every page. blocked_by: [[slice-629-odm-tokens]]
- [[slice-631-home-odm-layout]]: home article uses kicker, CTA row, path steps, and cards. blocked_by: [[slice-630-site-shell]]
- [[slice-632-docs-article-odm]]: Learn and Reference article column uses kicker, badge, and ODM prose. blocked_by: [[slice-629-odm-tokens]] [[slice-630-site-shell]]
- [[slice-633-docs-nav-groups]]: side nav lists Learn and Reference pages in groups with aria-current. blocked_by: [[slice-630-site-shell]]
- [[slice-634-hub-cards]]: Learn and Reference hubs use a card grid. blocked_by: [[slice-632-docs-article-odm]]
- [[slice-635-mobile-odm-wrap]]: small viewport stacks the side nav and wraps links. blocked_by: [[slice-630-site-shell]]
- [[slice-644-ui-audit]]: walk the restyled site and file tickets. blocked_by: [[slice-631-home-odm-layout]] [[slice-632-docs-article-odm]] [[slice-633-docs-nav-groups]] [[slice-634-hub-cards]] [[slice-635-mobile-odm-wrap]]
- [[slice-673-search-below-nav-fold]]: search and theme toggle stay above the aside fold. blocked_by: none
- [[slice-675-search-results-clipped]]: Dual worlds hits are not clipped by aside overflow. blocked_by: [[slice-673-search-below-nav-fold]]
- [[slice-679-unescaped-angle-placeholders]]: list placeholders keep angle brackets as text. blocked_by: none
- [[slice-683-badge-before-heading]]: docs article is kicker, heading, then badge. blocked_by: none
- [[slice-685-chapter-nav-focus-ring]]: chapter links get token focus rings. blocked_by: none
- [[slice-687-chapter-nav-touch-target]]: chapter links match hub min-h-11. blocked_by: [[slice-685-chapter-nav-focus-ring]]
- [[slice-681-current-page-visual]]: current nav item is visually distinct. blocked_by: [[slice-687-chapter-nav-touch-target]]
- [[slice-689-first-h2-border]]: first section h2 gets the ODM rule. blocked_by: [[slice-683-badge-before-heading]]
- [[slice-677-fence-horizontal-overflow]]: small-viewport fences do not widen the page. blocked_by: [[slice-689-first-h2-border]]

## Slices out

- playground
- in-page runners
- serving docs/ vault
- replacing TanStack Start with static HTML
- Next.js/VitePress/Starlight/mdBook
- dropping search, theme toggle, or fence compile
- copying ODM product copy
- language ROADMAP atoms
- rewriting CONTEXT.md Learn/Reference terms

## Drain

`/afk-slice` from unblocked UI-audit follow-ups. First runnable tasks are [[task-674-search-below-nav-fold]], [[task-680-unescaped-angle-placeholders]], [[task-684-badge-before-heading]], and [[task-686-chapter-nav-focus-ring]] (`status: ready`, `mode: afk`, empty `blocked_by`). Later tasks stay `ready` but drain must honor `blocked_by`. Public site is `website/` TanStack Start, not `ui-components-web`.
