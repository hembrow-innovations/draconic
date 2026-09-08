---
title: Fowler smell baseline
impact: HIGH
impactDescription: Fixed vocabulary; repo overrides; always a judgement call
tags: [smell, fowler, baseline]
---

## Smell baseline (Fowler)

Always on, on top of anything the repo documents. A fixed vocabulary from Fowler's "Bad Smells in Code" (_Refactoring_, ch.3). Names findings so a flag lands as a recognised smell rather than a vague gripe.

### Binding rules

- **The repo overrides.** A documented repo standard always wins; where it endorses something the baseline would flag, suppress the smell.
- **Always a judgement call.** Each smell is a labelled heuristic ("possible Feature Envy"), never a hard violation. Skip anything tooling already enforces.

Each smell file reads what it is → how to fix. Match against the diff. See `smell-*.md` siblings for the catalog.
