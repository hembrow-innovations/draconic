---
title: Submit the last field with Enter
impact: HIGH
impactDescription: the keyboard covers the submit control
tags: [input, keyboard]
---

## Submit the last field with Enter

Software keyboards cover the primary button. Tapping Submit then hits the key or misses.

```yaml
- tapOn:
    id: password_input
- inputText: ${PASSWORD}
- pressKey: Enter
- assertVisible:
    id: home_root
```

**Incorrect:** `tapOn: Submit` while the keyboard is still up, then a long sleep.

**Correct:** `pressKey: Enter` after the last `inputText`. Assert the next screen. Tap Submit only if the form does not treat Enter as submit.

Notes: Never pair this with `hideKeyboard`. See `input-no-hidekeyboard`.
