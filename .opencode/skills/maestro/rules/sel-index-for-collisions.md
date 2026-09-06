---
title: Disambiguate colliding text with index
impact: HIGH
impactDescription: the first match is often the wrong control
tags: [sel, index]
---

## Disambiguate colliding text with index

Dashboard labels reuse tab names. Alerts reuse "OK". `index` is 0-based.

```yaml
- tapOn:
    text: OK
    index: 1
```

**Incorrect:** `tapOn: Tasks` on a screen that shows Overview "Tasks" and a Tasks tab, then calling the harness broken.

**Correct:** Prefer a `testID`. If you must use text, add `index` after you inspect the hierarchy. Alerts: title is often index 0, the button index 1.

Notes: Hierarchy dumps live with fail artifacts (`artifact-gitignored`).
