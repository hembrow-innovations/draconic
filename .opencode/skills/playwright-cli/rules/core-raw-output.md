---
title: Pipe with --raw
impact: MEDIUM
tags: [core, raw, json]
---

## Pipe with --raw

Default output includes page status, generated code, and a snapshot. `--raw` returns only the result.

```bash
playwright-cli --raw snapshot > before.yml
playwright-cli --raw eval "el => el.textContent" e5
playwright-cli --raw localstorage-get theme
TOKEN=$(playwright-cli --raw cookie-get session_id)
playwright-cli list --json
```

**Incorrect:** Parsing the full snapshot blob to extract one string.

**Correct:** `--raw` on `eval`, `cookie-get`, `localstorage-get`, `generate-locator`, and `snapshot` when you will pipe or diff.

Notes: `--json` wraps whole replies. Use it for `list`, not for a single eval.
