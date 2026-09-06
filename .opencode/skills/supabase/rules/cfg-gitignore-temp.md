---
title: Keep CLI temp files out of git
impact: MEDIUM
impactDescription: .temp can hold a project-ref and local junk
tags: [cfg]
---

## Keep CLI temp files out of git

The CLI writes work files under `supabase/.temp`. A linked project-ref lives there. Those files are not source.

**Incorrect:** Committing `supabase/.temp/` or removing the ignore so status "shows up in git".

**Correct:** Leave `supabase/.gitignore` ignoring `.temp` and similar. Do not add those paths.

Notes: `config.toml`, `migrations/`, `functions/`, and `seed.sql` are source.
