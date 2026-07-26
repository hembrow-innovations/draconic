# Issue Tracker

There is **no GitHub Issues**. Issues, plans, tasks and PRDs live as markdown under
`docs/planning/` — see `docs/reference/guides/issue-tracker.md`.

## Kinds

- **issue** — `docs/planning/issues/issues-<N>-<slug>.md`
- **task** — only when an issue fans into ≥2 units: `docs/planning/tasks/tasks-<N>-<slug>.md`
- **plan** — multi-task strategy: `docs/planning/plans/plans-<N>-<slug>.md`

## Allocating `<N>`

```sh
node scripts/planning-next-id.mjs
```

One global sequence for issues, tasks, and plans. Never eyeball active-folder max.

## Status

Issues: `open | reviewing | promoted | closed | wontfix`.
`ready-for-agent` = `status: open` + tag `ready-for-agent` + `## Agent Brief`.
Terminal states move notes to `issues/closed/` (or `*/completed/` for plans/tasks).

## When a skill says "publish to the issue tracker"

Create a note under `docs/planning/` with the matching template
(`.agents/skills/docs/templates/`). Allocate id first.

## When a skill says "fetch the relevant ticket"

Read `docs/planning/issues/issues-<N>-…` (or the path given).

## Wayfinding operations

- **Map** — `docs/planning/plans/plans-<N>-<slug>.md`, tag `wayfinder-map`
- **Ticket** — issue note with tags `wayfinder`, `wayfinder-<type>`
- **Blocking** — `## Blocked by` with `[[wikilinks]]`
- **Claim** — `status: reviewing`
- **Resolve** — `## Answer`, `status: closed`, move to `closed/`, update map
