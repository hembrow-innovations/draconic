---
id: "task-615-home-hero-cta"
title: "Add home hero and CTAs"
kind: task
status: completed
mode: afk
blocked_by: ["task-614-site-footer-skip"]
sprint: "website-redesign"
slice: "slice-596-home-hero-cta"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T23:50:00Z"
---

# Add home hero and CTAs

## Blocked by

[[task-614-site-footer-skip]]: `/` should sit inside finished site chrome (header, skip, footer).

## Done

`/` is a language homepage with a pitch plus Install and Learn CTAs, not `learn.md` copied to index.

## Context

The old generate script copied `learn.html` to `index.html`. That is not a language homepage. Pitch from `website/learn.md` and `CONTEXT.md`: JavaScript you already know, native types when you need them, one language two backends. CTAs are Install and Learn. Do not rewrite Learn chapter markdown.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Route `/` renders a hero with Install and Learn CTAs. It is not the Learn hub body. Typecheck holds.

scope: home route under `website/src/`

## Links

[[slice-596-home-hero-cta]] [[task-614-site-footer-skip]]

## Agent Brief

**Category:** enhancement
**Summary:** Make `/` a language homepage with Install and Learn CTAs.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.home:landing`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`scripts/generate-website.sh` copies `learn.html` to `index.html`. There is no marketing home in the Start app.

**Desired behavior:**
`/` is a language homepage like TypeScript.org: a short pitch, then CTAs Install and Learn. Pitch language comes from `website/learn.md` and `CONTEXT.md`. Do not paste the Learn hub article onto `/`. Feature grid waits for [[task-616-home-features]].

**Key interfaces:**
- File route for `/`
- Home hero in a nested feature folder
- Links to Install and Learn (app routes, not `.html`)
- Load **frontend-development** (`start-file-routes`, `token-semantic-roles`)

**Acceptance criteria:**
- [x] `/` is a language homepage, not Learn copied to index
- [x] CTAs are Install and Learn
- [x] Pitch does not invent playground, vault-as-site, or new glossary terms
- [x] Learn chapter markdown files are not rewritten
- [x] If [[task-614-site-footer-skip]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Home feature grid ([[task-616-home-features]])
- Rewriting `website/learn.md` or chapter files

**Explain this part:**
`/` is a language homepage like TypeScript.org, not `learn.md` copied to index. The job of home is to say what Draconic is and send people to Install or Learn. The job of Learn is the walkable path after they choose to learn.
