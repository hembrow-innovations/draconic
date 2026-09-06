# Supabase (local first). Reference only

**Prefer `rules/` + `SKILL.md` stack notes.** This file is optional bulk context for humans or when the agent is stuck. Do not load it by default on skill activation.

## Official docs

- Local overview: https://supabase.com/docs/guides/local-development
- Install and run the CLI: https://supabase.com/docs/guides/local-development/cli/getting-started
- CLI config (`config.toml`): https://supabase.com/docs/guides/local-development/cli/config
- Managing environments: https://supabase.com/docs/guides/deployment/managing-environments
- API keys: https://supabase.com/docs/guides/api/api-keys
- Generating types: https://supabase.com/docs/guides/api/rest/generating-types
- Edge Functions quickstart: https://supabase.com/docs/guides/functions/quickstart
- CLI reference: https://supabase.com/docs/reference/cli

## Discover the repo

Do not assume a global `supabase` binary, a linked project, or Next.js env names.

1. Project script, `npx supabase --help`, or `which supabase`. Stop if missing.
2. Docker (or OrbStack, Colima, Podman, Rancher) is running.
3. Search `supabase/config.toml`, `supabase/migrations/`, `supabase/functions/`, `supabase/seed.sql`.
4. Read package scripts, justfile, README, CI, and leftover project `supabase` skills.
5. Copy env names and the generated-types path from what you find.

If nothing exists, propose `supabase init` and wait.

## Decision tree

```
What failed or what are you adding?
  CLI missing                         → disc-cli-on-path; stop
  Docker not running                  → disc-docker-running; stop
  No supabase/ folder                 → propose init; wait
  Start / status / Studio / Mailpit   → local-*
  Schema or seed change               → mig-* then local-reset-migrations
  App cannot reach API                → key-env-from-status, pitfall-mixed-local-prod-keys
  Device or emulator                  → local-device-loopback
  Client auth or RLS                  → key-publishable-client, rls-*
  Secret key in a bundle              → key-secret-server-only, pitfall-service-role-browser
  Edge function locally               → fn-serve-local
  Types out of date                   → types-gen-local
  Human named production              → prod-opt-in-explicit, then the matching gated command
  link / db push / db pull / deploy   → matching prod-*-gated rule; do not run unless asked
```

## Canonical local commands

Use the project's script when it exists. Translate `supabase` to `npx supabase` when the CLI is a devDependency.

```bash
supabase --version
supabase init
supabase start
supabase status
supabase stop
supabase db reset
supabase migration new add_widgets
supabase db diff -f add_widgets
supabase gen types typescript --local
supabase functions new hello-world
supabase functions serve hello-world
```

Default local ports (override only if `config.toml` already did):

| Service | URL |
|---|---|
| API / PostgREST / Auth / Storage / Functions | http://127.0.0.1:54321 |
| Postgres | postgresql://postgres:postgres@127.0.0.1:54322/postgres |
| Studio | http://127.0.0.1:54323 |
| Mailpit | http://127.0.0.1:54324 |

`supabase start` prints publishable and secret keys (or legacy `anon` / `service_role`). Put those in the local env file the repo already uses. Never commit the secret.

## Production is opt-in

These commands talk to a hosted project. Do not run them unless the human named production, staging, or the project-ref.

```bash
supabase login
supabase link --project-ref <project-id>
supabase db push
supabase db pull
supabase db dump --linked
supabase functions deploy
supabase gen types typescript --project-id <project-id>
supabase branches
```

`supabase db reset` recreates the **local** database. It is not a hosted reset. Never invent a remote reset.

CI deploys belong in the repo's existing GitHub Action, not an ad-hoc laptop push.

## Keys

| Kind | Format | Where |
|---|---|---|
| Publishable | `sb_publishable_...` | Browser, mobile, committed examples |
| Secret | `sb_secret_...` | Server, Edge Function, CI secret store |
| `anon` | long JWT | Legacy publishable |
| `service_role` | long JWT | Legacy secret |

Secret and `service_role` bypass RLS. They are never a client env var.

## Maintain this pack

```bash
node skills/data/supabase/scripts/validate.mjs
```

The script fails if `SKILL.md` grows past 150 lines, if the index cites a missing rule, or if a rule file is not in the index.

## Sibling skills

- `principle-rls-is-the-security-boundary` owns the access-control judgment
- `principle-boundary-discipline` owns where env and keys are parsed
- `tdd` owns what a good test is
- A project `supabase` skill owns paved paths, seed users, and env names when installed
