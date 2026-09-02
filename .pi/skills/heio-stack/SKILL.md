---
name: heio-stack
description: Heio-stack operating system under `.heio/planning` (including task-pool), `.heio/tickets`, and `.heio/archive`. Intent, locations, sprints, slices, task-pool, tickets, oracles. Use when finding or writing those notes, classifying inbound work as TASK/TICKET/ESCALATE/VERIFY, or when another skill needs the stack.
---

# Heio-stack

`.heio/planning/`, `.heio/tickets/`, and `.heio/archive/` are the working tree. Templates live in `templates/`. Extra grain lives in `rules/`. This file is enough to run the OS.

Search `.heio/` first, including `archive/`. Copy the matching template. Place it per the tree.

## Working tree

```text
.heio/
├─ tickets/
│  └─ ticket-01-<slug>.md
├─ archive/
│  ├─ index.md
│  ├─ tickets/
│  └─ planning/
│     ├─ task-pool/
│     ├─ locations/
│     └─ sprints/
└─ planning/
   ├─ intent.md
   ├─ roadmap.md
   ├─ task-pool/
   │  └─ <task>.md
   ├─ locations/
   │  └─ <slug>.md
   └─ sprints/
      └─ <sprint-id>/
         ├─ shape.md
         └─ slices/
            └─ s-<slug>.md
```

Create a folder when the first file needs it.

## Artifacts

- **intent**: why the project exists, success, non-goals. `.heio/planning/intent.md`
- **roadmap**: locations as destinations, not a schedule. `.heio/planning/roadmap.md`
- **location**: extra depth for one roadmap bullet. `.heio/planning/locations/<slug>.md`
- **sprint**: grouping of slices. `shape.md` is the grouping. `.heio/planning/sprints/<sprint-id>/shape.md`
- **slice**: one markdown file. Status, oracle checklist, durable links to task-pool ids. `.heio/planning/sprints/<sprint-id>/slices/s-<slug>.md`
- **task**: one markdown file in the task pool. `.heio/planning/task-pool/<task>.md`
- **ticket**: inbound product signal. `.heio/tickets/ticket-<NN>-<slug>.md`
- **archive**: completed work, mirroring the live tree. `.heio/archive/index.md` plus `archive/planning/task-pool/`, `archive/planning/sprints/`, `archive/planning/locations/`, `archive/tickets/`

## Status

- **intent**: `active` / `superseded`
- **roadmap**: `draft` / `active`
- **location**: `active` / `done`
- **sprint**: `shaping` / `active` / `review` / `closed`
- **slice**: `shaping` / `frozen` / `active` / `met` / `abandoned`
- **ticket**: `open` / `parked` / `promoted` / `dropped` / `closed`
- **task**: `draft` → `ready` → `claimed` → `implemented` → `completed`

A slice is `met` when every linked task-pool id is `completed` and the oracles hold. Links are never dropped.

## Workflow

Work hangs off sprint grouping → slice → task-pool files.

- **shape.md** lists which slices are in this grouping.
- A slice is one file. Name `blocked-by` when it waits on another slice. Unblocked slices may run in parallel.
- Oracles live on the slice file (`CHECK` / `EXPECT` / `EVIDENCE` / `ABANDON`).
- Each unit of work is a task-pool file. The slice keeps durable `[[id]]` links to those ids.
- Inbound product work is a ticket. Triage it into a task-pool file (and link it), park it, or escalate it to the map.
- Completed work moves to archive. Completed task files move to `.heio/archive/planning/task-pool/`. Closed sprints, done locations, and closed tickets move under the matching archive path. Add a one-liner to `archive/index.md`.

## Loop

Every output is one of four. End with the block.

- **TASK**: it fits an unblocked active slice. Do it now, or add a task-pool file and link it from the slice.
- **TICKET**: it belongs to the project, not this slice. File it under `.heio/tickets/` and leave the slice alone.
- **ESCALATE**: the change would rewrite a location destination. Stop and bump it to the map.
- **VERIFY**: check the oracles on the slice file until they hold, or `ABANDON:` with a named home.

```
VERDICT: TASK | TICKET | ESCALATE | VERIFY
EVIDENCE: <one line>
```

## Naming

- **sprint-id**: short folder name (`week-1`, `auth-working`). The id is the folder.
- **location slug**: `locations/<slug>.md`. Lowercase kebab-case.
- **slice file**: `s-<slug>.md`. Lowercase kebab-case.
- **ticket**: `ticket-<NN>-<slug>.md`. `<NN>` is the next unused integer, zero-padded to two digits.
- **task**: the file stem is the id.
- **links**: `[[id]]`. The `id` is the stem or folder name.

`slug` is lowercase kebab-case. Keep it short.

## Templates

Copy the matching file from `templates/`. Shared fields: `templates/required-fields.md`.

- **intent**: `templates/intent.md` → `.heio/planning/intent.md`
- **roadmap**: `templates/roadmap.md` → `.heio/planning/roadmap.md`
- **location**: `templates/location.md` → `.heio/planning/locations/<slug>.md`
- **sprint**: `templates/sprint-shape.md` → `.heio/planning/sprints/<id>/shape.md`
- **slice**: `templates/slice.md` → `.heio/planning/sprints/<id>/slices/s-<slug>.md`
- **ticket**: `templates/ticket.md` → `.heio/tickets/ticket-<NN>-<slug>.md`
- **task**: `templates/pool-task.md` → `.heio/planning/task-pool/<task>.md`
- **archive index**: `templates/archive-index.md` → `.heio/archive/index.md`

## When to apply

- Finding or writing intent, roadmap, location, sprint shape, a slice, a task-pool file, a ticket, or an archive entry
- Classifying inbound work as TASK, TICKET, ESCALATE, or VERIFY
- Closing a slice or a sprint, or moving finished work to archive
