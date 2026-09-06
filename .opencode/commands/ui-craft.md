---
description: Collaborative UI/UX polish on a route or screen — open app, see with user, implement token-true fixes. Use for /ui-craft, make this nicer, design feedback. Not bulk defect sweeps (/ui-audit).
agent: ui-craft
---

Polish UI/UX collaboratively with the user.

`$ARGUMENTS` = target surface/route/screen (e.g. `/tasks`, `finances accounts`, `mobile tasks list`). Empty = ask once, default web home after sign-in.

## Run

1. Load skills as needed: **playwright-cli** (web/desktop), **frontend-development** (always before implement), **maestro** (mobile).
2. Preflight env for that surface; sign in if needed.
3. Open the target; snapshot + screenshot.
4. Critique against frontend-development + design tokens; if feedback is visual, offer `playwright-cli show --annotate`.
5. Agree a small change set, implement in pure/client UI packages only, re-see.
6. Stop when user accepts. Bulk multi-route defect hunt → tell them to use `/ui-audit` instead.

Do not mint a pile of planning issues. One real bug can be noted or filed if asked.
