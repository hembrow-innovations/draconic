---
name: management
description: Source of truth for `.heio` planning docs. Intent, roadmap, locations, sprints, slices, tasks, rounds, tickets, and archive. Use when finding, writing, claiming, closing, or filing those notes, or when another skill needs paths, status enums, or frontmatter keys.
---

# Management

`.heio/planning/` and `.heio/archive/` are the working tree. This skill owns layout, filenames, status enums, and frontmatter. Other skills in this stack follow it. Copy templates from `templates/`.

Load **obsidian-axi** for file operations (`read`, `write`, `patch`, `mv`). This skill owns meaning. The CLI does not.

There is no GitHub Issues. There is no plan file. `docs/` is the committed source of truth. Load **docs** for vault standards. Do not put planning docs in `docs/`.

Search `.heio/` first, including `archive/`. Copy the matching template. Place it per the tree.

## Working tree

```text
.heio/
├─ archive/
│  ├─ index.md
│  └─ planning/
│     ├─ tasks/
│     ├─ sprints/
│     ├─ locations/
│     ├─ rounds/
│     └─ tickets/
└─ planning/
   ├─ intent.md
   ├─ roadmap.md
   ├─ tasks/
   │  └─ task-<NN>-<slug>.md
   ├─ locations/
   │  └─ location-<NN>-<slug>.md
   ├─ rounds/
   │  └─ rounds-<NN>-<slug>.md
   ├─ tickets/
   │  └─ ticket-<NN>-<slug>.md
   └─ sprints/
      └─ <sprint_name>/
         ├─ shape.md
         └─ slice-<NN>-<slug>.md
```

Create a folder when the first file needs it.

## Artifacts

- **intent**: why the project exists, success, non-goals. `.heio/planning/intent.md`
- **roadmap**: locations as destinations, not a schedule. `.heio/planning/roadmap.md`
- **location**: extra depth for one roadmap bullet. `.heio/planning/locations/location-<NN>-<slug>.md`
- **sprint**: grouping of slices. `shape.md` is the grouping. `.heio/planning/sprints/<sprint_name>/shape.md`
- **slice**: one markdown file. Status, oracle checklist, durable links to task ids. `.heio/planning/sprints/<sprint_name>/slice-<NN>-<slug>.md`
- **task**: one markdown file in the task pool. `.heio/planning/tasks/task-<NN>-<slug>.md`
- **round**: one planning session file. `kind: round`. `sitting_kind: planning` or `wayfinder` (frontmatter, not `mode`). Rounds append in that file. `.heio/planning/rounds/rounds-<NN>-<slug>.md`
- **ticket**: inbound product signal. `.heio/planning/tickets/ticket-<NN>-<slug>.md`
- **archive**: completed work, mirroring the live tree. `.heio/archive/index.md` plus `archive/planning/tasks/`, `archive/planning/sprints/`, `archive/planning/locations/`, `archive/planning/rounds/`, `archive/planning/tickets/`

## Status

- **intent**: `active` / `superseded`
- **roadmap**: `draft` / `active`
- **location**: `active` / `done`
- **sprint**: `shaping` / `active` / `review` / `closed`
- **slice**: `shaping` / `frozen` / `active` / `met` / `abandoned`
- **ticket**: `open` / `parked` / `promoted` / `dropped` / `closed`
- **task**: `draft` → `ready` → `claimed` → `implemented` → `completed`
- **round**: `awaiting-answers` → `ready-to-resume` → `awaiting-confirm` → `published`. `parked` is a side door

A slice is `met` when every linked task id is `completed` and the oracles hold. Links are never dropped.

Task `mode` is `afk` or `hitl`. It is not status.

## Frontmatter

Every note uses `templates/required-fields.md`. Kind-specific fields live on the kind template.

Keys use `_`, never `-`. `blocked_by`, `sitting_kind`, `ticket_type`, `created_at`, `updated_at`.

`blocked_by` is a list of ids this note waits on. Empty means unblocked.

## Workflow

Work hangs off sprint grouping → slice → task files.

- **shape.md** lists which slices are in this grouping.
- A slice is one file next to `shape.md`. Name `blocked_by` when it waits on another slice. Unblocked slices may run in parallel.
- Oracles live on the slice file (`CHECK` / `EXPECT` / `EVIDENCE` / `ABANDON`).
- A planning sitting freezes the in-slices and publishes their task files in one pass. Each task is `ready` with `mode: afk` or `mode: hitl` and `blocked_by`.
- The slice keeps durable `[[id]]` links to those ids. Drain claims unblocked AFK tasks (`status: ready`). HITL waits.
- Inbound product work is a ticket. Triage it into a slice and tasks (and link it), park it, or escalate it to the map.
- Completed work moves to archive. Completed task files move to `.heio/archive/planning/tasks/`. Closed sprints, done locations, published rounds, and closed or dropped tickets move under the matching archive path. Add a one-liner to `archive/index.md`. Use **obsidian-axi** `mv` so links survive.

