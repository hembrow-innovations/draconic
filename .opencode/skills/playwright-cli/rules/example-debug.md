---
title: Debug with console, requests, then trace
impact: MEDIUM
tags: [example, debug]
---

## Debug with console, requests, then trace

```bash
playwright-cli click e15
playwright-cli console
playwright-cli requests
playwright-cli tracing-start
playwright-cli click e15
playwright-cli tracing-stop
```

**Incorrect:** Increasing waits or re-running the whole flow after one missed click.

**Correct:** Snapshot, check console and requests, highlight the ref you meant. Trace only if that is not enough. Attaching to `npx playwright test --debug=cli` is a different workflow. Stay on this CLI unless the user asked to heal a committed spec.

Notes: Committed specs are `layer-not-committed-suite`.
