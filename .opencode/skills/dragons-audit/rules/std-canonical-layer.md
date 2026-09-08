---
title: Canonical layer and reuse
impact: HIGH
impactDescription: Logic belongs in the owning package; reuse existing helpers
tags: [std, architecture, layers, dry]
---

## Canonical layer and reuse

Keep logic in the canonical layer and reuse existing helpers.

- Call out feature logic leaking into shared paths or implementation details leaking through APIs.
- Prefer existing canonical utilities/helpers over bespoke one-offs.
- Push code toward the right package, service, or module instead of normalizing architectural drift.

**project note:** Respect `packages/<group>/<domain>/<platform>/` ownership and feature vs shared boundaries.
