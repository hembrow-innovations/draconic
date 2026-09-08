---
title: Direct boring code over magic
impact: HIGH
impactDescription: Brittle, magical, or thin wrappers are quality problems
tags: [std, simplicity, abstractions]
---

## Direct, boring, maintainable code

Prefer direct, boring, maintainable code over hacky or magical code.

- Treat brittle, ad-hoc, or "magic" behavior as a code-quality problem.
- Be skeptical of generic mechanisms that hide simple data-shape assumptions.
- Flag thin abstractions, identity wrappers, or pass-through helpers that add indirection without buying clarity.
