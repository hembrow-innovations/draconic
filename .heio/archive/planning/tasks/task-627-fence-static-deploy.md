---
id: "task-627-fence-static-deploy"
title: "Fence compile and static deploy"
kind: task
status: completed
mode: afk
blocked_by: ["task-618-markdown-render", "task-616-home-features"]
sprint: "website-redesign"
slice: "slice-608-fence-static-deploy"
tags: []
created_at: "2026-09-06T18:00:00Z"
updated_at: "2026-09-07T02:00:00Z"
---

# Fence compile and static deploy

## Blocked by

[[task-618-markdown-render]]: fences live in rendered markdown; compile checks need that subset.
[[task-616-home-features]]: static publish must include the language homepage, not Learn copied to index.

## Done

Shipped `drac` fences compile, not-yet pages have no fences, and the Start static build replaces `generate.drac` as the publisher. Website pipeline tests still run.

## Context

Shipped pages may include copy-paste fences and those samples must build. Not-yet pages stay prose. Start static build is the publisher. GitHub Pages remains the host. `scripts/generate-website.sh` may wrap `pnpm` build. Retarget `tests/integration/tests/website_pipeline.rs` if HTML paths change. Do not drop tests. Do not restore a playground. Workflows may stay `.disabled`; do not fight that. Do not touch ROADMAP language rows.

Stack for this sprint: pnpm, TypeScript only, TanStack Start plus Router, nested `website/src/`, Tailwind v4 `@theme`, CVA in `.variants.ts`, no Next.js, no named `max-w-sm`, no hardcoded hex in components, file budget 1000 lines. Markdown sources stay `website/*.md` until this sitting retires `generate.drac` as renderer.

## Verify

Start static build emits the public site. `tests/integration/tests/website_pipeline.rs` still passes (retargeted if paths changed). Shipped fences compile. Not-yet pages contain no fences. `generate.drac` is no longer the publisher.

scope: website build, `scripts/generate-website.sh`, `tests/integration/tests/website_pipeline.rs`, `website/generate.drac` retirement as renderer

## Links

[[slice-608-fence-static-deploy]] [[task-618-markdown-render]] [[task-616-home-features]]

## Agent Brief

**Category:** enhancement
**Summary:** Enforce fence rules and replace `generate.drac` with the Start static build as publisher.

If any `blocked_by` task is not `completed`, stop. Drain uses `blocked_by`.

**Intent (required when product behaviour changes):**
- Promise ids: `public-site.fences:shipped-must-build`, `public-site.fences:forbid-not-yet-fences`
- Purpose: [[docs/specs/draconic/public-site/purpose]]
- Contract: [[docs/specs/draconic/public-site/contract]]
- Contract-first: do not invent playground or vault-as-site

**Current behavior:**
`scripts/generate-website.sh` compiles `website/generate.drac`, copies `website/*.html`, and clones `learn.html` to `index.html`. `website_pipeline.rs` compiles that generator, asserts nav and markdown subset, and `draconic build`s shipped `drac` fences.

**Desired behavior:**
Start static build publishes the site. Home is the language homepage, not Learn copied to index. Shipped `drac` fences must compile. Not-yet pages must not contain fences. Retarget `tests/integration/tests/website_pipeline.rs` if HTML paths change. Do not drop those tests. `scripts/generate-website.sh` may wrap `pnpm` build. GitHub Pages still hosts. Workflows may stay `.disabled`; do not fight that. Retire `website/generate.drac` as renderer. Do not restore a playground.

**Key interfaces:**
- Start static build output (GitHub Pages dist)
- `scripts/generate-website.sh` as a wrapper if kept
- `tests/integration/tests/website_pipeline.rs` (keep; retarget paths)
- Fence extraction from shipped markdown only
- ADR-0013 presentation replacement; ADR-0010 split and fence rules kept

**Acceptance criteria:**
- [x] Shipped `drac` fences compile
- [x] Not-yet pages contain no fences
- [x] Start static build is the publisher; `generate.drac` is retired as renderer
- [x] `website_pipeline.rs` still runs (paths retargeted if needed); tests are not dropped
- [x] No playground restored; `docs/` not published as the site
- [x] ROADMAP language rows are not edited
- [x] If [[task-618-markdown-render]] or [[task-616-home-features]] is not `completed`, stop

**Out of scope:**
- Playground; in-page runners; Next.js; VitePress; Starlight; mdBook; publishing `docs/`; rewriting CONTEXT terms; `cargo test --workspace`; other tasks’ files except declared scope
- Fighting `.disabled` GitHub workflows
- ROADMAP language feature rows
- Rewriting teaching markdown except fence presence required by the promises

**Explain this part:**
Shipped `drac` fences must compile so copy-paste samples are real Programs. Not-yet pages must not contain fences, because those chapters are not claimed as runnable. The Start static build replaces `generate.drac` as publisher. GitHub Pages stays the host. Do not drop the pipeline tests; retarget them if HTML paths change.

## Gauntlet

- **Round 1:** `pnpm --dir website build && cargo test -p draconic-integration-tests --test website_pipeline` — win. Critic: `test result: ok.` 12 passed. Promises `public-site.fences:shipped-must-build` and `public-site.fences:forbid-not-yet-fences` still locked by named pipeline tests. Start static build is publisher; `generate.drac` retired; index is homepage not Learn copy. No playground.
