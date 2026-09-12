import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { referenceNeighbors } from "../features/reference/referenceSequence";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const contentDir = join(websiteDir, "content");
const pagerDir = join(srcDir, "features", "reference", "ReferencePager");
const sequencePath = join(
  srcDir,
  "features",
  "reference",
  "referenceSequence.ts",
);
const pageDir = join(srcDir, "features", "reference", "ReferencePage");
const routesDir = join(srcDir, "routes");
const learnPagerPath = join(
  srcDir,
  "features",
  "learn",
  "LearnPager",
  "LearnPager.tsx",
);

const cli = { href: "/cli", label: "CLI" } as const;
const types = { href: "/types", label: "types" } as const;
const dualWorldRules = {
  href: "/dual-world-rules",
  label: "Dual-world rules",
} as const;
const hostIo = { href: "/reference-host-io", label: "host I/O" } as const;
const packages = { href: "/reference-packages", label: "packages" } as const;

const referencePages = [
  "cli",
  "types",
  "dual-world-rules",
  "reference-host-io",
  "reference-packages",
] as const;

function walkProductFiles(dir: string): string[] {
  const out: string[] = [];
  for (const name of readdirSync(dir)) {
    const path = join(dir, name);
    if (statSync(path).isDirectory()) {
      if (name === "tests") {
        continue;
      }
      out.push(...walkProductFiles(path));
      continue;
    }
    if (name.endsWith(".gen.ts") || name === "routeTree.gen.ts") {
      continue;
    }
    if (name.endsWith(".tsx") || name.endsWith(".ts")) {
      out.push(path);
    }
  }
  return out;
}

test("reference prev next", () => {
  expect(statSync(sequencePath).isFile()).toBe(true);
  expect(statSync(pagerDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "ReferencePager.tsx",
    "ReferencePager.types.ts",
    "ReferencePager.variants.ts",
  ]) {
    expect(statSync(join(pagerDir, name)).isFile()).toBe(true);
  }

  expect(referenceNeighbors("cli")).toEqual({
    prev: [],
    next: [types],
  });
  expect(referenceNeighbors("types")).toEqual({
    prev: [cli],
    next: [dualWorldRules],
  });
  expect(referenceNeighbors("dual-world-rules")).toEqual({
    prev: [types],
    next: [hostIo],
  });
  expect(referenceNeighbors("reference-host-io")).toEqual({
    prev: [dualWorldRules],
    next: [packages],
  });
  expect(referenceNeighbors("reference-packages")).toEqual({
    prev: [hostIo],
    next: [],
  });
  expect(referenceNeighbors("reference")).toEqual({ prev: [], next: [] });

  expect(referenceNeighbors("cli").prev).toEqual([]);
  expect(referenceNeighbors("reference-packages").next).toEqual([]);
  expect(referenceNeighbors("cli").next).not.toContainEqual(packages);
  expect(referenceNeighbors("reference-packages").prev).not.toContainEqual(cli);

  const extra = [
    "learn",
    "install",
    "tutorial",
    "getting-started",
    "vault",
    "playground",
  ];
  for (const slug of extra) {
    expect(referenceNeighbors(slug)).toEqual({ prev: [], next: [] });
  }
  expect(existsSync(join(routesDir, "playground.tsx"))).toBe(false);

  const referenceHub = readFileSync(join(contentDir, "reference.md"), "utf8");
  expect(referenceHub).toContain("[CLI](cli.html)");
  expect(referenceHub).toContain("[types](types.html)");
  expect(referenceHub).toContain("[Dual-world rules](dual-world-rules.html)");
  expect(referenceHub).toContain("[host I/O](reference-host-io.html)");
  expect(referenceHub).toContain("[packages](reference-packages.html)");

  const pageSource = readFileSync(join(pageDir, "ReferencePage.tsx"), "utf8");
  const pager = readFileSync(join(pagerDir, "ReferencePager.tsx"), "utf8");
  const variants = readFileSync(
    join(pagerDir, "ReferencePager.variants.ts"),
    "utf8",
  );
  const sequence = readFileSync(sequencePath, "utf8");
  const hub = readFileSync(join(routesDir, "reference.tsx"), "utf8");
  const shell = readFileSync(
    join(srcDir, "features", "docs", "DocsShell", "DocsShell.tsx"),
    "utf8",
  );
  const learnPager = readFileSync(learnPagerPath, "utf8");

  expect(pageSource).toContain("ReferencePager");
  expect(pageSource).toContain("page.slug");
  expect(pageSource).not.toContain("LearnPager");
  expect(pageSource).not.toMatch(/playground/i);
  expect(pageSource).not.toContain("docs/specs");
  expect(pageSource).not.toContain(".html");

  expect(hub).not.toContain("ReferencePager");
  expect(hub).toContain("ReferenceHubCards");
  expect(shell).not.toContain("ReferencePager");
  expect(shell).not.toContain("referenceNeighbors");
  expect(shell).not.toContain("LearnPager");

  expect(learnPager).toContain("learnNeighbors");
  expect(learnPager).toContain("Learn sequence");
  expect(learnPager).not.toContain("referenceNeighbors");
  expect(learnPager).not.toContain("ReferencePager");
  expect(learnPager).not.toContain("dual-world-rules");

  expect(pager).toContain("from \"@tanstack/react-router\"");
  expect(pager).toContain("Link");
  expect(pager).toContain("referenceNeighbors");
  expect(pager).toContain("Previous");
  expect(pager).toContain("Next");
  expect(pager).toContain("aria-label");
  expect(pager).toContain("Reference sequence");
  expect(pager).toContain("<nav");
  expect(pager).not.toContain(".html");
  expect(pager).not.toMatch(/playground/i);
  expect(pager).not.toContain("docs/");
  expect(pager).not.toContain("vault");
  expect(pager).not.toContain("tutorial");
  expect(pager).not.toContain("LearnPager");
  expect(pager).not.toContain("learnNeighbors");
  expect(pager).not.toContain("@ts-expect-error");

  expect(sequence).not.toContain("tutorial");
  expect(sequence).not.toContain("getting-started");
  expect(sequence).not.toContain("playground");
  expect(sequence).not.toContain("vault");
  expect(sequence).not.toContain(".html");
  expect(sequence).not.toContain("/install");
  expect(sequence).not.toContain("/from-javascript");

  expect(variants).toContain('from "class-variance-authority"');
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-body");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(pager).toContain("referencePager");
  expect(pager).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(pager).not.toMatch(/\bmax-w-sm\b/);
  expect(pager).not.toMatch(/bg-blue-500/);

  for (const slug of referencePages) {
    const route = readFileSync(join(routesDir, `${slug}.tsx`), "utf8");
    expect(route).toContain("ReferencePage");
  }

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
