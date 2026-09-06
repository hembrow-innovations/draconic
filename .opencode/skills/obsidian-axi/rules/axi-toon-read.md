---
title: Read TOON as-is
impact: CRITICAL
impactDescription: JSON conversion and extra grep waste the AXI savings
tags: [axi, toon]
---

## Read TOON as-is

Output is TOON. Lists already carry `count:`. Default schemas are 3–4 fields. The format is the point of the CLI.

**Incorrect:** Piping every call through `jq`, wrapping with `--json`, or grepping a short list just to count rows.

**Correct:** Read the TOON. Use `count:`. Add `--fields path` when you only need paths. Use `read --metadata` when you only need frontmatter and size.

Notes: Pipe through `grep` or `head` only when a list is genuinely long. `--full` is the escape hatch for truncated bodies (`read-truncate-full`).
