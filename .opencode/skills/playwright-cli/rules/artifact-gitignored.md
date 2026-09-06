---
title: Screenshots go in gitignored tmp
impact: HIGH
tags: [artifact, tmp]
---

## Screenshots go in gitignored tmp

```bash
playwright-cli screenshot --filename=.tmp/page.png
playwright-cli screenshot e5 --filename=.tmp/submit.png
playwright-cli pdf --filename=.tmp/page.pdf
playwright-cli screenshot --hires
```

**Incorrect:** Writing `screenshot.png` in the repo root or under `docs/`.

**Correct:** Use the project's gitignored scratch dir. If none exists, `.tmp/` is the default to create and ignore. Snapshots the CLI writes under `.playwright-cli/` stay there.

Notes: Discover the dir first (`disc-match-conventions`).
