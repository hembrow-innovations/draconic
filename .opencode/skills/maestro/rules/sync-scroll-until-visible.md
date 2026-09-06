---
title: Scroll until the target is visible
impact: HIGH
impactDescription: below-fold copy is not a failed selector
tags: [sync, scroll]
---

## Scroll until the target is visible

Lists and long forms keep targets off screen. A miss is often a scroll, not a bad id.

```yaml
- scrollUntilVisible:
    element:
      id: legal_accept
```

**Incorrect:** Failing the flow because "I agree" was below the fold, then switching to `point:`.

**Correct:** `scrollUntilVisible` on the `id:` or text. Dismiss covering sheets (Getting Started, coach marks) first.

Notes: If a nested scroller ignores the default scroll, inspect the tree before inventing a custom swipe.
