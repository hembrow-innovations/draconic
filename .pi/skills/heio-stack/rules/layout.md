---
title: Folder layout and naming
impact: CRITICAL
tags: [layout]
---

# Folder layout and naming

`.heio/` sits at the project root.

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

Create a folder when the first file needs it. `planning/locations/` exists only when a location needs a file. `archive/` exists on the first move.

`.heio/planning/task-pool/` is one markdown file per task. Completed task files move to `.heio/archive/planning/task-pool/`.

A slice is one markdown file: status, oracle checklist, durable links to task-pool ids. Sprint `shape.md` stays the grouping. Slice `met` means linked task-pool ids are `completed` and oracles hold. Links are never dropped.

Parked tickets live in `.heio/tickets/`, never in a slice.

Reserved at the root. Leave them in place.

- **TODO.md**: leftover stub. Not the live list.
- **decisions.tsv**: long-run decision trail
- **worktrees/**, **sessions/**, **teams/**: runtime

## Names

- **sprint-id**: short folder name (`week-1`, `auth-working`). The id is the folder. A location name or a timebox.
- **location file**: `locations/<slug>.md`. Lowercase kebab-case. Only when the roadmap bullet needs depth.
- **slice file**: `s-<slug>.md`. Lowercase kebab-case. A file, not a folder.
- **ticket**: `ticket-<NN>-<slug>.md`. `<NN>` is the next unused integer, zero-padded to two digits. Scan `.heio/tickets/`. Start at `01`. Do not reuse a number.
- **task**: the file stem is the id. `.heio/planning/task-pool/<id>.md`

`slug` is lowercase kebab-case. Keep it short.

## Status

- **intent**: `active` / `superseded`
- **roadmap**: `draft` / `active`
- **location**: `active` / `done`
- **sprint**: `shaping` / `active` / `review` / `closed`
- **slice**: `shaping` / `frozen` / `active` / `met` / `abandoned`
- **ticket**: `open` / `parked` / `promoted` / `dropped` / `closed`
- **task**: `draft` / `ready` / `claimed` / `implemented` / `completed`

## Archive

`.heio/archive/` mirrors the live tree.

- **index**: `.heio/archive/index.md`
- **tickets**: `.heio/archive/tickets/`
- **task-pool**: `.heio/archive/planning/task-pool/`
- **sprints**: `.heio/archive/planning/sprints/`
- **locations**: `.heio/archive/planning/locations/`

Closed sprint folders, closed tickets, done location files, and completed task files **move**. Done location bullets leave the live roadmap. Add a one-liner to `archive/index.md` that says what landed.

Live lists come from the live tree. Archive holds completed work.

## Links

Link notes with `[[id]]`. The `id` is the stem or folder name (`ticket-01-login-flash`, `s-login`, `week-1`, `auth-working`, a task-pool id). Carry enough ADRs, specs, and paths on the note that a stranger does not hunt.
