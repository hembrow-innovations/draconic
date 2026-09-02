# Required frontmatter fields

Every stack note includes these fields. Kind-specific fields follow on the kind template.

```yaml
id: "<filename stem or folder name>"
title: "<same string as the h1>"
kind: intent | roadmap | location | sprint | slice | ticket | task
tags: []
created_at: "<ISO-8601>"
updated_at: "<ISO-8601>"
```

`id` matches the file stem, except:

- **intent**: `intent` (file `intent.md`)
- **roadmap**: `roadmap` (file `roadmap.md`)
- **location**: the slug (`auth-working`)
- **sprint**: the sprint folder name (`week-1`, `auth-working`)
- **slice**: the file stem (`s-login`)
- **ticket**: `ticket-01-slug`
- **task**: the file stem (`task-id`)

`archive/index.md` has no frontmatter.

## Optional fields

Add only the ones the kind uses.

```yaml
description: "one sentence"
status: "see rules/layout.md"
labels: feature
sprint: "week-1"
slice: "s-login"
references: ["ticket-01-slug"]
```
