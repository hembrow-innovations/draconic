---
description: Walk the signed-in web app route by route, screenshot each page, collect console/network/a11y errors, verify findings, and file them as issue notes. Use when the user wants a UI sweep, a "find everything broken" pass, or invokes /ui-audit.
agent: build
---

Sweep the web app for UI defects and file the real ones as issues.

For interactive polish with the user (annotate + implement), use **/ui-craft** / the **ui-craft** agent instead — this command is findings-only.

`$ARGUMENTS` = optional route scope (e.g. `/finances`, `finances vehicles`). Empty = every authed route.

Uses the **playwright-cli** skill for the walk. Load it if unsure of a command.

## Preflight (orchestrator, once)

Check before starting anything — the dev server and DB are usually already up:

```bash
curl -s -o /dev/null -w "%{http_code}" http://localhost:3000/   # want 200
curl -s -o /dev/null -w "%{http_code}" http://localhost:54321/rest/v1/
```

- Supabase down → `pnpm db:start`. Web down → `pnpm dev:web` (background) → :3000.
- **Only seed if there is no data.** Check first (`select count(*) from transactions`) — a reseed can clobber the user's local state. Seeded dev user: `dev@life-engine.dev` / `password123`. Empty DB = every page is an empty state = worthless audit.
- Sign in once and save the session so auditors don't each re-login:

```bash
playwright-cli -s=ui-audit open http://localhost:3000/auth/signin
playwright-cli -s=ui-audit snapshot          # get refs
playwright-cli -s=ui-audit fill <email-ref> "dev@life-engine.dev"
playwright-cli -s=ui-audit fill <pw-ref> "password123" --submit
playwright-cli -s=ui-audit state-save .audit/auth.json
```

`mkdir -p .audit` first (gitignored — screenshots + auth state).

## Route list

Routes are TanStack file-based under `apps/web/src/routes/`. Derive, don't guess:

- Glob `apps/web/src/routes/**/*.tsx`; drop `_`-prefixed layout files, `-`-prefixed folders, `api/`, and non-page routes (`robots[.]txt`, `sitemap.xml`, `auth/callback`, `auth/handoff`).
- `index.tsx` → the folder's `/` path.
- `$param` routes: **don't synthesise IDs.** Visit the parent list, click the first row, audit where you land. If nothing links there, that IS a finding (see below).
- Honour `$ARGUMENTS` — audit only routes under those prefixes.

Split into ~5 groups by domain. One auditor per group, max 5 concurrent.

## Phase 1 — audit (subagents, read-only)

Each auditor gets its own session (`-s=audit-<group>`) so they don't clobber each other:

```bash
playwright-cli -s=audit-<group> open http://localhost:3000
playwright-cli -s=audit-<group> state-load .audit/auth.json
playwright-cli -s=audit-<group> goto http://localhost:3000<route>
playwright-cli -s=audit-<group> screenshot --filename=.audit/<slug>.png
playwright-cli -s=audit-<group> console      # JS errors, React warnings
playwright-cli -s=audit-<group> requests     # NOT `network` — then `request <n>` to inspect one
playwright-cli -s=audit-<group> find "text"  # grep the snapshot instead of dumping it all
```

Then **read the screenshot** and look for:

- Layout broken: overlap, overflow, clipped text, unreadable contrast.
- Dead UI: buttons/filters that render but do nothing (click, re-snapshot, confirm).
- **Unreachable pages**: a detail route exists but nothing links to it. Check for a real `<a>` — `main a[href*="..."]` returning zero while rows are present is the tell.
- Missing states: no loading skeleton, no empty state, and especially **a blank page where an error state belongs** (a failed query that renders nothing).
- Wrong locale: `$`/`gal`/`mi`/`lb` where AUD/L/km/kg belong (AGENTS.md — Australian defaults).
- Inconsistent chrome: a page not using the shared `ui-components-web` components.

**Scope selectors to `main`.** The sidebar renders links off-canvas; an unscoped `a[href*=...]` matches an invisible nav item and hangs the click forever ("element is outside of the viewport", retried until timeout).

**Auditors are read-only**: no fixes, no vault writes, no commits, no new files in the repo. Each returns JSON:

```json
[{ "route": "/vehicles?tab=fuel", "severity": "high", "title": "...",
   "observed": "...", "expected": "...", "evidence": ".audit/x.png | console | requests" }]
```

## Phase 1b — a11y (orchestrator, once — not per auditor)

`@axe-core/playwright` is already a devDependency in `apps/web`. Write **one** throwaway spec covering all in-scope routes (log in as the dev user, `new AxeBuilder({ page }).withTags(["wcag2a","wcag2aa"]).analyze()`), print the full violation nodes (`failureSummary`, `html`, `target`), run it:

```bash
cd apps/web && pnpm exec playwright test e2e/tests/zz-audit.spec.ts \
  --project=chromium --workers=1 --retries=0 --reporter=line
```

**Delete the spec afterwards.** Report serious/critical only, and map each node back to its source component. Group by *root cause*, not by rule: one `color-contrast` rule can be three unrelated bugs (a bad token, a bad pairing, an ancestor `opacity`) — that's three issues, not one.

## Phase 2 — verify (fresh agent, never the finder)

For each finding: reproduce it, and **name the file that causes it**. Default to rejecting. Kill anything that doesn't reproduce, duplicates another finding, or already has an open ticket (search `.heio/planning/tickets/` first).

The verify pass is the whole point — it is what stops a plausible-looking screenshot from becoming a wrong ticket. Non-negotiable checks:

- **Check the seed's dates before calling stale data a bug.** Seed transactions may be months old, so "This month: $0" is often *correct*.
- **Check the DB and schema before blaming the UI.** A wrong value on screen may be faithfully rendering wrong data (e.g. accounts showing USD because the seed wrote USD, while the column default is AUD).
- **A 403 from PostgREST is not automatically RLS.** Postgres `42501` / "permission denied for table" means the table-level `GRANT` is missing — the request never reached RLS. Check `has_table_privilege('authenticated', ...)` against a working peer table.
- **A 406 with `PGRST116` may be already handled** (`.single()` on zero rows, error swallowed). Confirm it actually breaks something before filing it.

## Phase 3 — file (orchestrator only)

Only the orchestrator writes tickets.

Allocate each ID **before** writing with the helper — it reads the one global high-water mark across tickets, tasks, slices, locations, and rounds, including archive, so ids never collide. Re-run it immediately before each write; it reports the next free id without reserving it.

```
node skills/management/scripts/planning-next-id.mjs
```

Never pick the id by eye from `ls .heio/planning/tickets/`; closed notes move to archive and their ids look free.

Load **obsidian-axi**. Write `.heio/planning/tickets/ticket-<NN>-<slug>.md` with **management** `templates/ticket.md`. `status: open`. `ticket_type: bug`.

Body: `## Signal`, `## Fit`, `## Notes` plus observed route and file:line.

Then clean up: delete the throwaway spec, `rm -rf .audit apps/web/test-results`, `playwright-cli close-all`, add the changelog line, commit with explicit paths, and report one line per issue.

## Rules

- Findings, not fixes. This command never edits app code.
- Verified only. An unverified finding costs the user more than it saves.
- Say what you skipped. A route that wouldn't load, or a param route with no seed row, gets reported — silent gaps read as "all clear".
