---
title: Product face load and swap
impact: MEDIUM
impactDescription: Divergent faces and FOIT across web, desktop, and mobile
tags: [type, font, web, native]
---

## Product face load and swap

One product sans. Web is the visual standard. Desktop matches the web SPA when they share it. Weights 400 / 500 / 600 / 700 unless the font package documents otherwise. Features use tokens and `font-sans`, not ad-hoc `fontFamily` strings.

**Incorrect:** a feature imports Inter locally. Mobile splash shows system font. `font-semibold` maps to a missing 600 file.

**Correct:**

1. Edit only the shared font package. Keep web CSS, native maps, and mirrors in lockstep.
2. One load site per platform. Web root class plus CSS import. Mobile `useFonts` (or the project's loader) plus splash hold until ready plus NativeWind family.
3. Map `font-semibold` to the real bold face the package ships (often 700).

**Notes.** Durable scales live in **docs**. Icon fonts are out of scope. Diagnose drift at the shared package or the single load site, never with a one-off face in a feature. See `quality-typography-spacing` and `tw-v4-theme`.
