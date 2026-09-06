---
title: Match dynamic copy with regex
impact: HIGH
impactDescription: time-of-day and names make exact text flake
tags: [sel, regex]
---

## Match dynamic copy with regex

`text` and `id` are regex. Greetings, counts, and display names change.

```yaml
- assertVisible: "Good (Morning|Afternoon|Evening|day), Dev.*"
```

**Incorrect:** `assertVisible: "Good morning, Dev"` in a suite that runs at any hour.

**Correct:** A regex that covers the real variants, or an `id:` on the heading so the copy can move.

Notes: Escape `$` and `[` in the pattern. Prefer `id:` when you own the component.
