---
title: Bias toward cleaning the design
impact: HIGH
impactDescription: Working code that messes the codebase is not enough
tags: [std, design, simplification]
---

## Bias toward cleaning the design

- If behavior can stay the same while the structure becomes meaningfully cleaner, push for the cleaner version.
- Do not rubber-stamp "it works" implementations that leave the codebase messier.
- Strongly prefer simplifications that remove moving pieces altogether over refactors that merely spread the same complexity around.
