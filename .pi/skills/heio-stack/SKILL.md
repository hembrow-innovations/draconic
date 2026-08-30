---
name: heio-stack
description: Heio-stack operating loop under `.heio/planning` and `.heio/tickets`. Intent, roadmap, sprints, slices, tickets, tasks, and oracles. Use when finding or writing those notes, triaging inbound work as TASK/TICKET/ESCALATE/VERIFY, or when another skill needs the stack.
---

# Heio-stack. Local operating loop

`.heio/planning/` and `.heio/tickets/` are the working tree for this stack. Git ignores `.heio/`. `docs/` is the committed source of truth. Load **docs** for that vault. Load **domain-modeling** when a term or ADR belongs there.

This tree is the tracker. If `AGENTS.md` or `WORKSPACE.md` already names a tracker, that file wins. Do not start a second tree.

Per-rule detail lives in `rules/`. Copy-ready skeletons live in `templates/`.

## Before writing (always)

1. Search `.heio/` first. Update in place over near-dupes.
2. Pick the kind. Copy the matching file from `templates/`.
3. Place and name per `layout.md`.
4. Leave reserved root files alone.

Full steps: `rules/write-before.md`.

## When to apply

- Finding or writing intent, roadmap, sprint shape, slice spec, slice oracles, slice tasks, or a ticket
- Triaging inbound work as TASK, TICKET, ESCALATE, or VERIFY
- Deciding whether a note belongs in `.heio/` or `docs/`
- Closing a slice or a sprint

## Prefer / careful / do not

### Prefer

- **write-before** before any new note
- **layout** for path and naming
- **layers** for intent vs shape vs work
- **loop** for every output
- **template-kinds** plus `templates/` for the skeleton

### Careful

- **change.** Everything new is a ticket first. Never a task.
- **oracles.** `--reverify` is a different pass. `EXPECT:` freezes with the slice.
- **tasks.** Only the active slice has them. TDD is the build grain; oracles prove the slice.

### Do not

- Commit `.heio/`
- Treat `.heio/tickets/` as a second brain
- Edit intent or roadmap from a builder pass
- Write tasks before the slice is `frozen`
- Invent a folder under `.heio/` that `layout.md` does not name

## Rule categories by priority

- **1 CRITICAL** - Before writing (`write-`)
- **2 CRITICAL** - Folder layout (`layout`)
- **3 CRITICAL** - Layers (`layers`)
- **4 CRITICAL** - Loop (`loop`)
- **5 HIGH** - Templates (`template-`)
- **6 HIGH** - Tickets, sprints, slices, oracles, tasks, change

## Quick reference

- `write-before` - Search first, pick kind, place, leave reserved files alone
- `layout` - Tree, naming, status, links
- `layers` - Intent sticky, shape semi-sticky, work fluid
- `loop` - TASK / TICKET / ESCALATE / VERIFY
- `template-kinds` - Kind to template to destination
- `tickets` - Signal, triage, rot at sprint close
- `sprints` - Roadmap bets, force-function sprints, cut line
- `slices` - Vertical cut, one sitting, freeze then tasks
- `oracles` - CHECK / EXPECT / ABANDON, `--reverify`
- `tasks` - Active slice only, TDD grain
- `change` - Ticket first, bet-changes escalate

## How to use

```text
rules/write-before.md
rules/layout.md
rules/layers.md
rules/loop.md
rules/template-kinds.md
rules/tickets.md
rules/sprints.md
rules/slices.md
rules/oracles.md
rules/tasks.md
rules/change.md
templates/required-fields.md
templates/<kind>.md
```

Read only the rules for the current task. Do not bulk-read `rules/` or every template.

Chart a roadmap or sprint with **heio-wayfinder**. Plan a slice or ticket with **heio-planning**. Execute a frozen slice with **heio-slice**.

Committed truth (ADR, spec, architecture, guide). Load the **docs** skill.
