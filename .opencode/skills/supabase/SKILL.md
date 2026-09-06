---
name: supabase
description: Local-first Supabase. Use when starting supabase, writing migrations, RLS, gen types, edge functions, config.toml, seed.sql, or wiring an app to Postgres Auth Storage. Default is the local Docker stack. Production, supabase link, db push, functions deploy, and hosted keys are opt-in only. Triggers on supabase start, supabase status, sb_publishable, service_role, PostgREST, Mailpit.
metadata:
  version: "1.0.0"
---

# Supabase (local first)

Progressive rules for the Supabase CLI and client. Discover the repo first. Read only rule files that match the task. Production is never the default target.

## Discover first

1. Find the CLI. Prefer a project script, then `npx supabase`, then `which supabase`. If missing, tell the user how to install and stop.
2. Confirm Docker or a Docker-compatible runtime is running.
3. Find `supabase/config.toml`, `supabase/migrations/`, env files, and package scripts.
4. Copy existing env names, generated-types path, and start scripts.

If there is no `supabase/` folder, propose `supabase init` and wait.

## Stack caveats

**Assumes:** Supabase CLI plus Docker (or OrbStack, Colima, Podman, Rancher). The app talks to `http://127.0.0.1:54321` unless the human opted into a hosted project.

**Prefer:** `supabase start`, `status`, and `db reset`. Migrations in `supabase/migrations`. Types from `--local`. Publishable or `anon` in the client. RLS in SQL. Mailpit for auth mail. Project scripts when they exist.

**Careful:** An npm install has no global `supabase` binary. Use `npx supabase`. Config edits need `stop` then `start`. Device emulators cannot use `127.0.0.1`. New keys are `sb_publishable_` and `sb_secret_`. Older projects still use `anon` and `service_role`.

**Do not introduce:** Prisma or Drizzle as the schema source if migrations already exist. Do not `link`, `db push`, `db pull`, `functions deploy`, `gen types --project-id`, or put a hosted URL in `.env.local` unless the human named production. Do not put a secret or `service_role` key in a browser, mobile app, or committed file.

A project-local **supabase** skill owns paved paths when present. The **principals** rule `principle-rls-is-the-security-boundary` owns the access-control judgment when installed.

## When to apply

Init, start, status, reset. Migrations, seed, RLS. Client env and keys. Edge functions locally. Typegen. Auth redirects and Mailpit. Any request that names a hosted project, `link`, `db push`, or deploy.

## Priority bands

| Pri | Category | Impact | Prefix |
| ----- | ---------- | -------- | -------- |
| 1 | Production gate | CRITICAL | `prod-` |
| 2 | Discover | CRITICAL | `disc-` |
| 3 | Pitfalls | CRITICAL | `pitfall-` |
| 4 | Keys | CRITICAL | `key-` |
| 5 | Local stack | HIGH | `local-` |
| 6 | Migrations | HIGH | `mig-` |
| 7 | RLS | HIGH | `rls-` |
| 8 | Config and env | MEDIUM-HIGH | `cfg-` |
| 9 | Auth | MEDIUM | `auth-` |
| 10 | Functions | MEDIUM | `fn-` |
| 11 | Types | MEDIUM | `types-` |
| 12 | Storage | MEDIUM | `store-` |
| 13 | Test and CI | LOW | `test-` `ci-` |

## Quick reference

**prod-:** `prod-never-default` local is the target · `prod-opt-in-explicit` human names production · `prod-link-gated` no silent link · `prod-db-push-gated` no silent push · `prod-db-pull-gated` no silent pull · `prod-functions-deploy-gated` no silent deploy · `prod-no-db-reset-remote` never reset hosted · `prod-dump-linked-gated` dump --linked is prod data · `prod-branching-gated` preview branches are hosted

**disc-:** `disc-cli-on-path` install then stop · `disc-docker-running` Docker first · `disc-project-layout` find supabase/ · `disc-match-conventions` copy env names

**pitfall-:** `pitfall-mixed-local-prod-keys` never mix hosts · `pitfall-service-role-browser` secret never in client · `pitfall-dashboard-as-source` migrations own schema · `pitfall-link-then-reset` reset is local · `pitfall-skip-rls-because-local` RLS still on

**key-:** `key-publishable-client` sb_publishable or anon · `key-secret-server-only` sb_secret or service_role · `key-gitignore-secrets` never commit · `key-env-from-status` fill from status

**local-:** `local-init-first` supabase init · `local-start` supabase start · `local-status-keys` status for URLs · `local-stop-keep-data` stop keeps DB · `local-reset-migrations` reset applies SQL · `local-studio` :54323 not cloud · `local-no-link-required` unlink is fine · `local-device-loopback` 10.0.2.2 or LAN

**mig-:** `mig-new-file` migration new · `mig-source-of-truth` files not dashboard · `mig-no-edit-applied` new file instead · `mig-db-diff-local` diff -f · `mig-seed-sql` seed on reset · `mig-list-before-push` list first · `mig-declarative-optional` schemas/ only if present

**rls-:** `rls-enable-every-table` enable RLS · `rls-policy-in-migration` policies in SQL · `rls-test-as-user` not service_role · `rls-is-boundary` UI hide is not authz

**cfg-:** `cfg-config-toml` restart after edit · `cfg-env-split` local vs hosted files · `cfg-env-substitution` env() for secrets · `cfg-gitignore-temp` .temp stays out

**auth-:** `auth-site-url-local` localhost redirects · `auth-mailpit` :54324 not SMTP · `auth-confirmations-off-local` skip confirm locally

**fn-:** `fn-new-file` functions new · `fn-serve-local` functions serve · `fn-no-deploy-default` deploy is opt-in · `fn-secrets-local-env` functions/.env

**types-:** `types-gen-local` --local · `types-commit-check` commit the file

**store-:** `store-buckets-config` buckets in config.toml · `store-local-files` local disk

**test, ci:** `test-against-local` real local stack · `ci-db-start` db start in CI

## How to use

1. Discover the repo (CLI, Docker, `supabase/`, env names).
2. Pick 1 to N rule ids (higher priority first). Production-gate rules first if the task names a hosted project.
3. `Read` only `rules/<id>.md` (relative to this skill directory).
4. Do **not** bulk-read `rules/` or load all of `AGENTS.md` unless stuck or asked.
5. Reviewing or refactoring: walk categories top-down until covered.

## Full reference

Upstream CLI notes: `AGENTS.md` (reference only; prefer `rules/` + this router).
