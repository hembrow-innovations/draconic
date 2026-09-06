---
title: Prefer testID as Maestro id
impact: CRITICAL
impactDescription: visible text breaks on copy and locale
tags: [sel, testid]
---

## Prefer testID as Maestro id

React Native `testID` is Maestro `id:`. Visible `title` or `Text` is `text:` and is a regex. Icons, tabs, and translated copy need an id.

```jsx
<TextInput placeholder="Username" testID="username_input" />
```

```yaml
- tapOn:
    id: username_input
- inputText: "dev@example.com"
```

**Incorrect:** `tapOn: "Log in"` as the only selector in an app that localizes or restyles that button.

**Correct:** `id:` from `testID` for anything a flow must hit more than once. Text is fine for a unique assertion of user-visible copy.

Notes: If the control has no `testID`, add one in the app (`sel-add-testid-in-app`).
