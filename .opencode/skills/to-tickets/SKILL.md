---
name: to-tickets
description: File inbound product signals as management tickets. Use when converting a complaint, request, or idea into a ticket, promoting a ticket into slices and tasks, or breaking a spec or PRD into tracker notes.
---

# To tickets

File inbound work as tickets using **management** conventions. This skill does not invent layout, status, or frontmatter keys.

Load **management** before any write under `.heio/`. Load **docs** for glossary terms and ADRs. Load **to-slices** and **to-tasks** when promoting into executable work.

A ticket is a signal. A task is the executable. Do not design the solution inside the ticket.

## Process

### 1. Gather context

Work from conversation context. If the user passes a ticket id, read that note under `.heio/planning/tickets/` and `.heio/archive/planning/tickets/`.

### 2. Explore (optional)

Use glossary vocabulary discovered under `docs/`. Respect ADRs in the area.

### 3. Choose outcome

- Fits an unblocked active slice → **TASK**. Status `promoted`. Load **to-tasks** (and **to-slices** if the slice does not exist).
- Fits the project, not this slice → **TICKET**. Status `parked` or leave `open` if triage is still running.
- Fans into independent problems → one ticket per problem, then promote each that is ready.
- Would rewrite a location destination → **ESCALATE**. Do not write a ticket as if it were a map change.

### 4. Show, then publish

Present the list. For each note: title, kind (ticket, slice, or task), `blocked_by`, AFK or HITL.

Copy `templates/ticket.md` from **management**. **management** owns `<NN>`, placement, and frontmatter.

- Path: `.heio/planning/tickets/ticket-<NN>-<slug>.md`
- Status: `open` until triaged. Then `parked`, `promoted`, `dropped`, or `closed`
- `ticket_type`: `bug` | `feature-request` | `observation` when known
- `blocked_by` lists ids this ticket waits on
- Publish blockers first so wikilinks are real

Do not tag a ticket as the AFK unit. Drain hunts tasks with `status: ready`.

## Body extras

Keep the management template headings. Add these sections when they carry information the template does not.

```markdown
## Parent

Wikilink to the parent slice, ticket, or spec.

## What to build

End-to-end behavior. Not layer-by-layer implementation.

## Blocked by

- [[ticket-01-slug]] or [[slice-01-slug]] or none
```

## Loop

End with `VERDICT: TASK | TICKET | ESCALATE | VERIFY`.
