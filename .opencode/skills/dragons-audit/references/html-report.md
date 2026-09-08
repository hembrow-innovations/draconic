# HTML report format

The audit is rendered as a single self-contained HTML file in the OS temp directory. Tailwind and Mermaid both come from CDNs. Mermaid handles graph-shaped diagrams; hand-built divs and inline SVG handle editorial visuals (mass diagrams, cross-sections). Mix the two.

## Scaffold

```html
<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <title>Dragons audit — {{repo name}}</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script type="module">
      import mermaid from "https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.esm.min.mjs";
      mermaid.initialize({ startOnLoad: true, theme: "neutral", securityLevel: "loose" });
    </script>
    <style>
      .seam { stroke-dasharray: 4 4; }
      .leak { stroke: #dc2626; }
      .deep { background: linear-gradient(135deg, #0f172a, #1e293b); }
    </style>
  </head>
  <body class="bg-stone-50 text-slate-900 font-sans">
    <main class="max-w-5xl mx-auto px-6 py-12 space-y-12">
      <header>...</header>
      <section id="verdict">...</section>
      <section id="quality" class="space-y-10">...</section>
      <section id="candidates" class="space-y-10">...</section>
      <section id="top-recommendation">...</section>
    </main>
  </body>
</html>
```

## Header

Repo name, date, and a compact legend: solid box = module, dashed line = seam, red arrow = leakage, thick dark box = deep module. No introduction paragraph.

## Verdict

One card. Approval bar from `rules/output-approval-bar.md`: approve, or do not approve, with the blocking reasons. Behavior-correct is not enough.

## Quality finding card

Each finding is one `<article>`. High-conviction first (`rules/output-priority.md`).

- **Title** — short, names the problem
- **Badge row** — band (`CRITICAL` = red, `HIGH` = amber, `MEDIUM` = slate, `LOW` = stone) plus rule id (`std-no-spaghetti`, `smell-shotgun-surgery`, …)
- **Files** — monospaced list, `font-mono text-sm`
- **Problem** — one sentence. What hurts.
- **Code-judo move** — one sentence. The restructuring that deletes complexity, if there is one
- **Remedy** — one sentence from the preferred-remedy posture

No paragraphs of explanation. Do not flood with nits if structural issues exist.

## Deepening candidate card

The diagrams carry the weight. Prose is sparse and uses the glossary terms from `vocabulary.md` without ceremony.

Each candidate is one `<article>`:

- **Title** — short, names the deepening (e.g. "Collapse the Order intake pipeline").
- **Badge row** — recommendation strength (`Strong` = emerald, `Worth exploring` = amber, `Speculative` = slate), plus a tag for the dependency category (`in-process`, `local-substitutable`, `ports & adapters`, `mock`).
- **Files** — monospaced list, `font-mono text-sm`.
- **Before / After diagram** — the centrepiece. Two columns, side by side.
- **Problem** — one sentence. What hurts.
- **Solution** — one sentence. What changes.
- **Wins** — bullets, ≤6 words each.
- **ADR callout** (if applicable) — one line in an amber-tinted box.

If the diagram needs a paragraph to be understood, redraw the diagram.

Use domain glossary vocabulary for the domain, and `vocabulary.md` for the architecture. If the glossary defines "Order," talk about "the Order intake module" — not "the FooBarHandler."

**ADR conflicts**: if a candidate contradicts an existing ADR, only surface it when the friction is real enough to warrant revisiting the ADR. Mark it clearly. Don't list every theoretical refactor an ADR forbids.

## Diagram patterns

Pick the pattern that fits. Mix them.

### Mermaid graph

Use a Mermaid `flowchart` or `graph` when the point is "X calls Y calls Z." Style leakage edges red and the deep module dark. Sequence diagrams work for "before: 6 round-trips; after: 1."

```html
<div class="rounded-lg border border-slate-200 bg-white p-4">
  <pre class="mermaid">
    flowchart LR
      A[OrderHandler] --> B[OrderValidator]
      B --> C[OrderRepo]
      C -.leak.-> D[PricingClient]
      classDef leak stroke:#dc2626,stroke-width:2px;
      class C,D leak
  </pre>
</div>
```

### Hand-built boxes-and-arrows

Modules as `<div>`s with borders and labels. Arrows as inline SVG. Use when the "after" should feel like one thick-bordered deep module with greyed-out internals.

### Cross-section

Stack horizontal bands (`h-12 border-l-4`) to show layers a call passes through. Before: 6 thin layers. After: 1 thick band labelled with the consolidated responsibility.

### Mass diagram

Two rectangles per module — interface surface vs implementation. Before: interface nearly as tall as implementation (shallow). After: short interface, tall implementation (deep).

### Call-graph collapse

Before: a tree of function calls as nested boxes. After: the same tree collapsed into one box, internal calls faded inside it.

## Style

- Lean editorial, not corporate-dashboard. Generous whitespace. Serif optional for headings (`font-serif` with stone/slate).
- Colour sparingly: one accent (emerald or indigo) plus red for leakage and amber for warnings.
- Keep diagrams ~320px tall so before/after sits side by side without scrolling.
- Use `text-xs uppercase tracking-wider` for module labels inside diagrams.
- The only scripts are the Tailwind CDN and the Mermaid ESM import. Otherwise static.

## Top recommendation

One larger card. Candidate or finding name, one sentence on why, anchor link to its card.

## Tone

Plain English, concise — architectural nouns come from `vocabulary.md`.

**Use exactly:** module, interface, implementation, depth, deep, shallow, seam, adapter, leverage, locality.

**Never substitute:** component, service, unit (for module) · API, signature (for interface) · boundary (for seam) · layer, wrapper (for module, when you mean module).

**Phrasings that fit:**

- "Order intake module is shallow — interface nearly matches the implementation."
- "Pricing leaks across the seam."
- "Deepen: one interface, one place to test."
- "Two adapters justify the seam: HTTP in prod, in-memory in tests."

**Wins bullets** name the gain in glossary terms: *"locality: bugs concentrate in one module"*, *"leverage: one interface, N call sites"*, *"interface shrinks; implementation absorbs the wrappers"*. Don't write *"easier to maintain"* or *"cleaner code"*.

No hedging, no throat-clearing. If a sentence could be a bullet, make it a bullet. If a bullet could be cut, cut it.
