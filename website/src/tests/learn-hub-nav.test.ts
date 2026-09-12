import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { loadMarkdownPage, renderMarkdown } from "../lib/content";
import { assertCurrentPageInkContrast } from "./current-page-contrast";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const contentDir = join(websiteDir, "content");
const navDir = join(srcDir, "features", "learn", "LearnNav");
const cardsDir = join(srcDir, "features", "learn", "LearnHubCards");
const learnRoute = join(srcDir, "routes", "learn.tsx");

/** Hub order from `public-site.ia:learn-walkable` and `website/content/learn.md`. */
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

/** Accessible name is aria-label when present, otherwise the visible CONTEXT term. */
function linkAccessibleName(
  source: string,
  href: string,
  visibleLabel: string,
): string {
  const marker = `to="${href}"`;
  const start = source.indexOf(marker);
  expect(start, href).toBeGreaterThan(-1);
  const after = source.slice(start);
  const close = after.indexOf(`>${visibleLabel}<`);
  expect(close, `${href} ${visibleLabel}`).toBeGreaterThan(-1);
  const opening = after.slice(0, close);
  const aria = /aria-label="([^"]*)"/.exec(opening);
  return aria?.[1] ?? visibleLabel;
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
  expect(statSync(cardsDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "LearnHubCards.tsx",
    "LearnHubCards.types.ts",
    "LearnHubCards.variants.ts",
  ]) {
    expect(statSync(join(cardsDir, name)).isFile()).toBe(true);
  }

  const route = readFileSync(learnRoute, "utf8");
  const nav = readFileSync(join(navDir, "LearnNav.tsx"), "utf8");
  const referenceNav = readFileSync(
    join(srcDir, "features", "reference", "ReferenceNav", "ReferenceNav.tsx"),
    "utf8",
  );
  const variants = readFileSync(join(navDir, "LearnNav.variants.ts"), "utf8");
  const cards = readFileSync(join(cardsDir, "LearnHubCards.tsx"), "utf8");
  const cardVariants = readFileSync(
    join(cardsDir, "LearnHubCards.variants.ts"),
    "utf8",
  );
  const learnPage = readFileSync(
    join(srcDir, "features", "learn", "LearnPage", "LearnPage.tsx"),
    "utf8",
  );
  const home = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");
  const header = readFileSync(
    join(srcDir, "components", "SiteHeader", "SiteHeader.tsx"),
    "utf8",
  );
  const shell = readFileSync(
    join(srcDir, "features", "docs", "DocsShell", "DocsShell.tsx"),
    "utf8",
  );
  const learnHub = readFileSync(join(contentDir, "learn.md"), "utf8");
  const routeTree = readFileSync(join(srcDir, "routeTree.gen.ts"), "utf8");
  const page = loadMarkdownPage("learn");

  expect(route).toContain("createFileRoute");
  expect(route).toContain('"/learn"');
  expect(route).toContain("DocsShell");
  expect(route).toContain("LearnNav");
  expect(route).toContain("LearnHubCards");
  expect(route).toContain('loadMarkdownPage("learn")');
  expect(route).toContain("body");
  expect(route).not.toContain("HomeHero");
  expect(route).not.toContain("HomeFeatures");
  expect(route).not.toMatch(/playground/i);
  expect(route).not.toContain("docs/specs");
  expect(route).not.toContain(".html");
  expect(routeTree).toContain("./routes/learn");
  expect(routeTree).toContain("'/learn'");

  expect(home).not.toContain("DocsShell");
  expect(home).not.toContain("LearnNav");
  expect(home).not.toContain("LearnHubCards");
  expect(learnPage).not.toContain("LearnHubCards");
  expect(root).not.toContain("LearnNav");
  expect(header).toContain("LearnNav");
  expect(header).toContain('from "../../features/learn/LearnNav"');
  assertOrder(header, [
    'to="/"',
    ">GitHub<",
    'to="/learn"',
    "LearnNav",
    'to="/reference"',
  ]);
  expect(shell).not.toContain("from JavaScript");
  expect(shell).not.toContain("from systems");
  expect(shell).not.toContain("Dual worlds");

  expect(nav).toContain("public-site.chrome:current-page");
  expect(nav).toContain("public-site.a11y:distinct-nav-names");
  expect(nav).toContain("from \"@tanstack/react-router\"");
  expect(nav).toContain("Link");
  expect(nav).toContain("activeProps");
  expect(nav).toContain('"aria-current": "page"');
  expect(variants).toContain("aria-[current=page]:text-accent-2");
  expect(variants).toContain("text-muted");
  expect(variants).not.toMatch(/aria-\[current=page\]:text-muted/);
  const theme = readFileSync(join(srcDir, "styles", "theme.css"), "utf8");
  assertCurrentPageInkContrast(theme, variants);
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
  assertOrder(
    cards,
    learnPath.flatMap((chapter) => [chapter.href, `>${chapter.label}<`]),
  );

  const learnHostIo = linkAccessibleName(nav, "/host-io", "host I/O");
  const referenceHostIo = linkAccessibleName(
    referenceNav,
    "/reference-host-io",
    "host I/O",
  );
  const learnPackages = linkAccessibleName(nav, "/packages", "packages");
  const referencePackages = linkAccessibleName(
    referenceNav,
    "/reference-packages",
    "packages",
  );
  expect(learnHostIo).not.toBe(referenceHostIo);
  expect(learnPackages).not.toBe(referencePackages);
  expect(learnHostIo).toBe("Learn · host I/O");
  expect(referenceHostIo).toBe("Reference · host I/O");
  expect(learnPackages).toBe("Learn · packages");
  expect(referencePackages).toBe("Reference · packages");

  const fromJs = nav.indexOf("/from-javascript");
  const fromSystems = nav.indexOf("/from-systems");
  const dualWorlds = nav.indexOf("/dual-worlds");
  expect(fromJs).toBeGreaterThan(-1);
  expect(fromSystems).toBeGreaterThan(-1);
  expect(dualWorlds).toBeGreaterThan(fromJs);
  expect(dualWorlds).toBeGreaterThan(fromSystems);

  const cardsFromJs = cards.indexOf("/from-javascript");
  const cardsFromSystems = cards.indexOf("/from-systems");
  const cardsDualWorlds = cards.indexOf("/dual-worlds");
  expect(cardsFromJs).toBeGreaterThan(-1);
  expect(cardsFromSystems).toBeGreaterThan(-1);
  expect(cardsDualWorlds).toBeGreaterThan(cardsFromJs);
  expect(cardsDualWorlds).toBeGreaterThan(cardsFromSystems);

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
  expect(variants).toContain("learnNavLinkVariants");
  expect(variants).toContain("focus-visible:ring-2");
  expect(variants).toContain("focus-visible:ring-accent");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(nav).toContain("learnNav");
  expect(nav).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(nav).not.toMatch(/\bmax-w-sm\b/);
  expect(nav).not.toMatch(/bg-blue-500/);
  expect(route).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(route).not.toMatch(/\bmax-w-sm\b/);

  expect(cards).toContain("public-site.ia:learn-walkable");
  expect(cards).toContain("from \"@tanstack/react-router\"");
  expect(cards).toContain("Link");
  expect(cards).toContain("<div");
  expect(cards).toContain("<h3>");
  expect(cards).toContain("learnHubCardsVariants");
  expect(cards).toContain("learnHubCardVariants");
  expect(cards.match(/<div className=\{learnHubCardVariants/g)?.length).toBe(
    learnPath.length,
  );
  expect(cards.match(/<h3>/g)?.length).toBe(learnPath.length);
  expect(cards).not.toContain(".html");
  expect(cards).not.toMatch(/playground/i);
  expect(cards).not.toContain("docs/");
  expect(cards).not.toContain("vault");
  expect(cards).not.toContain("tutorial");
  expect(cards).not.toContain("CLI");
  expect(cards).not.toContain("getting started");
  expect(cards).not.toContain("Reference");
  expect(cards).not.toContain("HomeFeatures");
  expect(cards).not.toContain("HomeHero");
  expect(cards).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(cards).not.toMatch(/\bmax-w-sm\b/);
  expect(cards).not.toMatch(/bg-blue-500/);

  expect(cardVariants).toContain('from "class-variance-authority"');
  expect(cardVariants).toContain("cva(");
  expect(cardVariants).toContain("grid");
  expect(cardVariants).toContain("card");
  expect(cardVariants).toContain("font-body");
  expect(cardVariants).not.toContain("grid-cols-3");
  expect(cardVariants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(cardVariants).not.toMatch(/\bmax-w-sm\b/);

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
