---
id: "task-612-typography-badge"
title: "Add typography scale and status Badge"
kind: task
status: completed
mode: afk
blocked_by: ["task-611-semantic-tokens"]
sprint: "website-redesign"
slice: "slice-593-typography-badge"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T13:07:30Z"
---

# Add typography scale and status Badge

## Blocked by

[[task-611-semantic-tokens]]: Badge variants must consume role tokens, not hex.

## Done

Display, body, and mono type roles exist, and a shipped/not-yet Badge lives in a component folder with CVA in `.variants.ts`.

## Context

Every public page is tagged `shipped` or `not-yet` in markdown frontmatter. The Badge is how that status shows in chrome. It is a primitive with variants, not a span with a class soup. Copy the neighboring primitive folder layout from the frontend skill (folder per component: component, types, variants, index).

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Typography tokens exist. Badge folder includes `.variants.ts`. Typecheck holds. No page copy rewrite.

scope: typography tokens plus Badge component folder under nested `website/src/`

## Links

[[slice-593-typography-badge]] [[task-611-semantic-tokens]]

## Agent Brief

**Category:** enhancement
**Summary:** Add display/body/mono type roles and a CVA Badge for shipped versus not-yet.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: none (primitive; status display is used by later chrome promises)
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`generate.drac` inlines a status badge string. The Start app has tokens but no type scale primitive and no Badge folder.

**Desired behavior:**
Typography roles cover display, body, and mono. Badge has variants `shipped` and `not-yet` in `Badge.variants.ts`. Classes are not inlined in JSX. Folder shape matches the frontend primitive layout.

**Key interfaces:**
- Typography tokens in `@theme`
- `website/src/.../Badge/` with `index.ts`, `Badge.tsx`, `Badge.types.ts`, `Badge.variants.ts`
- Load **frontend-development** (`package-component-layout`, `cva-variants-file`, `quality-typography-spacing`)

**Acceptance criteria:**
- [x] Display, body, and mono roles exist as tokens
- [x] Badge lives in a component folder, not a lone file at `src/`
- [x] CVA lives in `.variants.ts`, not inline class soup
- [x] Variants cover shipped and not-yet
- [x] No hardcoded hex; no named `max-w-sm`
- [x] If [[task-611-semantic-tokens]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Header, footer, Learn pages, markdown loader
- Changing `status` frontmatter in `website/*.md`

**Explain this part:**
Display, body, and mono are a type scale: titles, reading text, and code. Shipped versus not-yet is a CVA variant in a `.variants.ts` file so the Badge is one primitive with two looks, not a pile of Tailwind strings in every page.
