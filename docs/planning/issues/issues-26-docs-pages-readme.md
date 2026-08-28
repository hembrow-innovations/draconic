---
id: issues-26
created_at: "2026-08-28"
updated_at: "2026-08-28"
area: planning
domain: language
title: "Publish docs site and link README"
description: "CI publishes generated HTML to GitHub Pages; README links the site; mark P03 done."
status: open
issue-type: feature-request
severity: medium
tags:
  - planning
  - issue
  - enhancement
  - docs
  - ready-for-agent
---

# Publish docs site and link README

## Parent

[[issues-19-language-docs-site]]

## Description / What to build

The site is public. CI runs the website pipeline and publishes HTML to GitHub Pages. The README stays clone-build-run and links here. Roadmap **P03** becomes `done` only when that is true.

## Blocked by

- [[issues-23-docs-fence-pipeline]]
- [[issues-24-docs-learn-skeleton]]
- [[issues-25-docs-reference-skeleton]]

## Acceptance criteria

- [ ] CI generates the site with the Draconic generator and deploys HTML to GitHub Pages
- [ ] Dist is not the authoring source of truth
- [ ] README links the public site and still documents write-parse-build
- [ ] Roadmap **P03** is `done` with tests pointing at the pipeline and the README link

## Agent Brief

### Goal

Make the skeleton a real website and close **P03** honestly.

### Contract

- GitHub Pages from CI. Do not commit generated HTML as the source of truth.
- README remains onboarding, not the book.
- **P03** done only when pipeline is green and the README link exists.
- Close this ticket; leave parent [[issues-19-language-docs-site]] for a human to close after review.

### Out of scope

Custom domain. Playground. Changing language semantics.

## Comments

> Child of [[issues-19-language-docs-site]]. Last slice for **P03**.
