---
title: Un-swallow nested taps on iOS
impact: HIGH
impactDescription: RN wrappers eat the inner control
tags: [sel, ios, accessible]
---

## Un-swallow nested taps on iOS

React Native on iOS can hide an inner pressable when the wrapper is `accessible`. Maestro then taps the outer node or misses.

```jsx
<TouchableOpacity accessible={false}>
  <Text>Wrapper</Text>
  <TouchableOpacity accessible={true} testID="inner_action">
    <Text>I'm a small button</Text>
  </TouchableOpacity>
</TouchableOpacity>
```

**Incorrect:** Repeating `tapOn: "I'm a small button"` and raising timeouts when the wrapper owns the hit target.

**Correct:** Flip `accessible` so the inner node is the one in the tree. Target `id: inner_action`.

Notes: Android is usually fine. Apply this when iOS is the only miss.
