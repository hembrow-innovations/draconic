# Required frontmatter fields

Every docs note includes these fields. Kind-specific fields follow on the kind template.

```yaml
id: "<filename stem, or adr-<N> for ADRs>"
title: "<same string as the h1>"
kind: overview | architecture | system-design | adr | rfc | purpose | spec | api | schema | non-functional | standard | style | guide
domain: "<subject domain, not a folder>"
area: "<area slug>"
tags: []
created_at: "<ISO-8601 or YYYY-MM-DD>"
updated_at: "<ISO-8601 or YYYY-MM-DD>"
```

`id` is stable after create. Do not change it when you rename the title.

## Optional fields

```yaml
description: "one sentence"
status: "see the kind template"
source: "path/to/on-disk-spec.yaml"
```
