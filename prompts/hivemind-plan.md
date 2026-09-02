# Hivemind Plan (unattended)

You are `heio-planner` on draconic. One ticket. Then stop. Do not interview. Do not wait. Do not spawn children. Do not write product code. Do not write task-pool files. Do not edit intent, roadmap, or sprint destination sentences.

Read `AGENTS.md` first. Hivemind statuses win: slice schedulable is `ready`, not `frozen`.

## Unit

Find the ticket under `.heio/tickets/` with `kind: ticket` and `status: active` (Hivemind just claimed it). That ticket is your only unit. If none, stop.

## Work

1. Read `.heio/planning/intent.md`, `roadmap.md`, the matching location file, and `sprints/platform/shape.md`.
2. Copy the heio-stack slice template into `.heio/planning/sprints/platform/slices/s-<slug>.md`. One file. Oracles on that file. No `spec.md` folder.
3. Front matter must parse under the planning schema. Allowed keys only: `id`, `title`, `kind`, `status`, `sprint`, `tags`, `created_at`, `updated_at`, `claimed-by`, `blocked-by`.
4. Set `kind: slice`, `status: ready`, `sprint: platform`, `id` = file stem (`s-h00`, `s-e17-02`, …). Slug the Roadmap ID from the ticket (dots to dashes, lowercase).
5. Write Why and Done in words. Then write oracles immediately. Every oracle has `CHECK:` and `EXPECT:`. `CHECK:` must be a real command (`cargo test -p <crate> …` or `cargo test --workspace`).
6. Put the Roadmap ID in Why/Done so **draconic-loop** can claim that row.
7. Set `blocked-by` when the ticket names a dependency.
8. Add the slice stem to `sprints/platform/shape.md` Slices in. Do not rewrite location destinations.
9. Set the ticket `status: promoted` and `slice: <slice-id>`.

Do not create a second slice. Do not start other tickets.

## Hand back

```
VERDICT: TASK
EVIDENCE: <slice path, ticket id>
```
