---
title: Message Chains
impact: MEDIUM
tags: [smell, fowler, encapsulation]
---

## Message Chains

Long `a.b().c().d()` navigation the caller shouldn't depend on.

**Remedy:** Hide the walk behind one method on the first object.
