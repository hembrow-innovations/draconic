# Domain docs (single-context layout)

This vault is a single-context layout. `AGENTS.md` names this note and wins over the docs skill layout-vault: locked decisions live in `docs/adr/`, not `docs/decisions/adr/`.

- **Glossary**: [[CONTEXT]] (`CONTEXT.md`)
- **Locked decisions**: `docs/adr/`
- **Completeness**: [[ROADMAP]] (`ROADMAP.md`) — Loop source of truth with the Conformance suite

Heio-stack under `.heio/` is the agent operating system (verdicts, slices, tickets, pump). It is not language completeness and must not replace [[ROADMAP]]. `.heio/decisions.tsv` is dest-runtime and gitignored; do not commit it.

Language Loop atoms stay on `ROADMAP.md` plus tests. Agent occupancy and Hivemind nouns live in `AGENTS.md`.
