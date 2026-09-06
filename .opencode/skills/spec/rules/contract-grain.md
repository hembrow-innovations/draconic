---
title: One promise, many tests
impact: HIGH
tags: [contract]
---

# One promise, many tests

Promise = a unit of **intent** (something a human would list). Test = a unit of **verification** (one case). Different axes → **one promise, many tests**.

Same promise / more cases → more `test:` lines, **not** more promises.

A promise that reads like a test ("returns 7 when week") is mis-grained. Raise it to intent. Push cases into `test:` lines.

Put a contract at any **coherent unit of functionality**, not one-per-feature. A feature is a folder of finer contracts. Skipping levels breaks nothing.

Large features: section by concern (`data`, `ui`, …) when natural, not by framework. Small features: one `contract.md` is enough.

Purpose is job and fences only (`purpose-job`). Do not put React Query or file layout in contracts or purpose. That is data-flows.
