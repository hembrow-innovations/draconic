---
name: draconic-loop
description: "Run one atomic Draconic language Loop: pick the next Roadmap item, implement it test-first, mark done only when tests pass. Use when the user wants to advance Draconic, run the language build loop, grow the compiler/runtime, or mentions draconic-loop / next feature / continue the language."
---

# Draconic Loop

One **Loop** = one atomic Roadmap item, red → green → done. Completeness is **Roadmap + Conformance suite**, not model judgment.

Read before work: `CONTEXT.md`, relevant `docs/adr/`, `ROADMAP.md`.

## Steps

1. **Claim** — In `ROADMAP.md`, find the first `todo` item whose blockers are satisfied (lower bootstrap IDs done; do not start `E*` clusters until **B10** is `done` unless the user overrides). Prefer historical **B/E/T/N/U** before Phase 2 **P/S** unless the user asks for product/spec work. Set it to `in_progress`. Exactly one `in_progress` at a time.
   - **Empty-todo guard:** if there is **no** `todo` row anywhere on the Roadmap, **stop immediately**. Do not invent work, do not add speculative Roadmap rows, do not “keep the loop busy.” Report empty board; further work needs a human or a vault issue.
2. **Orient** — Read existing tests and code for that item’s **Tests** paths. Skim ADRs if the item touches IR, Runtime, Embed, or dual worlds.
3. **Red** — Add or extend tests that fail for the missing behavior. Prefer crate unit tests for compiler pieces; `tests/conformance/**` for language semantics; both backends when **Targets** is `both`.
4. **Green** — Implement the minimum to pass. No silent subsetting of ECMA-262: if the item’s scope is large, **split** the Roadmap row into child IDs (e.g. `E01.01`) and complete only the claimed child this Loop.
5. **Verify** — Run `cargo test` for touched crates, then `cargo test --workspace` (**fast by default**). Full Test262 allowlist (~37k Node runs) is **opt-in**:
   - Default: workspace tests use a **smoke** allowlist slice only (`DRACONIC_TEST262_FULL` unset).
   - Always **probe** newly added allowlist paths (or the item’s target fixtures) with `cargo build -p draconic-test262 --bin probe --release` + `./target/release/probe` before `done`.
   - For **Expand allowlist** rows or when you changed many allowlist entries: `DRACONIC_TEST262_FULL=1 cargo test -p draconic-test262 --lib default_run` (parallel via `DRACONIC_TEST262_JOBS`, default = CPU count).
   - Run `cargo build -p draconic-cli` if the CLI changed.
   - Do **not** burn a Loop on a serial full-suite run; prefer probe + unit tests, full gate only when expanding.
6. **Close** — Set the item to `done` only if verify is green. Never `done` with failing or missing tests. If blocked on tools (e.g. system LLVM), set `blocked` and note the reason in the Roadmap cell or a short comment under the table.
7. **Commit** — Stage all changes for this work package and create a git commit before stopping or claiming the next item. Message: Roadmap ID(s) + short summary (e.g. `E03.05 Arrow functions: parse, lower, emit, conformance`). One commit per completed Loop item.
8. **Stop** — Default: **one item per invocation**. Do not start the next item unless the user says to continue.

## Completion criterion

The Loop is complete when: claimed item is `done` or `blocked` with reason, workspace tests relevant to the change pass, the work package is committed, and no second item was started.

## Rules

- **Tests are truth.** Do not mark progress from reading code alone.
- **Both targets when required.** For `both`, JS and native paths must be tested or explicitly split into `js` / `native` child rows — never ship one backend and call the language feature done.
- **Hard error > wrong code.** Native-only / JS-only features must diagnostic on the other backend.
- **No empty-board invention.** Zero `todo` ⇒ stop (see Claim step). Completeness is Roadmap + suite, not model judgment.
- **Vocabulary:** use terms from `CONTEXT.md` (Program, IR, Frontend, JS backend, LLVM backend, Runtime, Embed, Roadmap, Loop).
- **Drive with `/tdd`** when implementing non-trivial behavior.
- **Commit every work package.** Never leave a finished Loop uncommitted; never batch multiple done items into one commit unless the user asks.

## Pointers

- Roadmap: [`ROADMAP.md`](../../../ROADMAP.md)
- Glossary: [`CONTEXT.md`](../../../CONTEXT.md)
- ADRs: [`docs/adr/`](../../../docs/adr/)
- Workspace: `cargo test --workspace`, binary `draconic` from `crates/draconic-cli`
