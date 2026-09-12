---
id: "task-885-copy-announcement"
title: "Announce fence copy and distinguish controls"
kind: task
status: ready
mode: afk
blocked_by: []
sprint: "website-chrome-polish"
slice: "slice-884-copy-announcement"
area: public-site
tags: [website, public-site]
created_at: "2026-09-12T17:20:00Z"
updated_at: "2026-09-12T17:20:00Z"
---

# Announce fence copy and distinguish controls

## Blocked by

None.

## Done

Fence Copy controls on Learn and Reference are distinguishable, a successful copy is announced, and Copied does not stick.

## Context

Copy-the-text already works. Every control uses the same accessible name Copy. Copied never clears. No live announcement. Home samples have no Copy. Assert `public-site.fences:copy-announce`, keep `public-site.fences:copy`.

## Verify

`pnpm --dir website exec vitest run code-fence-copy` prints `Test Files  1 passed`. `pnpm --dir website exec tsc --noEmit` exits 0.

scope: `docs/specs/draconic/public-site/`, `website/src/components/CodeFence/`, `website/src/features/docs/DocsShell/DocsShell.tsx`, `website/src/tests/code-fence-copy.test.ts`

## Links

[[slice-884-copy-announcement]] [[ticket-858-copy-announcement]]

## Agent Brief

**Category:** bug
**Summary:** Make Learn and Reference fence Copy controls distinguishable, announce a successful copy, and clear the Copied state so it does not stick.

**Drain:** `/afk-task`. Empty `blocked_by`. Claim and implement.

**Skills:** load **frontend-development**, **tdd**, **gauntlet-loop**, **docs**, **spec**, **website**. Public site is TanStack Start under `website/`. Do not use `ui-components-web`. No changelog.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-884-copy-announcement]]; [[ticket-858-copy-announcement]].

**TDD:** assert a contract promise for copy announcement and distinct names, point `code-fence-copy` tests at it so they fail while every control is named Copy, there is no live announcement, and Copied never clears, then implement. Then typecheck.

**Intent (required when product behaviour changes):**
- Promise ids: add and lock `public-site.fences:copy-announce`; keep `public-site.fences:copy`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: edit/assert promise → test → code. Do not put Copy on home samples.

**Current behavior:**
A visitor can copy fence text on Learn and Reference. Every Copy control uses the same accessible name Copy. After a successful copy the visible name can become Copied and never returns. Nothing is announced on a live region. Home samples have no Copy control.

**Desired behavior:**
Copy-the-text still works on Learn and Reference fences. Each Copy control on a page has a distinct accessible name, not a shared Copy. A successful copy is announced to assistive tech. Copied does not remain for the rest of the article. Home samples stay without Copy. Teaching copy is unchanged.

**Key interfaces:**
- CodeFence copy control
- DocsShell fence rendering
- Existing `code-fence-copy` lock for copy-the-text

**Acceptance criteria:**
- [ ] Contract lists `public-site.fences:copy-announce` with a test pointer
- [ ] Tests fail if every fence control is only named Copy, if there is no live announcement, or if Copied never clears
- [ ] Named vitest file passes and typecheck exits 0
- [ ] `public-site.fences:copy` still holds

**Out of scope:**
- Home sample Copy buttons; fence compile rules; playground; vault-as-site

**Explain this part:**
This sitting is allowed to assert the missing announcement promise. It must not weaken copy-the-text and must not add Copy to home samples.
