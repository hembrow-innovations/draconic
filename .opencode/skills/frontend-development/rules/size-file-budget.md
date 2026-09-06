---
title: File size budget
impact: CRITICAL
impactDescription: Huge files hide seams and stall review
tags: [size, layout, dx]
---

## File size budget

Target ≤1000 lines per file. Hard limit 1250. Past the hard limit, split before adding more.

**Incorrect:** a 1600-line `VehicleScreen.tsx` that owns fetch, table, dialogs, and helpers.

**Correct:** split by seam into nested folders:

```
features/vehicles/detail/
  VehicleDetailScreen.tsx      # compose only
  header/VehicleHeader.tsx
  trips/TripList.tsx
  fuel/FuelPanel.tsx
```

**Notes.** Count the whole file. Generated or vendored files are exempt. A types file and a variants file next to the component count separately. If a split would be a shallow pass-through, move behaviour behind the interface instead of adding a file of aliases. See `arch-deep-modules` and `arch-src-nested`.
