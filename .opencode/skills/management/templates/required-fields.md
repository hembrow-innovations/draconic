# Required frontmatter fields

Every stack note includes these fields. Kind-specific fields follow on the kind template.

```yaml
id: "<filename stem or folder name>"
title: "<same string as the h1>"
kind: intent | roadmap | location | sprint | slice | ticket | task | round
tags: []
created_at: "<ISO-8601>"
updated_at: "<ISO-8601>"
```

`id` matches the file stem, except:

- **intent**: `intent` (file `intent.md`)
- **roadmap**: `roadmap` (file `roadmap.md`)
- **location**: `location-<NN>-<slug>`
- **sprint**: the sprint folder name (`week-1`, `auth-working`)
- **slice**: `slice-<NN>-<slug>`
- **ticket**: `ticket-<NN>-<slug>`
- **task**: `task-<NN>-<slug>`
- **round**: `rounds-<NN>-<slug>`

`archive/index.md` has no frontmatter.

Keys use `_`, never `-`.

## Optional fields

Add only the ones the kind uses.

```yaml
description: "one sentence"
status: "see management SKILL.md"
labels: feature
sprint: "week-1"
slice: "slice-01-slug"
references: ["ticket-01-slug"]
mode: afk
blocked_by: []
sitting_kind: planning
ticket_type: bug
```
