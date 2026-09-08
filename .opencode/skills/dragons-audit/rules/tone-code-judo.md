---
title: Code judo posture
impact: CRITICAL
impactDescription: Always on; ambitious structural simplification
tags: [tone, posture, code-judo, ambition]
---

## Code judo posture

Do not stop at local cleanup. Actively search for restructurings that preserve behavior while making the implementation dramatically simpler, smaller, more direct, and more elegant.

### Core prompt baseline

> Perform a deep code quality audit of the current branch's changes.
> Rethink how to structure / implement the changes to meaningfully improve code quality without impacting behavior.
> Work to improve abstractions, modularity, reduce spaghetti code, improve succinctness and legibility.
> Be ambitious, if there is a clear path to improving the implementation that involves restructuring some of the codebase, go for it.
> Be extremely thorough and rigorous. Measure twice, cut once.

### Ambition rules

- Do not stop at "this could be a bit cleaner."
- Look for opportunities to reframe the change so that whole branches, helpers, modes, conditionals, or layers disappear entirely.
- Prefer the solution that makes the code feel inevitable in hindsight.
- Assume there is often a "code judo" move: a re-organization that uses the existing architecture more effectively.
- If you see a path to delete complexity rather than rearrange it, push hard for that path.

### Do not settle for

- "Maybe rename this" when the real issue is structural.
- A merely cleaner version of the same messy idea when a much simpler idea is plausible.
