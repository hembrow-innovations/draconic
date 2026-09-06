---
title: testID becomes data-testid
impact: HIGH
impactDescription: querying testID as an id misses the node
tags: [rn, testid]
---

## testID becomes data-testid

React Native Web maps `testID` to `data-testid`. Playwright `getByTestId` reads that.

```jsx
<Pressable testID="submit-button" onPress={onSubmit}>
  <Text>Save</Text>
</Pressable>
```

```bash
playwright-cli click "getByTestId('submit-button')"
playwright-cli eval "el => el.getAttribute('data-testid')" e5
```

**Incorrect:** `click "#submit-button"` or `getByRole('button')` on a view that only set `testID`.

**Correct:** Add `testID` in the app when the control is icon-only or has colliding copy. Click `getByTestId` or the snapshot ref.

Notes: `accessibilityLabel` becomes `aria-label`. Use it for the accessible name when you also want a role.
