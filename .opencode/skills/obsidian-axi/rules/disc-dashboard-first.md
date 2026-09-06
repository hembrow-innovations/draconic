---
title: No-args dashboard first
impact: CRITICAL
impactDescription: skips guessing the wrong vault and the next command
tags: [disc, dashboard]
---

## No-args dashboard first

No arguments is a live dashboard, not help. It shows which vault resolved and what changed recently. AXI content-first.

**Incorrect:** Jumping to `write` or a guessed path before any dashboard or search.

**Correct:**

```sh
obsidian-axi --vault docs
obsidian-axi search <query> --vault docs
obsidian-axi read <path> --vault docs
```

Notes: `--help` at the top level must be the only argument. After the dashboard, follow the printed `help[]` lines (`axi-help-follow`).
