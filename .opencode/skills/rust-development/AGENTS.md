# Rust development reference

Reference only. Prefer `SKILL.md` stack notes and `rules/<id>.md`.

Do not load this file unless you are stuck and the router plus two or three rules were not enough.

This pack folds and replaces:

- `codebase-design` (Rust crate modules; keep the standalone skill for the shared vocabulary)
- `diagnose` (Rust bugs; keep the standalone skill for non-Rust)

Testing, review, and language Loop stay in sibling skills. Load them; do not copy their bodies here:

- `spec`, `tdd`, `draconic-loop`, `gauntlet-loop`

Locked decisions that constrain Rust live in `docs/adr/`:

- `0001` compiler stays Rust
- `0002` one shared IR, two backends
- `0003` dual worlds (GC JS values, unboxed native)
- `0005` TS-inspired checker, JS backend emits JS
- `0008` sockets-first host I/O, permissive default
- `0009` git packages, `draconic.toml` / `draconic.lock`
- `0011` catchable exceptions vs `draconic_rt_abort`
- `0012` keep `cargo test --workspace`; 10 minute oracle

Glossary: `CONTEXT.md`. Completeness: `ROADMAP.md`.

Deep-module deepening: `../codebase-design/DEEPENING.md`.
