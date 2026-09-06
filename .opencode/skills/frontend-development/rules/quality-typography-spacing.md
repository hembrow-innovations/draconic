---
title: Typography and spacing
impact: MEDIUM
impactDescription: Hierarchy without color-only cues
tags: [quality, type, spacing]
---

## Typography and spacing

Use the project font scale. Build hierarchy with weight and size, not color alone. Prefer Tailwind spacing scale with generous whitespace.

Face load and swap live in `type-face-load`. Durable scales and token notes live in **docs**. Do not hardcode alternate sans stacks in feature UI.

**Incorrect:**

```tsx
<p className="text-sm text-blue-600">Section title</p>
<p className="text-sm text-gray-500">Body that looks the same weight</p>
<div className="p-[13px] gap-[7px]">...</div>
```

**Correct:**

```tsx
<h2 className="text-lg font-semibold text-foreground">Section title</h2>
<p className="text-sm text-muted-foreground">Supporting body</p>
<div className="p-4 gap-3">...</div>
```

**Notes.** Color can reinforce state. It must not be the only difference between levels.
