---
id: "task-623-reference-hub-pages"
title: "Add Reference hub and pages"
kind: task
status: completed
mode: afk
blocked_by: ["task-619-docs-shell"]
sprint: "website-redesign"
slice: "slice-604-reference-hub-pages"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T12:00:00Z"
---

# Add Reference hub and pages

## Blocked by

[[task-619-docs-shell]]: Reference uses the same handbook chrome as Learn, with its own aside.

## Done

Reference hub and working pages (CLI, types, Dual-world rules, host I/O, packages) are walkable app routes, not vault `api-cli` notes.

## Context

Reference is pages kept open while writing a Program. It is not a generated API dump and not the Learn path. Hub is `website/reference.md`. Do not publish `docs/` or `api-cli` as the public Reference.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until [[task-627-fence-static-deploy]]. Do not delete `website/generate.drac` in this sitting.

## Verify

Routes exist for the listed Reference markdown files. Aside lists CLI, types, Dual-world rules, host I/O, packages. Typecheck holds.

scope: Reference hub and pages under `website/src/`; sources `website/reference.md`, `cli.md`, `types.md`, `dual-world-rules.md`, `reference-host-io.md`, `reference-packages.md`

## Links

[[slice-604-reference-hub-pages]] [[task-619-docs-shell]]

## Agent Brief

**Category:** enhancement
**Summary:** Add Reference hub and working pages in the docs shell.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.ia:reference-walkable`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
Docs shell exists. Reference still lives as `website/*.md` / old `.html`. Vault API notes are not the public site.

**Desired behavior:**
Walkable Reference: hub plus CLI, types, Dual-world rules, host I/O, packages. Sources are the listed `website/*.md` files. Keep teaching copy. Fix `.html` links to app routes. Badge from frontmatter. Not `docs/` `api-cli`.

**Key interfaces:**
- Reference hub and child file routes
- Reference aside (not the Learn aside)
- Loader + renderer + docs shell + Badge
- Files: `website/reference.md`, `cli.md`, `types.md`, `dual-world-rules.md`, `reference-host-io.md`, `reference-packages.md`

**Acceptance criteria:**
- [x] Hub plus five working pages are app routes
- [x] Aside matches `website/reference.md`
- [x] Not vault API notes; not Learn chapters
- [x] Teaching copy kept; `.html` hrefs rewritten if needed
- [x] If [[task-619-docs-shell]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Learn chapter routes
- Search ([[task-624-search]])
- Serving `docs/api-cli` or other vault notes

**Explain this part:**
Reference is working pages: CLI, types, Dual-world rules, host I/O, packages. You keep them open while writing a Program. They are not `api-cli` vault notes and not a generated API dump. Learn teaches a path; Reference looks up a surface.

## Gauntlet

- **Round 1**: `pnpm --dir website test -- reference-hub-pages` — win. `Test Files  1 passed`. Typecheck holds. Diff keeps `public-site.ia:reference-walkable`; no purpose out-of-scope.
