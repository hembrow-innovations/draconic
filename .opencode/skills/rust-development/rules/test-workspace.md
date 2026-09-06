---
title: Workspace oracle stays --workspace
impact: CRITICAL
impactDescription: --lib --bins drops conformance and fakes green
tags: [test, workspace, oracle]
---

## Workspace oracle stays --workspace

The workspace bar is `cargo test --workspace`. Focused crate runs are `cargo test -p draconic-frontend`. Do not "fix" timeouts by narrowing to `--lib --bins`.

**Incorrect:**

```bash
cargo test --workspace --offline --lib --bins
```

**Correct:**

```bash
cargo test -p draconic-check --lib
cargo test --workspace
```

**Notes.** ADR-0012: default CHECK budget is 10 minutes. `exit=timeout` with `match=yes` is a budget miss, not a hang. `[profile.test]` uses `debug = "line-tables-only"` and `incremental = false` so `target/` stays under 10GB. Keep that profile. See `avoid-narrow-oracle`.