```
ticket ──▶ (`status: promoted`, to slices and tasks)
slice ──▶ task/s
task ──▶ execute ──▶ review ──▶ close or create a new ticket
```

A task is the executable. A ticket is a signal. Do not execute from a ticket.

## Loop

Every output is one of four. End with the block.

- **TASK**: it fits an unblocked active slice. Do it now, or add a task file and link it from the slice.
- **TICKET**: it belongs to the project, not this slice. File it under `.heio/planning/tickets/` and leave the slice alone.
- **ESCALATE**: the change would rewrite a location destination. Stop and bump it to the map.
- **VERIFY**: check the oracles on the slice file until they hold, or `ABANDON:` with a named home.

```
VERDICT: TASK | TICKET | ESCALATE | VERIFY
EVIDENCE: <one line>
```

A workflow loop must not rewrite intent success or non-goals, must not rewrite location destination sentences, must not write tasks before freeze, and must not patch frozen `EXPECT:`.

## Naming

- **sprint_name**: short folder name (`week-1`, `auth-working`). The id is the folder.
- **NN**: next unused integer in the global sequence, zero-padded to two digits.
- **slug**: lowercase kebab-case. Keep it short.
- **intent**: `intent`
- **roadmap**: `roadmap`
- **location**: `location-<NN>-<slug>`
- **slice**: `slice-<NN>-<slug>`
- **task**: `task-<NN>-<slug>`
- **round**: `rounds-<NN>-<slug>`
- **ticket**: `ticket-<NN>-<slug>`
- **links**: `[[id]]`. The `id` is the file stem or sprint folder name.

## Templates

Copy the matching file from `templates/`. Shared fields: `templates/required-fields.md`.

- **intent**: `templates/intent.md` → `.heio/planning/intent.md`
- **roadmap**: `templates/roadmap.md` → `.heio/planning/roadmap.md`
- **location**: `templates/location.md` → `.heio/planning/locations/location-<NN>-<slug>.md`
- **sprint**: `templates/sprint-shape.md` → `.heio/planning/sprints/<sprint_name>/shape.md`
- **slice**: `templates/slice.md` → `.heio/planning/sprints/<sprint_name>/slice-<NN>-<slug>.md`
- **ticket**: `templates/ticket.md` → `.heio/planning/tickets/ticket-<NN>-<slug>.md`
- **task**: `templates/task.md` → `.heio/planning/tasks/task-<NN>-<slug>.md`
- **round**: `templates/round.md` → `.heio/planning/rounds/rounds-<NN>-<slug>.md`
- **archive index**: `templates/archive-index.md` → `.heio/archive/index.md`

## Allocating ids

One global sequence for tickets, tasks, slices, locations, and rounds.

```
node scripts/planning-next-id.mjs
```

Run it from this skill folder. Never eyeball the highest number in an active folder. Re-run immediately before writing the file.

## Filing

Move a note the moment it leaves the active set. Keep the filename. Use **obsidian-axi** `mv`.

- Ticket `closed` or `dropped` → `.heio/archive/planning/tickets/`
- Task `completed` → `.heio/archive/planning/tasks/`
- Sprint `closed` → `.heio/archive/planning/sprints/<sprint_name>/`
- Location `done` → `.heio/archive/planning/locations/`
- Round `published` (when the sitting is done) → `.heio/archive/planning/rounds/`

Status stays in frontmatter. Add a one-liner to `.heio/archive/index.md`.

## Conventions

- Search `.heio/planning/` and `.heio/archive/` before creating a note.
- Never reach for `gh issue`.
- Commits as work packages: `<type>(<scope>): <description>` — feat | fix | test | refactor | chore
- No Co-Authored-By lines
- Load **to-tickets**, **to-slices**, or **to-tasks** when publishing those kinds. This skill still owns path, id, status, and frontmatter.

## When to apply

- Finding or writing intent, roadmap, location, sprint shape, a slice, a task, a ticket, a round, or an archive entry
- Classifying inbound work as TASK, TICKET, ESCALATE, or VERIFY
- Closing a slice or a sprint, or moving finished work to archive
