---
id: "task-613-site-header-nav"
title: "Add site header and primary nav"
kind: task
status: completed
mode: afk
blocked_by: [ "task-612-typography-badge" ]
sprint: "website-redesign"
slice: "slice-594-site-header-nav"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T13:18:54Z"
---
# Add site header and primary nav

## Blocked by

[[task-612-typography-badge]]: header chrome should use type roles; do not invent a second type scale.

## Done

Root layout shows a site header whose primary nav is Wordmark, Learn, Reference, and GitHub.

## Context

Primary nav is information architecture, not decoration. Visitors must be able to reach Learn, Reference, and the repo from every page. GitHub URL is `https://github.com/hembrow-innovations/draconic`. Mobile disclosure waits for [[task-626-mobile-a11y]]. Skip link and footer wait for [[task-614-site-footer-skip]].

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Header is in the root layout. Links: wordmark/home, Learn, Reference, GitHub (that URL). Typecheck holds.

scope: header component plus root layout under `website/src/`

## Links

[[slice-594-site-header-nav]] [[task-612-typography-badge]]

## Agent Brief

**Category:** enhancement
**Summary:** Put Wordmark, Learn, Reference, and GitHub in the site header on the root layout.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.chrome:primary-nav`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`generate.drac` emits a thin nav of Learn, Reference, GitHub with `.html` hrefs. The Start app has no site chrome yet.

**Desired behavior:**
Root layout renders a header. Primary items are Wordmark (home), Learn, Reference, GitHub. GitHub is `https://github.com/hembrow-innovations/draconic`. Use tokens and type roles. Do not dump the Learn chapter list into the site header.

**Key interfaces:**
- Header component folder under nested `website/src/`
- Root file-route layout
- Router links for in-app destinations; external anchor for GitHub
- Load **frontend-development** (`start-file-routes`, `package-component-layout`, `token-semantic-roles`)

**Acceptance criteria:**
- [x] Primary nav is Wordmark, Learn, Reference, GitHub
- [x] GitHub URL is `https://github.com/hembrow-innovations/draconic`
- [x] Header is site chrome on the root layout, not copied into each page
- [x] No playground link; no vault/`docs/` link
- [x] No hardcoded hex; no named `max-w-sm`
- [x] If [[task-612-typography-badge]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Footer and skip link ([[task-614-site-footer-skip]])
- Mobile disclosure ([[task-626-mobile-a11y]])
- Learn sidebar ([[task-619-docs-shell]])

**Explain this part:**
Primary nav is IA: Wordmark, Learn, Reference, GitHub. Those four answers “where am I, how do I learn, how do I look up, where is the code.” Chapter lists belong in Learn or Reference chrome, not in the site header.
