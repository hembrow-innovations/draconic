# GC runtime and dual worlds

Native binaries link a Runtime with tracing GC so JS values (objects, closures, cycles) match JavaScript semantics. Native types stay unboxed and outside the GC heap. Ownership-only and arena-only models were rejected because they cannot host a full ECMA-262 superset without cutting semantics.
