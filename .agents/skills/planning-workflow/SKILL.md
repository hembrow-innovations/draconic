---
name: planning-workflow
description: The tracker workflow for docs/planning/ (vault, replacing GitHub Issues and .scratch/). Drives work through issue → plan/tasks → execute → review → new issues. Use when triaging work, planning a feature, picking up a task, or closing the loop after implementation.
---

# Planning workflow

`docs/planning/` **is** the issue tracker. There is no GitHub Issues and no
`.scratch/` tracker. All non-Roadmap work moves through one loop; each artefact
is a markdown note (templates under `.agents/skills/docs/templates/`):

```
issue ──▶ (ready, or fan out to plan/tasks) ──▶ execute ──▶ review ──▶ new issues ──▶ …
```

**Language completeness** still uses `ROADMAP.md` + **draconic-loop** — do not
file ECMA-262 Loop atoms only as vault issues unless promoting out of the Roadmap.

**Notes = independent work-units.** A single-unit issue *is* the executable —
triage it `ready-for-agent` (+ `## Agent Brief`) and run it directly, no task
note. Spin out task children only when one issue fans into ≥2 parallel units.

## 1. Issue — `docs/planning/issues/issues-<N>-<slug>.md`

Capture the problem/opportunity. Template: `issue.md`.
`status: open | reviewing | promoted | closed | wontfix`. Triage facets on
`tags` and optional `issue-type` / `severity`.

## 2. Make it executable

- **One unit** — `status: open` + tag `ready-for-agent` + `## Agent Brief`
  (scope / verification / acceptance). No task note. Claim → `reviewing`; done →
  `closed` + move to `issues/closed/`.
- **Multiple units** — `status: promoted`, then tasks (`tasks-<N>-…`) and optional
  plan (`plans-<N>-…`).

## 3. Execute

Pick lowest-numbered ready unit. Claim before work. On finish flip status **and**
move terminal notes out of the active folder.

```sh
grep -l "^status: \(closed\|wontfix\)" docs/planning/issues/*.md
grep -l "^status: complete" docs/planning/tasks/*.md
# both should print nothing after filing
```

## 4. Review

Verify acceptance; file follow-ups as **new issues**.

## 5. New issues — close the loop

Gaps from review become new notes in `planning/issues/`.

## Filing done & rejected work

- Issue `closed` / `wontfix` → `docs/planning/issues/closed/`
- Plan complete / abandoned → `docs/planning/plans/completed/`
- Task complete / abandoned → `docs/planning/tasks/completed/`

Use link-safe moves when `notesmd-cli` is available; otherwise `git mv` and fix
broken wikilinks.

## Allocating ids

```sh
node scripts/planning-next-id.mjs
node scripts/planning-check-ids.mjs
```

Never eyeball the highest number in the active folder.

## Ops guide

`docs/reference/guides/issue-tracker.md` · `docs/reference/guides/triage-labels.md`
