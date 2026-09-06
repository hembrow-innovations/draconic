---
id: "ticket-588-b03-parse-cli-untested"
title: "B03 Tests column does not spawn `draconic parse`"
kind: ticket
status: open
ticket_type: bug
tags: []
created_at: "2026-09-06T12:04:48Z"
updated_at: "2026-09-06T12:04:48Z"
---

# B03 Tests column does not spawn `draconic parse`

## Signal

Roadmap B03 is `done` for the CLI verb `draconic parse <file>` printing an AST dump. The Tests path `crates/draconic-cli` has no binary spawn of that verb. `parse_sample_program` calls `parse_and_dump` in-process. Deleting `cmd_parse` would still leave that test green.

## Fit

this project, later slice

## Notes

- **Roadmap ID**: B03. Item: CLI: `draconic parse <file>` prints AST dump. Targets: compiler. Tests: `crates/draconic-cli`.
- **Command**: `cargo test -p draconic-cli parse_sample_program` — 1 passed (`tests::parse_sample_program`). `cargo run -p draconic-cli -- parse examples/shebang/hello.drac` printed a `Program` dump (exit 0).
- **path:line**: `crates/draconic-cli/src/main.rs:85` (`cmd_parse`), `:1595` (`parse_sample_program`). No `tests/*.rs` uses `CARGO_BIN_EXE` with `parse`. Sibling coverage lives in `tests/integration/tests/install_smoke.rs` (`fresh_path_draconic_parse_hello`), outside the Tests column.
