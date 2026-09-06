---
title: Exit 0 includes no-ops
impact: CRITICAL
impactDescription: empty results and retries look like failure if you only watch stderr
tags: [axi, errors]
---

## Exit 0 includes no-ops

Exit `0` worked, including “nothing to do”. `1` failed. `2` the command was wrong. Errors print on stdout as TOON with `code:` and `help[]`. No interactive prompts.

**Incorrect:** Treating empty stdout as failure, or looking only at stderr.

**Correct:** Read stdout. A `count: 0` line is a definitive empty. Retrying an append of text already at the end, a delete of a missing note, or a move onto itself is exit 0.

Notes: Example error shape:

```
error: Note not found `Notes/Roadmap.md`
code: NOTE_NOT_FOUND
help[2]:
  obsidian-axi ls Notes
  obsidian-axi search Roadmap
```
