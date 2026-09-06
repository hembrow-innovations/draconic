---
title: Declare local buckets in config.toml
impact: MEDIUM
impactDescription: dashboard-only buckets vanish on reset
tags: [store, cfg]
---

## Declare local buckets in config.toml

Local Storage buckets belong in `config.toml` so reset recreates them.

**Incorrect:** Creating a bucket only in Studio, then losing it on `db reset`.

**Correct:** Add the bucket under the storage section of `config.toml` (the file already documents the table). Restart the stack. Put object policies in a migration.

Notes: `cfg-config-toml`.
