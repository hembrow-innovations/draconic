---
id: guide-triage-labels
created_at: 2026-07-26
updated_at: 2026-07-26
area: engineering
domain: system
title: "Triage Labels"
description: "Canonical triage roles mapped to issue note status and tags frontmatter (no GitHub labels)."
status: active
tags: [guide, planning]
---

# Triage Labels

Skills speak in five canonical state roles plus category roles. There is no
GitHub label store — issues are notes in `docs/planning/issues/` (see
[[issue-tracker]]). Roles map to **frontmatter**:

- **State roles → the `status` field** (exactly one) plus a matching state `tag`.
- **Category & overflow → the `tags` array.**

## Category roles (frontmatter `tags`, apply exactly one)

- **`bug`** — something is broken
- **`enhancement`** — new feature or improvement

Optional `issue-type`: `bug` | `feature-request` | `observation`.

## State roles

Issue `status` enum: `open | reviewing | promoted | closed | wontfix`.

- **`needs-triage`** — `status: open` + tag `needs-triage`
- **`needs-info`** — `status: open` + tag `needs-info`
- **`ready-for-agent`** — `status: open` + tag `ready-for-agent` (+ `## Agent Brief`)
- **`ready-for-human`** — `status: open` + tag `ready-for-human`
- **`wontfix`** — `status: wontfix` (then move to `issues/closed/`)

Accepted multi-unit work → `status: promoted`. Done → `status: closed` + move to
`closed/`. Every triaged issue carries one category tag and one state tag.
