---
title: Local confirmations can stay off
impact: LOW
impactDescription: confirm-required local auth blocks every new test user
tags: [auth]
---

## Local confirmations can stay off

`auth.email.enable_confirmations` defaults to false locally. That is fine. Turn it on when you are testing the confirm path.

**Incorrect:** Enabling confirmations and production SMTP so "local matches prod" for every task.

**Correct:** Leave confirmations off unless the task is the email confirm flow. Use Mailpit when they are on.

Notes: Hosted auth settings are a different project. Do not copy them down unless asked.
