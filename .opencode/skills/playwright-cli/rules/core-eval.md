---
title: Read attributes with eval
impact: HIGH
tags: [core, eval]
---

## Read attributes with eval

Snapshots often omit `id`, class, `data-*`, and computed style. `eval` reads the DOM node behind a ref.

```bash
playwright-cli eval "document.title"
playwright-cli eval "el => el.textContent" e5
playwright-cli eval "el => el.getAttribute('data-testid')" e5
playwright-cli eval "el => el.getAttribute('aria-label')" e5
playwright-cli eval "el => getComputedStyle(el).display" e5
```

**Incorrect:** Guessing a `testID` because the snapshot did not print it.

**Correct:** `eval` the ref, then act with that attribute or with `generate-locator` (`sel-role-testid`).

Notes: `run-code` is for multi-step page scripts (`inspect-run-code`).
