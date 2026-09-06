---
id: "task-645-ui-audit"
title: "Audit public-site UI and file tickets"
kind: task
status: ready
mode: afk
blocked_by:
  - task-638-home-odm-layout
  - task-639-docs-article-odm
  - task-640-docs-nav-groups
  - task-641-hub-cards
  - task-642-mobile-odm-wrap
sprint: "website-odm-match"
slice: "slice-644-ui-audit"
area: public-site
tags: [website, public-site]
created_at: "2026-09-07T19:00:00Z"
updated_at: "2026-09-07T19:30:00Z"
---

# Audit public-site UI and file tickets

## Blocked by

[[task-638-home-odm-layout]] [[task-639-docs-article-odm]] [[task-640-docs-nav-groups]] [[task-641-hub-cards]] [[task-642-mobile-odm-wrap]]: walk the ODM chrome, not the old top bar.

## Done

Pages walked. One ticket per finding. One closeout ticket listing walks and finding ids.

## Context

Walk `/`, `/learn`, one Learn chapter, `/reference`, one Reference page, and a small viewport. Compare to public-site purpose and contract, and to the ODM two-column shell (skip, sticky side nav, main, kicker, cards, wrap). Load **to-tickets** and **management**. Allocate ids with the planning-next-id script. File `status: open` tickets, `ticket_type: observation` or `bug`, each one problem. Then file a closeout ticket that wikilinks [[slice-644-ui-audit]] and lists pages plus finding ids. If nothing is wrong, the closeout ticket still exists and says zero findings.

Do not implement. Do not invent product rules. If a gap has no promise, file the ticket and leave it open. Do not drop search, theme toggle, or fences. Do not copy ODM copy. Do not serve `docs/`.

## Verify

`rg -l "slice-644-ui-audit" .heio/planning/tickets .heio/archive/planning/tickets` prints a ticket path.

scope: `.heio/planning/tickets/` only, plus the slice EVIDENCE line on [[slice-644-ui-audit]]

## Links

[[slice-644-ui-audit]] [[ticket-643-ui-audit]]

## Agent Brief

**Category:** enhancement
**Summary:** Walk the public site after the ODM restyle and file tickets. Do not restyle in this sitting.

**Drain:** `/afk-task`. If any `blocked_by` id is not `completed`, stop. Do not claim.

**Skills:** load **to-tickets**, **management**, **docs**, **spec**. Do not load frontend implement skills to restyle. No changelog. Do not use `ui-components-web`.

**Vault pack:** no `pnpm vault:pack`. Must-read: public-site purpose, contract, test; [[slice-644-ui-audit]]; [[ticket-643-ui-audit]].

**TDD:** not a code sitting. File tickets only.

**Intent (required when product behaviour changes):**
- **No product behaviour change** (audit and tickets only)
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- If a finding would change behaviour, file a ticket. Do not edit promises here.

**Current behavior:**
ODM restyle slices may be met. Gaps in spacing, contrast, type, leftover HTML, nav, or small-viewport use may remain untracked.

**Desired behavior:**
Each independent UI problem is its own ticket. A closeout ticket lists `/`, `/learn`, one Learn chapter, `/reference`, one Reference page, small viewport, and every finding id. Zero findings still get the closeout ticket. No code or CSS changes in this sitting except the slice EVIDENCE line.

**Key interfaces:**
- Management ticket template and planning-next-id script from the repo root
- Public-site purpose and contract

**Acceptance criteria:**
- [ ] Named routes walked
- [ ] One ticket per finding, or zero finding tickets
- [ ] Closeout ticket links [[slice-644-ui-audit]]
- [ ] No website source changes except slice EVIDENCE
- [ ] `rg -l "slice-644-ui-audit" .heio/planning/tickets` matches

**Out of scope:**
- Implementing fixes; playground; vault-as-site; Start replacement; copying ODM copy; language ROADMAP

## Gauntlet

- **Round 1**: `rg -l "slice-644-ui-audit" .heio/planning/tickets .heio/archive/planning/tickets` — win. Prints a `ticket-` path.
