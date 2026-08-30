# Pi runtime

- No Skill tool. Read the file.
- No Task tool. Use `subagent` for fan-out. If that tool is missing, do the work in this session and review your own diff.
- No MCP. Use git, gh, and project CLIs.
- Decision log lives at `.heio/decisions.tsv`.
- Project rules in AGENTS.md win on layout and tooling.

## Search

Correctness first. Pay for another read rather than guess.

- Known identifier (exact function, type, or hook name): Lens `symbol_search`, then read that symbol. Do not grep a name you already know.
- Typo, filename, or raw text: `find` then `grep` (FFF). After 1-2 searches, read the top hit before searching again.
- If the read does not confirm the answer, search again. Never answer from snippets alone.
- Do not use bash `rg` or `fd`. Do not keep grepping to avoid reading.
