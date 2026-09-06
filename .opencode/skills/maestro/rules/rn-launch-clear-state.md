---
title: Isolate journeys with clearState
impact: HIGH
impactDescription: leftover session state makes the next flow lie
tags: [rn, launch, state]
---

## Isolate journeys with clearState

`launchApp` stops and restarts by default. It does not clear data unless you ask.

```yaml
- launchApp:
    clearState: true
    permissions:
      all: allow
```

**Incorrect:** A "sign in" flow that only works when a previous flow already signed out, with no `clearState`.

**Correct:** Clear state at the start of a journey that assumes a fresh install. Share that launch via `runFlow` if the suite already does. Pass `permissions` here instead of tapping system dialogs.

Notes: `clearKeychain: true` is iOS-only and wipes the whole keychain. Use it when the suite already does.
