# Required frontmatter fields

Every stack note includes these fields. Kind-specific fields follow on the kind template.

```yaml
id: "<filename stem or folder name>"
title: "<same string as the h1>"
kind: intent | roadmap | sprint | slice | ticket
tags: []
created_at: "<ISO-8601>"
updated_at: "<ISO-8601>"
```

`id` matches the file stem, except:

- **intent**: `intent` (file `intent.md`)
- **roadmap**: `roadmap` (file `roadmap.md`)
- **sprint**: the sprint folder name (`m3`, `launch`)
- **slice**: the slice folder name (`s-login`)
- **ticket**: `ticket-01-slug`

Slice `tasks.md` and `oracles.md` have no frontmatter. They belong to the slice folder.

## Optional fields

Add only the ones the kind uses.

```yaml
description: "one sentence"
status: "see rules/layout.md"
labels: feature
sprint: "m3"
slice: "s-login"
references: ["ticket-01-slug"]
```
