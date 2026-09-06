---
title: Snapshot again after navigation
impact: CRITICAL
impactDescription: stale e-refs click the wrong node
tags: [snap, refs]
---

## Snapshot again after navigation

Refs like `e15` belong to the last snapshot. Expo Router, tabs, and modals replace the tree.

```bash
playwright-cli click e15
playwright-cli snapshot
```

**Incorrect:** Reusing `e15` after `goto`, `click` that changed route, or opening a modal.

**Correct:** Snapshot after every navigation or tree change. Use only refs from that snapshot.

Notes: Heavy trees: `--depth` first, then snapshot a subtree (`snap-target-refs`).
