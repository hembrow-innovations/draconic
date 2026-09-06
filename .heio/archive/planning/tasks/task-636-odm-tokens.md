---
id: "task-636-odm-tokens"
title: "Retoken to ODM dark navy"
kind: task
status: completed
mode: afk
blocked_by: []
sprint: "website-odm-match"
slice: "slice-629-odm-tokens"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T18:00:00Z"
updated_at: "2026-09-07T21:15:00Z"
---

# Retoken to ODM dark navy

## Blocked by

None.

## Done

Dark semantic tokens match ODM navy surfaces; accent-2 mint exists; product TS still has no hex.

## Context

Current dark canvas is `#07111c` with accent `#4c9ae8`. Desired dark values from the ODM public site stylesheet: canvas `#0c0f14`, elevated `#141a22`, soft `#1a222d`, code `#0a0d12`, line `#2a3544`, ink `#e8eef6`, muted `#a3b4c6`, accent `#5b9fd4`, accent-2 `#7dd3a7`, link `#7eb8e8`. Keep a derived light theme and the theme toggle. Map these onto Tailwind `@theme` names in `website/src/styles/theme.css`. Add `--color-accent-2` if missing. Extend `website/src/tests/semantic-tokens.test.ts` so dark canvas/ink/accent/line and accent-2 are asserted. Hex belongs in CSS and tests, never product TS/TSX.

## Verify

`pnpm --dir website exec vitest run semantic-tokens` passes. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `website/src/styles/theme.css`, `website/src/lib/theme/`, `website/src/tests/semantic-tokens.test.ts`

## Links

[[slice-629-odm-tokens]]

## Agent Brief

**Category:** enhancement
**Summary:** Point semantic tokens at ODM dark navy and mint without hex in components.

**Drain:** `/afk-task`. If any `blocked_by` id is not `completed`, stop. Do not claim.

**Skills:** load **frontend-development** (`tokens-theme-css`), **tdd**, **gauntlet-loop**, **docs**, **spec**. This public site is TanStack Start under `website/` with Tailwind v4 `@theme` and CVA in `.variants.ts`. Do not use `ui-components-web`, `ui-infra-web`, or React Native UI packages. No changelog; skip that `/afk-task` step.

**Vault pack:** this repo has no `pnpm vault:pack`. Must-read: docs/specs/draconic/public-site/purpose.md, contract.md, test.md; docs/adr/0013-public-site-tanstack-start.md; docs/guides/guides-public-docs.md; [[slice-629-odm-tokens]].

**TDD:** extend the semantic-tokens vitest first so dark canvas, ink, accent, line, and accent-2 fail. Then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: appearance supports `public-site.chrome:odm-shell`; keep token no-hex rule
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
Light-first cool blue tokens. Dark is a navy variant of TypeScript.org blue.

**Desired behavior:**
`html.dark` canvas, ink, accent, muted, and line match ODM. Mint `--color-accent-2` exists. Light theme remains a readable pair. Theme toggle stays. No hex in product TS.

**Key interfaces:**
- Tailwind `@theme` custom properties on the website stylesheet
- Semantic token vitest that forbids hex in product TS/TSX

**Acceptance criteria:**
- [x] Dark canvas `#0c0f14`, ink `#e8eef6`, accent `#5b9fd4`, line `#2a3544`
- [x] Accent-2 `#7dd3a7` exists as a token
- [x] Product TS/TSX still has no hex
- [x] `pnpm --dir website exec vitest run semantic-tokens` → Test Files  1 passed
- [x] `pnpm --dir website exec tsc --noEmit` exits 0

**Out of scope:**
- Site shell layout; home bands; docs article; dropping theme toggle; Start replacement; publishing `docs/`

**Explain this part:**
ODM looks like cool dark navy with blue accent and mint current-page. Tokens move first so later slices can use `bg-canvas` and `text-accent-2`.

## Gauntlet

- **Round 1**: `pnpm --dir website exec vitest run semantic-tokens` — win. `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.
