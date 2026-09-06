---
title: Use maestro test on a real path
impact: CRITICAL
impactDescription: the CLI is the runner when no wrapper exists
tags: [run, cli]
---

## Use maestro test on a real path

When the repo has no wrapper, call the CLI on a path you discovered.

```bash
maestro test path/to/flow.yaml
maestro test .maestro/
```

**Incorrect:** Inventing `npm run e2e:mobile` or a workspace script the package.json does not define.

**Correct:** `maestro test` on the existing folder or file. Add `--include-tags` only when the suite already tags flows.

Notes: `maestro studio` is authoring, not the suite. See `run-studio-optional`.
