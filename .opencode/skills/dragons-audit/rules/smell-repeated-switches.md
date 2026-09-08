---
title: Repeated Switches
impact: HIGH
tags: [smell, fowler, polymorphism]
---

## Repeated Switches

The same `switch`/`if`-cascade on the same type recurs across the change.

**Remedy:** Replace with polymorphism, or one map both sites share.
