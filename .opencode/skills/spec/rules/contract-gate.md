---
title: Discover and run the checker
impact: HIGH
tags: [contract]
---

# Discover and run the checker

Discover the project's contract checker. `check:contracts` is an example, often wired into a broader gate. Do not invent a second checker.

Green prints `N locked, M asserted`. Failure lists each unlocked or broken promise and exits 1.

Never rename or delete a test that a promise points at without updating the contract. The gate will fail.

Does **not** make bugs impossible. Guarantees: no *known* promise breaks silently, and every promise is enumerated. Can't invent promises nobody thought of. A bug in *unpromised* behaviour still needs a human to add a promise. Real metric: shrinking *unpromised surface*, not "zero bugs".
