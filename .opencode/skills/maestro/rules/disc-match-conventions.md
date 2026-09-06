---
title: Match the suite that already exists
impact: CRITICAL
impactDescription: a second layout splits the suite
tags: [disc, conventions]
---

## Match the suite that already exists

Naming, tags, subflow folders, and `config.yaml` are already a contract. Copy them.

**Incorrect:** Adding `flows/login.yml` next to `tests/e2e/mobile/signin.yaml`, or a new `appId` that is not in `app.json` or the existing header.

**Correct:** Same extension, same header keys, same tag names, same subflow directory. Read README, package scripts, justfile, and CI for the run command.

Notes: A project-local maestro skill overrides this pack on `appId` and seed users.
