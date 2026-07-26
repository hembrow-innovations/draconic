---
id: guide-issue-tracker
created_at: 2026-07-26
updated_at: 2026-07-26
area: engineering
domain: system
title: "Issue Tracker"
description: "Vault tracker ops: kinds, status enums, id allocation, filing rules, and wayfinding for docs/planning/."
status: active
tags: [guide, planning]
---

# Issue Tracker

There is **no GitHub Issues**. Issues, plans, tasks and PRDs live as markdown under
`docs/planning/` — the project's second brain and single source of truth for
non-Roadmap work. (Language completeness still uses [[ROADMAP]] + the test suite
and the **draconic-loop** skill.)

Drive notes with plain file edits (or `notesmd-cli` if installed). Prefer
`[[wikilinks]]` between notes.

## Kinds — when to use which

- **issue** — *what's wrong or wanted*, and — once triaged `ready-for-agent` with
  a `## Agent Brief` — *the thing an agent executes*. Default entry point **and**
  the execution atom for single-unit work: no task note.
- **task** — *a child work-unit, only when an issue fans out*. Create a task
  **only** when one issue splits into ≥2 independent/parallel units, or when
  several issues collapse into one unit. 1 issue → 1 unit = no task.
- **plan** — *the strategy across multiple tasks*. Create one **only** for
  genuinely multi-task work.
- **idea** (tag, not a kind) — file as a low-priority issue with
  `tags: [..., idea]`.

**Rule: number of notes = number of independent work-units.**

## Allocating `<N>` — never eyeball it

One **global** id sequence: `issues-N`, `tasks-N` and `plans-N` all draw from the
same counter:

```sh
node scripts/planning-next-id.mjs   # scans every folder incl. closed/ + completed/
```

Re-run immediately before writing the file. `node scripts/planning-check-ids.mjs`
fails on any duplicate id among live notes.

## Conventions

- **Issues** — `docs/planning/issues/issues-<N>-<slug>.md`, template
  `.agents/skills/docs/templates/issue.md`.
  `status: open | reviewing | promoted | closed | wontfix`.
  `ready-for-agent` = `status: open` + tag `ready-for-agent` + `## Agent Brief`.
  Claim → `status: reviewing`; done → `status: closed` **and** move to
  `issues/closed/`.
- **Plans / tasks** — `docs/planning/{plans,tasks}/`. Tasks:
  `status: hold | ready | active | complete | wontfix`.
- **Filing done work** — issues → `issues/closed/`; plans → `plans/completed/`;
  tasks → `tasks/completed/`. Don't delete. `wontfix` uses the same folders with
  `status: wontfix`.
- **Comments / history** — append under `## Comments` (or `## Log`). Link with
  `[[wikilinks]]`.
- **The board** — `docs/planning/index.md`.

## When a skill says "publish to the issue tracker"

Create a note under `docs/planning/` with the matching template. Allocate id via
`node scripts/planning-next-id.mjs`.

## When a skill says "fetch the relevant ticket"

Read the file at `docs/planning/issues/issues-<N>-…` (or the path the user gave).

## Wayfinding operations

Used by the **wayfinder** skill. The **map** is a plan note; its **tickets** are
issue notes.

- **Map** — `docs/planning/plans/plans-<N>-<slug>.md`, `tags: [..., wayfinder-map]`.
- **Ticket** — `docs/planning/issues/issues-<N>-<slug>.md`,
  `tags: [..., wayfinder, wayfinder-<type>]` (`research` | `prototype` |
  `grilling` | `task`). Body holds the question; `[[wikilink]]` the map.
- **Blocking** — `## Blocked by` with `[[wikilinks]]`. Unblocked when every listed
  ticket is `status: closed`.
- **Frontier** — open, unblocked, unclaimed; lowest id first.
- **Claim** — set `status: reviewing` before work.
- **Resolve** — `## Answer`, `status: closed`, move to `issues/closed/`, append
  gist + `[[wikilink]]` to the map's Decisions so far.
