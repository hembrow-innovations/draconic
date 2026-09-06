---
id: "task-609-scaffold-start-app"
title: "Scaffold the TanStack Start app"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-redesign"
slice: "slice-590-scaffold-start-app"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-06T12:50:00Z"
---

# Scaffold the TanStack Start app

## Blocked by

None.

## Done

`website/package.json` exists and the TanStack Start app typechecks with `pnpm --dir website exec tsc --noEmit`.

## Context

Public Learn and Reference is a language site. ADR-0013 locked presentation as a TanStack Start app rooted at `website/`. Today the HTML still comes from `website/generate.drac`. This sitting only scaffolds the app so later sittings can add routes, tokens, and pages.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

`pnpm --dir website exec tsc --noEmit` succeeds. `website/package.json` is present. Teaching markdown is unchanged.

scope: `website/package.json`, `website/tsconfig.json`, `website/vite.config.ts` or Start config, `website/src/` entry files

## Links

[[slice-590-scaffold-start-app]]

## Agent Brief

**Category:** scaffolding
**Summary:** Create the TanStack Start app under `website/` so the public site can leave `generate.drac` as the HTML renderer.

**Intent (required when product behaviour changes):**
- Promise ids: none (scaffold; no new product promise)
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`website/` is markdown plus a Draconic Program that dumps HTML. There is no `website/package.json` and no Start app.

**Desired behavior:**
A pnpm TypeScript TanStack Start app typechecks. Entry files exist under `website/src/`. The old generator still exists. Teaching copy is not rewritten.

**Key interfaces:**
- `website/package.json` scripts and Start dependencies
- `website/tsconfig.json`
- Vite or Start config at `website/`
- Start client/SSR entry files under `website/src/`
- Load **frontend-development** (`tool-pnpm-typescript`, `start-execution-model`)

**Acceptance criteria:**
- [x] `website/package.json` exists and uses pnpm plus TypeScript only
- [x] TanStack Start plus Router is the app, not Next.js, VitePress, Starlight, or mdBook
- [x] `pnpm --dir website exec tsc --noEmit` succeeds
- [x] Teaching markdown under `website/*.md` is not rewritten
- [x] `website/generate.drac` is not deleted

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Tokens, chrome, home copy, markdown loader, Learn or Reference routes
- Nested folder policy beyond what Start needs to boot (that is [[task-610-nested-src-root-route]])

**Explain this part:**
TanStack Start is the app framework: file routes, Vite, and TypeScript. It replaces `generate.drac` as the HTML renderer, not as the teaching source. pnpm is the only package manager. This sitting proves the app boots and typechecks so later sittings can hang routes and components on a real tree.

## Gauntlet

- **Round 1**: `pnpm --dir website exec tsc --noEmit` — win. Exit 0, no errors. No new promise ids.
