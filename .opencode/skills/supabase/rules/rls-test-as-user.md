---
title: Test as the role the app uses
impact: HIGH
impactDescription: service_role tests are always green
tags: [rls, test]
---

## Test as the role the app uses

Secret keys and `service_role` bypass RLS. A passing query with those credentials says nothing about the client.

**Incorrect:** Using the service role in a test or Studio session to "see if the table works".

**Correct:** Sign in as a seeded user, or `set local role authenticated` with a `request.jwt.claim.sub`. Confirm both allow and deny cases.

Notes: `test-against-local`.
