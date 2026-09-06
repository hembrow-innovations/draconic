---
title: Newlines via --content-file or stdin
impact: HIGH
impactDescription: inline --content drops or mangles multiline bodies
tags: [mut, content]
---

## Newlines via --content-file or stdin

Shell quoting breaks multiline `--content`. Write the body to a file and pass `--content-file`, or pipe stdin to `append`.

**Incorrect:** `obsidian-axi write Inbox/idea.md --content "# Idea\n\nMore" --vault docs`

**Correct:**

```sh
obsidian-axi write Inbox/idea.md --content-file /tmp/idea.md --vault docs
echo "- piped line" | obsidian-axi append Inbox/log.md --vault docs
```

Notes: New durable notes in this repo still need **docs** placement and templates (`layer-docs-create`). `append` adds to the end (`append-end`).
