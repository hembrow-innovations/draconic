---
title: Find the supabase folder first
impact: CRITICAL
impactDescription: commands are project-scoped to supabase/config.toml
tags: [disc]
---

## Find the supabase folder first

Most CLI commands expect to run in a directory that already has `supabase/config.toml`.

**Incorrect:** Running `supabase start` from a random subdirectory, or creating a second `supabase/` next to an existing one.

**Correct:** Search for `supabase/config.toml`, `supabase/migrations/`, and package scripts. Run the CLI from the directory that owns that folder.

Notes: Monorepos often keep one `supabase/` at the repo root. Do not add another.
