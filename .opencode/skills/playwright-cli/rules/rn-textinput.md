---
title: Fill TextInput. Do not fake a keyboard
impact: HIGH
tags: [rn, textinput]
---

## Fill TextInput. Do not fake a keyboard

On web, `TextInput` is `input` or `textarea`. `fill` sets the value. There is no IME and no software keyboard.

```bash
playwright-cli fill e5 "dev@example.com"
playwright-cli fill e6 "secret" --submit
```

**Incorrect:** Clicking the field, then `type` plus a Maestro-style hide-keyboard step.

**Correct:** `fill` the snapshot ref or `getByTestId`. Use `--submit` when the field should press Enter. `KeyboardAvoidingView` is a no-op here.

Notes: If fill does not stick, the tree may still be a custom view. Snapshot and check it is a real input (`core-eval`).
