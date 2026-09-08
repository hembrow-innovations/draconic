---
title: Type and boundary cleanliness
impact: HIGH
impactDescription: Casts, any, unknown, and silent fallbacks obscure invariants
tags: [std, types, boundaries, contracts]
---

## Type and boundary cleanliness

Push hard on type and boundary cleanliness when they affect maintainability.

- Question unnecessary optionality, `unknown`, `any`, or cast-heavy code when a clearer type boundary could exist.
- Prefer explicit typed models or shared contracts over loosely-shaped ad-hoc objects.
- If a branch relies on silent fallback to paper over an unclear invariant, ask whether the boundary should be made explicit instead.
