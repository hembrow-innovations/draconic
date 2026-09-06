---
title: Reset still means local after a link
impact: CRITICAL
impactDescription: people avoid reset because they think it wipes prod
tags: [pitfall, local]
---

## Reset still means local after a link

A linked repo still resets **local** Docker volumes. The fear of wiping production makes agents skip the command that applies migrations.

**Incorrect:** Skipping `db reset` after `migration new` because `.temp/project-ref` exists.

**Correct:** Run the local reset. Do not run any remote wipe. See `prod-no-db-reset-remote`.

Notes: If you are unsure which database a command hits, read the rule for that command before running it.
