---
id: issues-26
created_at: "2026-08-28"
updated_at: "2026-08-29"
area: planning
domain: language
title: "Publish docs site and link README"
description: "CI publishes generated HTML to GitHub Pages; README links the site; mark P03 done."
status: closed
issue-type: feature-request
severity: medium
tags:
  - planning
  - issue
  - enhancement
  - docs
  - closed
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

- [x] CI generates the site with the Draconic generator and deploys HTML to GitHub Pages
- [x] Dist is not the authoring source of truth
- [x] README links the public site and still documents write-parse-build
- [x] Roadmap **P03** is `done` with tests pointing at the pipeline and the README link

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
>
> **2026-08-29:** Landed. CI workflow `.github/workflows/docs-pages.yml` runs `scripts/generate-website.sh` (Draconic generator → `dist/pages`) and deploys with `actions/deploy-pages`. README links https://hembrow-innovations.github.io/draconic/ and stays clone-build-run. Generated HTML stays gitignored. Roadmap **P03** is `done`. Pipeline tests: `tests/integration/tests/website_pipeline.rs`. Parent [[issues-19-language-docs-site]] stays open for human review.
