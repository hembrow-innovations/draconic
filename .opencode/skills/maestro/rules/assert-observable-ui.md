---
title: Assert what the user can see
impact: MEDIUM
impactDescription: implementation checks rot and miss the bug
tags: [assert]
---

## Assert what the user can see

A flow proves a journey. Assert the next screen, the new row, the error banner. Do not assert internal flags.

```yaml
- tapOn:
    id: save_task
- assertVisible: "Buy groceries"
```

**Incorrect:** `assertTrue: ${output.reduxReady}` or an AI screenshot assert as the only check that a task was created.

**Correct:** `assertVisible` or `assertNotVisible` on copy or `id:` the user would notice. Keep `assertWithAI` off the default path.

Notes: One journey, one outcome (`flow-one-behavior`).
