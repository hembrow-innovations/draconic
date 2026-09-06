---
title: Do not hideKeyboard to dismiss
impact: HIGH
impactDescription: Android hideKeyboard is a back press
tags: [input, keyboard]
---

## Do not hideKeyboard to dismiss

On Android, `hideKeyboard` sends the system back event. That often pops the screen or closes a modal. On iOS it swipes the middle of the screen.

**Incorrect:**

```yaml
- inputText: "secret"
- hideKeyboard
- tapOn: Submit
```

**Correct:** After the last field, `pressKey: Enter` to submit, or tap a non-input `id:` (header, title) to dismiss. Then tap Submit if Enter did not submit.

Notes: Official docs confirm the Android back implementation. See `input-presskey-enter`.
