---
title: Prefer refs over raw coordinates
impact: HIGH
impactDescription: coordinate clicks break on --mobile and layout shift
tags: [core, keyboard, mouse]
---

## Prefer refs over raw coordinates

```bash
playwright-cli press Enter
playwright-cli press ArrowDown
playwright-cli keydown Shift
playwright-cli keyup Shift
playwright-cli mousemove 150 300
playwright-cli mousedown
playwright-cli mouseup
playwright-cli mousewheel 0 100
```

**Incorrect:** `mousemove` + `mousedown` on a labeled control because the snapshot looked busy.

**Correct:** `click` / `fill` / `press` the ref. Raw mouse is for canvas, maps, or a control with no accessible target. `mousewheel` can scroll a `ScrollView` on web.

Notes: There is no software keyboard on this CLI (`rn-textinput`).
