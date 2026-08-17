# Native depth gaps

Planning note from [[issues-3-native-depth]]. Gaps become Roadmap **N-rows (N09+)** — not a parallel private backlog. **N08.*** remains the primary native conformance drain (real ES observations off B08 hello stub); depth themes below are orthogonal and may interleave after seeds land.

## Decisions (locked 2026-07-26)

1. **Relation to N08:** N08.* stays the primary native conformance drain — do not replace it.
2. **First depth theme after N08:** GC durability / stress.
3. **Success metric:** this written gap list + at least one new Roadmap `todo` with a measurable tests path.
4. **Filing:** gaps → new Roadmap N-rows (N09+), not vault-only shadow backlog.

## JS-only / native-only / portable policy

Restated from `CONTEXT.md` (do not weaken):

| Kind | Meaning | Wrong backend must |
|------|---------|-------------------|
| **Portable** | Both backends accept with equivalent observable behavior (after documented polyfills) | N/A |
| **Native-only** | Valid only on LLVM/Runtime (e.g. `*T`, `&x`, native layout) | **Hard-error diagnostic** on JS (N04) |
| **JS-only** | Valid only on JS backend | **Hard-error diagnostic** on native |

Never silent wrong code, never hello-stub success for unsupported IR, never erase a dual-worlds boundary into a quiet coerce. Diagnostics name the policy; agents must not invent softer fallbacks.

## Already on Roadmap (N06–N08) — not net-new

| Area | Roadmap | What “done” already means | Residual vs depth |
|------|---------|---------------------------|-------------------|
| **Job queue / async** | **N06**, N06.01–N06.11 `done` | FIFO job queue ABI; Promise construct/then/statics/combinators; `async`/`await`; real native observations on `es/async/*` | Functional for fixture surface; no stress/volume, no host timers/I/O integration |
| **Embed / eval** | **N07**, N07.01–N07.04 `done` | Fold-at-emit constant-string `eval` / `Function` / indirect eval; Embed interpreter subset; real native on `es/eval/*` | Not full runtime compile-in-Runtime; large source / non-constant / deep stmt surface unsupported |
| **Real native ES observations** | **N08**, N08.01–N08.17 `todo` | Honesty drain: E01–E11, E13–E15, E17–E18, T06 native off B08 hello | Primary conformance work — keep draining; do not re-file as N09 |
| **Link Runtime + GC hello** | **N05** `done`, **B09** `done` | Static link Runtime; alloc string/object; root/collect smoke | Hello-scale only — durability is N09+ |
| **Dual-worlds rules (JS)** | **T06** `done` (js); **N08.17** `todo` (native obs) | Checker boundary + JS-side dual fixtures | Native observations still N08.17; UX/diagnostics polish is depth below |
| **Native-on-JS policy** | **N04** `done` | Pointers hard-error on JS | Policy exists; expand as new native features land |

## Gap themes (net-new candidates → N09+)

### 1. GC durability / stress — **first theme → N09**

**Current state:** Mark-sweep heap in `crates/draconic-runtime` (`draconic_rt.c`): tags string/object/promise/array; fixed root stack (`ROOT_STACK_MAX` 64); explicit `gc_collect` / `gc_live_count`; B09/N05 smoke tests only.

**Gaps:**

| Gap | Evidence / risk | Measurable path |
|-----|-----------------|-----------------|
| No volume stress | Hello alloc 1–2 values; no “many allocate / retain / drop” | `crates/draconic-runtime` tests: N allocs, root subset, collect, assert `live_count` + no crash |
| Object props not traced in mark | `mark_value` walks promise/array edges; **object `props` values not marked** | Stress + correctness: rooted object holding heap children must keep children live across collect |
| Root stack hard limit | ~~Overflow `abort`s at 64~~ → **N09.04 done** (growable stack) | Document limit or grow; stress deep root push/pop |
| No automatic collect on pressure | Collect is explicit only | Later: alloc-path threshold collect (separate Loop) |
| Cycles | No dedicated cycle fixture | Graph of mutual refs + unroot + collect → live_count 0 |
| Closures / exotic JS values | Heap tags incomplete vs full ES | Track with N08 ES surface; GC tags grow with Runtime value kinds |

**Seeded:** **N09** / **N09.01** (see `ROADMAP.md`).

### 2. Job queue depth

**Current state:** N06.01 FIFO enqueue/drain; re-entrant drain no-op; Promise reactions via jobs. Conformance covers semantics, not volume.

**Gaps (later N-rows):** many nested enqueues during drain; long reaction chains; OOM/abort paths under flood; interaction with GC roots across async boundaries; no host event-loop / timers (blocks real I/O programs — also [[issues-5-ship-real-program]]).

### 3. Stdlib surface (native Runtime)

**Current state:** Minimal std = print hooks (`print_i64`/`u64`/`f64`/`bool`/`str`) + hello + GC/Promise ABI. No sockets, fs, env, args, clock.

**Gaps (later N-rows):** decide portable vs native-only per API; hard-error on missing target; thin C ABI + LLVM declares via `draconic_runtime::abi`; conformance or runtime tests per symbol. Prefer driving from a real Program ([[issues-5-ship-real-program]]) over open-ended stdlib sprawl.

### 4. Embed limits

**Current state:** N07 fold-at-emit; constant-string eval; simple expressions + limited fold stmt/expr set; async/generator/spread/optional-member rejected in fold.

**Gaps (later N-rows):** non-constant eval strings (true runtime Embed compile); richer stmt/expr subset; error diagnostics parity with Frontend; memory bounds for eval-produced heap values (ties to GC stress). ADR-0004 destination remains full Embed — depth is incremental.

### 5. Dual-world UX / diagnostics

**Current state:** Checker dual-worlds boundary (`as`); N04 JS hard-errors for native pointers; T06 js fixtures; N08.17 native observations still todo.

**Gaps (later N-rows):** clearer diagnostic text/spans for boundary failures; catalog of portable vs native-only shapes in one place (this doc + N04 fixtures); avoid “works on js, silent wrong on native” (N08 honesty) and the reverse (N04). No policy change — only coverage and message quality.

### 6. Link / debug

**Current state:** LLVM backend links Runtime static lib; U03 source maps are **JS-only**; no DWARF/native debug story; multipath honesty closed ([[issues-10-collapse-llvm-multipath]]).

**Gaps (later N-rows):** native debug info (DWARF or llvm-equivalent) for Draconic source; reproducible link flags; crash diagnostics that name Runtime vs user code; optional `draconic build --target native` debug/release modes. Packaging for ship-real-program is adjacent, not N08.

## Priority order (agents)

1. Drain **N08.*** as primary native conformance work (unchanged).
2. Interleave **N09** GC stress atoms when picking native Runtime Loops (first depth theme).
3. After N09 parent/children green, promote next theme from this list into **N10+** with atomic rows + Tests paths — do not grow a vault-only backlog.

## Out of scope here

- Full GC rewrite / moving collector algorithm.
- Implementing N08 observations or stdlib APIs in this planning issue.
- Test262, README, examples, ADRs, issues 2/4/5/7.
