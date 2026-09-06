# Progressive-disclosure skills

How to structure large knowledge packs as agent skills without bloating context. Patterned on `vercel-react-best-practices`.

## Objective

Ship dozens of detailed rules on disk. Put only a small router in context when the skill loads. Deep content is read on demand.

## Context budget

- **Catalog entry**: `name` + short `description` in frontmatter. Always in skill list.
- **Router**: `SKILL.md`. Only after Skill tool load.
- **Atomic rules**: `rules/<id>.md`. Only when agent `Read`s that path.
- **Optional dump**: `AGENTS.md` / upstream full guide. Never auto; optional reference.

Rule of thumb: load cost ≈ `SKILL.md` size, not total skill directory size.

## Directory layout

```
ai/skills/<category>/<skill-name>/
  SKILL.md              # router + stack adaptations + index (required)
  AGENTS.md             # optional: full upstream / long-form reference
  rules/                # atomic, one-topic files
    <prefix>-<slug>.md
```

Optional later: `examples/`, `scripts/` (link from rules; do not inline large demos).

## Layer responsibilities

### Frontmatter (`SKILL.md` YAML)

- `name` — stable id (folder name should match)
- `description` — trigger text for when to load (stack, verbs, domains)
- Keep description dense but short; this is always-visible catalog noise

### `SKILL.md` (router — keep small, ~50–150 lines)

Must include:

1. Stack / project caveats (what applies, what to ignore, what not to introduce)
2. When to apply (task types)
3. Priority bands (critical → low) so the agent picks high-impact first
4. Quick reference index — rule id + one-line summary, grouped by category
5. How to use — explicit paths: `rules/<id>.md`; instruct “read only relevant rules”
6. Links to `AGENTS.md` as upstream-only if needed

Must not include:

- Full incorrect/correct code samples for every rule
- Entire upstream essay
- Every edge case

### `rules/*.md` (atomic detail)

One concern per file. Stable naming:

- Prefix by category: `async-`, `bundle-`, `auth-`, `rls-`, `pattern-`, …
- Slug is the rule: `async-parallel.md`

Suggested shape per rule:

```markdown
---
title: ...
impact: CRITICAL | HIGH | MEDIUM | LOW
impactDescription: optional metric
tags: [..]
---

## Title

Why it matters (2–4 sentences).

**Incorrect:** ...
**Correct:** ...
Notes / stack caveats (if any).
```

Keep each rule scannable (~20–80 lines). Prefer one clear before/after over essays.

### `AGENTS.md` (optional bulk reference)

- Full upstream or long compiled guide
- Label clearly: “reference only; prefer `rules/` + SKILL stack notes”
- Safe to be huge — it is not auto-injected on skill load

## Authoring workflow

1. Collect raw knowledge (existing SKILL.md, upstream doc, postmortems).
2. Split into atomic rules (one failure mode / one pattern each).
3. Adapt rules to this stack (replace foreign APIs; add “do not introduce”).
4. Write router `SKILL.md`: priorities + index + caveats only.
5. Park bulk in `AGENTS.md` if you still want the full source nearby.
6. Verify budget: open only `SKILL.md` in a fresh session; confirm agent can navigate to the right rule path without loading everything.
7. Tune description so the skill loads for the right tasks and not for noise.

## Agent behavior contract (put in SKILL.md)

1. Read `SKILL.md` guidance and pick 1–N rule ids for the current task.
2. `Read` only those `rules/<id>.md` files.
3. Do not bulk-read `rules/` or load all of `AGENTS.md` unless asked or stuck.
4. Prefer higher-priority categories when reviewing/refactoring.

## What makes it work

- Discovery ≠ load — listing a skill is cheap; loading is opt-in.
- Load ≠ library — Skill tool injects router, not the tree.
- Library is path-addressable — stable filenames + index lines = targeted `Read`.
- Stack fork at the edge — adaptations live in `SKILL.md` + adapted rules; upstream stays quarantined in `AGENTS.md`.

## Anti-patterns

- One giant `SKILL.md` with all examples inlined
- “Always read every file in `rules/`” instructions
- Duplicating full rule bodies in both `SKILL.md` and `rules/`
- Unstable renames (breaks muscle memory and cross-links)
- Foreign-stack examples with no adapter layer for this repo
- Vague `description` → skill never loads, or loads constantly

## Skills vs prompts

- **Skill**: reusable knowledge pack or multi-step workflow the agent loads mid-task via Skill tool.
- **Prompt** (`ai/prompts/<category>/<name>.md`, dest `.opencode/commands/<name>.md`): user-invoked single-shot entry (`/name`). Thin wrappers may load a skill then run; pure one-liners live only as prompts (no skill folder).

Do not keep a skill whose entire body is a one-shot instruction with no reusable knowledge. Move it to prompts.

## Checklist for a progressive pack

- [ ] Folder under `ai/skills/<category>/<name>/`
- [ ] Frontmatter `name` + trigger-heavy `description`
- [ ] `SKILL.md` ≤ ~150 lines; no full essay
- [ ] `rules/` atomic, prefixed, consistent frontmatter
- [ ] Priority ordering in router
- [ ] Stack “prefer / careful / do not introduce”
- [ ] Explicit “read only relevant rules” usage note
- [ ] Optional `AGENTS.md` marked upstream/reference-only
- [ ] Spot-check: task X → agent opens 1–3 rule files, not the whole tree

## When not to progressive-split

Keep sequential workflow skills as ordered steps in `SKILL.md` (optionally disclose large prompt templates under `references/`). Examples: diagnose phases. One-shot workflows such as `/verify-issue`, `/arena`, and `/swarm` are prompts. Progressive `rules/` fits knowledge packs (many independent patterns), not linear playbooks.
