---
title: Archetypes are navigation
impact: HIGH
tags: [contract]
---

# Archetypes are navigation

An archetype is a contract fragment naming a reusable promise-set. A finer contract inherits by linking `[[records-table]]`, then: inherit all (just link), extend (add promises), inherit part (`except records-table:delete`, a visible opt-out), or override.

Obsidian backlinks make an archetype show every consumer and an ADR show every promise realising it.

The checker never resolves this. It only enforces promises physically present in a file. Inheritance is for humans and navigation.

No archetype exists until a real repeated pattern earns it. YAGNI. `records-table` in this rule is an example name, not a required archetype.
