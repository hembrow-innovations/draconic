---
title: Click, fill, type, and related acts
impact: CRITICAL
tags: [core, click, fill]
---

## Click, fill, type, and related acts

Snapshot refs are the default target.

```bash
playwright-cli click e3
playwright-cli dblclick e7
playwright-cli fill e5 "user@example.com" --submit
playwright-cli type "search query"
playwright-cli hover e4
playwright-cli drag e2 e8
playwright-cli drop e4 --path=./image.png
playwright-cli select e9 "option-value"
playwright-cli upload ./document.pdf
playwright-cli check e12
playwright-cli uncheck e12
playwright-cli dialog-accept
playwright-cli dialog-dismiss
playwright-cli resize 1920 1080
```

**Incorrect:** CSS-only clicks on RN-web class hashes, or `fill` on a Pressable.

**Correct:** Snapshot, then `click` / `fill` the ref. `--submit` presses Enter after fill. Dialogs use `dialog-accept` / `dialog-dismiss` before the alert is gone.

Notes: TextInput fill is `rn-textinput`. Missing attributes are `core-eval`.
