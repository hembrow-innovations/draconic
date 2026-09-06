---
title: Keep target under 10GB
impact: CRITICAL
impactDescription: Fat test artifacts blow disk and the workspace oracle
tags: [size, cargo, target]
---

## Keep target under 10GB

Keep `target/` under 10GB. Workspace tests use lean debug info and no incremental.

**Incorrect:** restoring fat `debug = 2` or `incremental = true` under `[profile.test]` to "see better panics."

**Correct:** leave workspace test profile as line tables only:

```toml
[profile.test]
debug = "line-tables-only"
incremental = false
```

**Notes.** Fat debug test binaries spent minutes on process startup and blew the 600s workspace oracle. Panic locations still work with line tables. Do not "fix" timeouts by dropping conformance; see `test-workspace-oracle`.
