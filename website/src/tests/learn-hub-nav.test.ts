import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { loadMarkdownPage, renderMarkdown } from "../lib/content";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const navDir = join(srcDir, "features", "learn", "LearnNav");
const learnRoute = join(srcDir, "routes", "learn.tsx");

/** Hub order from `public-site.ia:learn-walkable` and `website/learn.md`. */
const learnPath = [
  { href: "/install", label: "Install" },
  { href: "/from-javascript", label: "from JavaScript" },
  { href: "/from-systems", label: "from systems" },
  { href: "/dual-worlds", label: "Dual worlds" },
  { href: "/modules", label: "modules" },
  { href: "/native-types", label: "native types" },
  { href: "/host-io", label: "host I/O" },
  { href: "/packages", label: "packages" },
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

function assertOrder(source: string, needles: readonly string[]): void {
  let cursor = -1;
  for (const needle of needles) {
    const at = source.indexOf(needle, cursor + 1);
    expect(at, needle).toBeGreaterThan(cursor);
    cursor = at;
  }
}

test("learn hub nav", () => {
  expect(statSync(navDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "LearnNav.tsx",
    "LearnNav.types.ts",
    "LearnNav.variants.ts",
  ]) {
    expect(statSync(join(navDir, name)).isFile()).toBe(true);
  }
  expect(statSync(learnRoute).isFile()).toBe(true);

  const route = readFileSync(learnRoute, "utf8");
  const nav = readFileSync(join(navDir, "LearnNav.tsx"), "utf8");
  const variants = readFileSync(join(navDir, "LearnNav.variants.ts"), "utf8");
  const home = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");
  const shell = readFileSync(
    join(srcDir, "features", "docs", "DocsShell", "DocsShell.tsx"),
    "utf8",
  );
  const learnHub = readFileSync(join(websiteDir, "learn.md"), "utf8");
  const routeTree = readFileSync(join(srcDir, "routeTree.gen.ts"), "utf8");
  const page = loadMarkdownPage("learn");

  expect(route).toContain("createFileRoute");
  expect(route).toContain('"/learn"');
  expect(route).toContain("DocsShell");
  expect(route).toContain("LearnNav");
  expect(route).toContain('loadMarkdownPage("learn")');
  expect(route).toContain("body");
  expect(route).not.toContain("HomeHero");
  expect(route).not.toMatch(/playground/i);
  expect(route).not.toContain("docs/specs");
  expect(route).not.toContain(".html");
  expect(routeTree).toContain("./routes/learn");
  expect(routeTree).toContain("'/learn'");

  expect(home).not.toContain("DocsShell");
  expect(home).not.toContain("LearnNav");
  expect(root).not.toContain("LearnNav");
  expect(shell).not.toContain("from JavaScript");
  expect(shell).not.toContain("from systems");
  expect(shell).not.toContain("Dual worlds");

  expect(nav).toContain("from \"@tanstack/react-router\"");
  expect(nav).toContain("Link");
  expect(nav).not.toContain(".html");
  expect(nav).not.toMatch(/playground/i);
  expect(nav).not.toContain("docs/");
  expect(nav).not.toContain("vault");
  expect(nav).not.toContain("tutorial");
  expect(nav).not.toContain("CLI");
  expect(nav).not.toContain("getting started");
  expect(nav).not.toContain("Reference");

  assertOrder(
    learnHub,
    learnPath.map((chapter) => `[${chapter.label}]`),
  );
  assertOrder(
    nav,
    learnPath.flatMap((chapter) => [chapter.href, `>${chapter.label}<`]),
  );

  const fromJs = nav.indexOf("/from-javascript");
  const fromSystems = nav.indexOf("/from-systems");
  const dualWorlds = nav.indexOf("/dual-worlds");
  expect(fromJs).toBeGreaterThan(-1);
  expect(fromSystems).toBeGreaterThan(-1);
  expect(dualWorlds).toBeGreaterThan(fromJs);
  expect(dualWorlds).toBeGreaterThan(fromSystems);

  expect(page.title).toBe("Learn");
  expect(page.section).toBe("learn");
  expect(page.status).toBe("shipped");
  expect(page.body).toContain("# Learn");
  expect(page.body).toContain(
    "JavaScript you already know. Native types when you need them. One language, two backends.",
  );
  expect(page.body).toContain(
    "It is not a beginner programming course.",
  );
  expect(page.body).toContain("Start at [Install](install.html)");
  expect(renderMarkdown(page.body)).toContain("<h1>Learn</h1>");

  expect(learnHub).toContain("# Learn");
  expect(learnHub).toContain("Start at [Install](install.html)");
  expect(learnHub).toContain("[from JavaScript](from-javascript.html)");
  expect(learnHub).toContain("[from systems](from-systems.html)");
  expect(learnHub).toContain("[Dual worlds](dual-worlds.html)");

  expect(variants).toContain('from "class-variance-authority"');
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-body");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(nav).toContain("learnNav");
  expect(nav).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(nav).not.toMatch(/\bmax-w-sm\b/);
  expect(nav).not.toMatch(/bg-blue-500/);
  expect(route).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(route).not.toMatch(/\bmax-w-sm\b/);

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
