---
title: Fill a form from a fresh snapshot
impact: MEDIUM
tags: [example, form]
---

## Fill a form from a fresh snapshot

```bash
playwright-cli open http://localhost:ORIGIN
playwright-cli snapshot
playwright-cli fill e1 "dev@example.com"
playwright-cli fill e2 "password"
playwright-cli click e3
playwright-cli snapshot
playwright-cli close
```

`e1` / `e2` / `e3` are placeholders.

**Incorrect:** Copying those refs into a real session, or using another app's seed user.

**Correct:** Snapshot, then fill the refs it printed. Use the project's seed when one exists (`disc-match-conventions`). `--submit` on the last field if Enter submits.

Notes: RN fields are `rn-textinput`.
