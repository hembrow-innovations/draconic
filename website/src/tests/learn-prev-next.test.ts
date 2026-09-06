import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { learnNeighbors } from "../features/learn/learnSequence";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const pagerDir = join(srcDir, "features", "learn", "LearnPager");
const sequencePath = join(srcDir, "features", "learn", "learnSequence.ts");
const pageDir = join(srcDir, "features", "learn", "LearnPage");
const routesDir = join(srcDir, "routes");

const install = { href: "/install", label: "Install" } as const;
const fromJavascript = {
  href: "/from-javascript",
  label: "from JavaScript",
} as const;
const fromSystems = { href: "/from-systems", label: "from systems" } as const;
const dualWorlds = { href: "/dual-worlds", label: "Dual worlds" } as const;
const modules = { href: "/modules", label: "modules" } as const;
const nativeTypes = { href: "/native-types", label: "native types" } as const;
const hostIo = { href: "/host-io", label: "host I/O" } as const;
const packages = { href: "/packages", label: "packages" } as const;

const learnChapters = [
  "install",
  "from-javascript",
  "from-systems",
  "dual-worlds",
  "modules",
  "native-types",
  "host-io",
  "packages",
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

test("learn prev next", () => {
  expect(statSync(sequencePath).isFile()).toBe(true);
  expect(statSync(pagerDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "LearnPager.tsx",
    "LearnPager.types.ts",
    "LearnPager.variants.ts",
  ]) {
    expect(statSync(join(pagerDir, name)).isFile()).toBe(true);
  }

  expect(learnNeighbors("install")).toEqual({
    prev: [],
    next: [fromJavascript, fromSystems],
  });
  expect(learnNeighbors("from-javascript")).toEqual({
    prev: [install],
    next: [dualWorlds],
  });
  expect(learnNeighbors("from-systems")).toEqual({
    prev: [install],
    next: [dualWorlds],
  });
  expect(learnNeighbors("dual-worlds")).toEqual({
    prev: [fromJavascript, fromSystems],
    next: [modules],
  });
  expect(learnNeighbors("modules")).toEqual({
    prev: [dualWorlds],
    next: [nativeTypes],
  });
  expect(learnNeighbors("native-types")).toEqual({
    prev: [modules],
    next: [hostIo],
  });
  expect(learnNeighbors("host-io")).toEqual({
    prev: [nativeTypes],
    next: [packages],
  });
  expect(learnNeighbors("packages")).toEqual({
    prev: [hostIo],
    next: [],
  });
  expect(learnNeighbors("learn")).toEqual({ prev: [], next: [] });

  expect(learnNeighbors("from-javascript").next).not.toEqual([fromSystems]);
  expect(learnNeighbors("from-javascript").next).not.toContainEqual(
    fromSystems,
  );
  expect(learnNeighbors("from-systems").prev).not.toContainEqual(
    fromJavascript,
  );
  expect(learnNeighbors("dual-worlds").prev).not.toEqual([fromSystems]);

  const extra = [
    "tutorial",
    "getting-started",
    "beginner",
    "vault",
    "playground",
    "reference",
  ];
  for (const slug of extra) {
    expect(learnNeighbors(slug)).toEqual({ prev: [], next: [] });
    expect(existsSync(join(routesDir, `${slug}.tsx`))).toBe(false);
  }

  const learnHub = readFileSync(join(websiteDir, "learn.md"), "utf8");
  expect(learnHub).toContain("[from JavaScript](from-javascript.html)");
  expect(learnHub).toContain("[from systems](from-systems.html)");
  expect(learnHub).toContain("[Dual worlds](dual-worlds.html)");
  expect(learnHub).toContain(
    "Those landings join at [Dual worlds](dual-worlds.html)",
  );

  const pageSource = readFileSync(join(pageDir, "LearnPage.tsx"), "utf8");
  const pager = readFileSync(join(pagerDir, "LearnPager.tsx"), "utf8");
  const variants = readFileSync(join(pagerDir, "LearnPager.variants.ts"), "utf8");
  const sequence = readFileSync(sequencePath, "utf8");
  const hub = readFileSync(join(routesDir, "learn.tsx"), "utf8");
  const shell = readFileSync(
    join(srcDir, "features", "docs", "DocsShell", "DocsShell.tsx"),
    "utf8",
  );

  expect(pageSource).toContain("LearnPager");
  expect(pageSource).toContain("page.slug");
  expect(pageSource).not.toMatch(/playground/i);
  expect(pageSource).not.toContain("docs/specs");
  expect(pageSource).not.toContain(".html");

  expect(hub).not.toContain("LearnPager");
  expect(shell).not.toContain("LearnPager");
  expect(shell).not.toContain("learnNeighbors");

  expect(pager).toContain("from \"@tanstack/react-router\"");
  expect(pager).toContain("Link");
  expect(pager).toContain("learnNeighbors");
  expect(pager).toContain("Previous");
  expect(pager).toContain("Next");
  expect(pager).toContain("aria-label");
  expect(pager).toContain("<nav");
  expect(pager).not.toContain(".html");
  expect(pager).not.toMatch(/playground/i);
  expect(pager).not.toContain("docs/");
  expect(pager).not.toContain("vault");
  expect(pager).not.toContain("tutorial");
  expect(pager).not.toContain("CLI");
  expect(pager).not.toContain("getting started");
  expect(pager).not.toContain("Reference");
  expect(pager).not.toContain("@ts-expect-error");

  expect(sequence).not.toContain("tutorial");
  expect(sequence).not.toContain("getting-started");
  expect(sequence).not.toContain("playground");
  expect(sequence).not.toContain("vault");
  expect(sequence).not.toContain(".html");

  expect(variants).toContain('from "class-variance-authority"');
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-body");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(pager).toContain("learnPager");
  expect(pager).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(pager).not.toMatch(/\bmax-w-sm\b/);
  expect(pager).not.toMatch(/bg-blue-500/);

  for (const slug of learnChapters) {
    const route = readFileSync(join(routesDir, `${slug}.tsx`), "utf8");
    expect(route).toContain("LearnPage");
  }

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
