---
title: Use env() for secrets in config.toml
impact: HIGH
impactDescription: OAuth client secrets committed in toml are leaks
tags: [cfg, secrets]
---

## Use env() for secrets in config.toml

Provider secrets in `config.toml` support `env(NAME)`. The values live in the environment, not in git.

**Incorrect:** `secret = "sk-live-..."` under `[auth.external.github]`.

**Correct:** `secret = "env(SUPABASE_AUTH_EXTERNAL_GITHUB_SECRET)"` and export that variable locally. Do not commit the value.

Notes: The same rule applies to Twilio, SMTP passwords, and hook secrets.
