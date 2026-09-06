import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { loadMarkdownPage, renderMarkdown } from "../lib/content";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const routesDir = join(srcDir, "routes");
const navDir = join(srcDir, "features", "reference", "ReferenceNav");
const pageDir = join(srcDir, "features", "reference", "ReferencePage");
const referenceRoute = join(routesDir, "reference.tsx");

/** Hub order from `public-site.ia:reference-walkable` and `website/reference.md`. */
const referencePath = [
  { href: "/cli", label: "CLI" },
  { href: "/types", label: "types" },
  { href: "/dual-world-rules", label: "Dual-world rules" },
  { href: "/reference-host-io", label: "host I/O" },
  { href: "/reference-packages", label: "packages" },
] as const;

/** Working pages for `public-site.ia:reference-walkable`. Hub is already `/reference`. */
const referencePages = [
  {
    slug: "cli",
    title: "CLI",
    status: "shipped",
    copy: "A Program is the unit of source it accepts.",
  },
  {
    slug: "types",
    title: "types",
    status: "not-yet",
    copy: "The Checker is not tsc: it does not compile existing TypeScript projects",
  },
  {
    slug: "dual-world-rules",
    title: "Dual-world rules",
    status: "not-yet",
    copy: "with explicit boundaries at the type and lowering level.",
  },
  {
    slug: "reference-host-io",
    title: "host I/O",
    status: "not-yet",
    copy: "This page is lookup. The designed lesson is",
  },
  {
    slug: "reference-packages",
    title: "packages",
    status: "not-yet",
    copy: "This page is lookup, not the Learn chapter.",
  },
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

test("reference hub pages", () => {
  expect(statSync(navDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "ReferenceNav.tsx",
    "ReferenceNav.types.ts",
    "ReferenceNav.variants.ts",
  ]) {
    expect(statSync(join(navDir, name)).isFile()).toBe(true);
  }
  expect(statSync(pageDir).isDirectory()).toBe(true);
  for (const name of ["index.ts", "ReferencePage.tsx", "ReferencePage.types.ts"]) {
    expect(statSync(join(pageDir, name)).isFile()).toBe(true);
  }
  expect(statSync(referenceRoute).isFile()).toBe(true);
  expect(statSync(join(websiteDir, "generate.drac")).isFile()).toBe(true);

  const route = readFileSync(referenceRoute, "utf8");
  const nav = readFileSync(join(navDir, "ReferenceNav.tsx"), "utf8");
  const variants = readFileSync(join(navDir, "ReferenceNav.variants.ts"), "utf8");
  const pageSource = readFileSync(join(pageDir, "ReferencePage.tsx"), "utf8");
  const home = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");
  const shell = readFileSync(
    join(srcDir, "features", "docs", "DocsShell", "DocsShell.tsx"),
    "utf8",
  );
  const learnNav = readFileSync(
    join(srcDir, "features", "learn", "LearnNav", "LearnNav.tsx"),
    "utf8",
  );
  const referenceHub = readFileSync(join(websiteDir, "reference.md"), "utf8");
  const routeTree = readFileSync(join(srcDir, "routeTree.gen.ts"), "utf8");
  const page = loadMarkdownPage("reference");

  expect(route).toContain("createFileRoute");
  expect(route).toContain('"/reference"');
  expect(route).toContain("DocsShell");
  expect(route).toContain("ReferenceNav");
  expect(route).toContain('loadMarkdownPage("reference")');
  expect(route).toContain("body");
  expect(route).not.toContain("HomeHero");
  expect(route).not.toContain("LearnNav");
  expect(route).not.toContain("LearnPage");
  expect(route).not.toMatch(/playground/i);
  expect(route).not.toContain("docs/specs");
  expect(route).not.toContain("api-cli");
  expect(route).not.toContain(".html");
  expect(routeTree).toContain("./routes/reference");
  expect(routeTree).toContain("'/reference'");

  expect(home).not.toContain("DocsShell");
  expect(home).not.toContain("ReferenceNav");
  expect(root).not.toContain("ReferenceNav");
  expect(shell).not.toContain("Dual-world rules");
  expect(shell).not.toContain("api-cli");

  expect(nav).toContain("from \"@tanstack/react-router\"");
  expect(nav).toContain("Link");
  expect(nav).not.toContain(".html");
  expect(nav).not.toMatch(/playground/i);
  expect(nav).not.toContain("docs/");
  expect(nav).not.toContain("vault");
  expect(nav).not.toContain("api-cli");
  expect(nav).not.toContain("Learn");
  expect(nav).not.toContain("Install");
  expect(nav).not.toContain("from JavaScript");
  expect(nav).not.toContain("from systems");
  expect(nav).not.toContain("Dual worlds");
  expect(nav).not.toContain("/host-io");
  expect(nav).not.toContain('to="/packages"');
  expect(nav).not.toContain("@ts-expect-error");

  expect(learnNav).not.toContain("/cli");
  expect(learnNav).not.toContain("/types");
  expect(learnNav).not.toContain("/dual-world-rules");
  expect(learnNav).not.toContain("/reference-host-io");
  expect(learnNav).not.toContain("/reference-packages");

  assertOrder(
    referenceHub,
    referencePath.map((entry) => `[${entry.label}]`),
  );
  assertOrder(
    nav,
    referencePath.flatMap((entry) => [entry.href, `>${entry.label}<`]),
  );

  expect(page.title).toBe("Reference");
  expect(page.section).toBe("reference");
  expect(page.status).toBe("shipped");
  expect(page.body).toContain("# Reference");
  expect(page.body).toContain(
    "Working pages kept open while writing a Program. Not a generated API dump, and not the Learn path.",
  );
  expect(page.body).toContain("[CLI](cli.html)");
  expect(page.body).toContain("[types](types.html)");
  expect(page.body).toContain("[Dual-world rules](dual-world-rules.html)");
  expect(page.body).toContain("[host I/O](reference-host-io.html)");
  expect(page.body).toContain("[packages](reference-packages.html)");
  expect(renderMarkdown(page.body)).toContain("<h1>Reference</h1>");
  expect(renderMarkdown(page.body)).toContain('<a href="/cli">CLI</a>');
  expect(renderMarkdown(page.body)).toContain(
    '<a href="/reference-host-io">host I/O</a>',
  );
  expect(renderMarkdown(page.body)).toContain(
    '<a href="/reference-packages">packages</a>',
  );
  expect(renderMarkdown(page.body)).not.toMatch(/href="[^"]*\.html"/);

  expect(referenceHub).toContain("# Reference");
  expect(referenceHub).toContain("[CLI](cli.html)");
  expect(referenceHub).toContain("[Learn](learn.html)");

  expect(variants).toContain('from "class-variance-authority"');
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-body");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(nav).toContain("referenceNav");
  expect(nav).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(nav).not.toMatch(/\bmax-w-sm\b/);
  expect(nav).not.toMatch(/bg-blue-500/);
  expect(route).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(route).not.toMatch(/\bmax-w-sm\b/);

  expect(pageSource).toContain("DocsShell");
  expect(pageSource).toContain("ReferenceNav");
  expect(pageSource).toContain("body");
  expect(pageSource).toContain("shipped");
  expect(pageSource).toContain("not-yet");
  expect(pageSource).not.toContain("LearnNav");
  expect(pageSource).not.toContain("LearnPager");
  expect(pageSource).not.toMatch(/playground/i);
  expect(pageSource).not.toContain("docs/specs");
  expect(pageSource).not.toContain("api-cli");
  expect(pageSource).not.toContain(".html");
  expect(pageSource).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(pageSource).not.toMatch(/\bmax-w-sm\b/);

  for (const working of referencePages) {
    const routePath = join(routesDir, `${working.slug}.tsx`);
    expect(statSync(routePath).isFile()).toBe(true);
    const workingRoute = readFileSync(routePath, "utf8");
    const source = readFileSync(join(websiteDir, `${working.slug}.md`), "utf8");
    const workingPage = loadMarkdownPage(working.slug);
    const html = renderMarkdown(workingPage.body);

    expect(workingRoute).toContain("createFileRoute");
    expect(workingRoute).toContain(`"/${working.slug}"`);
    expect(workingRoute).toContain(`loadMarkdownPage("${working.slug}")`);
    expect(workingRoute).toContain("ReferencePage");
    expect(workingRoute).not.toContain("LearnPage");
    expect(workingRoute).not.toContain("HomeHero");
    expect(workingRoute).not.toMatch(/playground/i);
    expect(workingRoute).not.toContain("docs/specs");
    expect(workingRoute).not.toContain("api-cli");
    expect(workingRoute).not.toContain(".html");
    expect(workingRoute).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(workingRoute).not.toMatch(/\bmax-w-sm\b/);

    expect(routeTree).toContain(`./routes/${working.slug}`);
    expect(routeTree).toContain(`'/${working.slug}'`);

    expect(workingPage.title).toBe(working.title);
    expect(workingPage.section).toBe("reference");
    expect(workingPage.status).toBe(working.status);
    expect(workingPage.body).toContain(`# ${working.title}`);
    expect(workingPage.body).toContain(working.copy);
    expect(source).toContain(working.copy);
    expect(source).toContain(`status: ${working.status}`);

    expect(html).toContain(`<h1>${working.title}</h1>`);
    expect(html).toContain(working.copy);
    expect(html).not.toMatch(/href="[^"]*\.html"/);
    expect(html).not.toMatch(/playground/i);
    expect(html).not.toContain("docs/");
    expect(html).not.toContain("api-cli");
  }

  const cli = renderMarkdown(loadMarkdownPage("cli").body);
  const types = renderMarkdown(loadMarkdownPage("types").body);
  const dualWorldRules = renderMarkdown(loadMarkdownPage("dual-world-rules").body);
  const hostIo = renderMarkdown(loadMarkdownPage("reference-host-io").body);
  const packages = renderMarkdown(loadMarkdownPage("reference-packages").body);

  expect(cli).toContain('<a href="/reference-packages">packages</a>');
  expect(cli).toContain('<a href="/types">types</a>');
  expect(cli).toContain('<a href="/dual-world-rules">Dual-world rules</a>');
  expect(types).toContain('<a href="/dual-world-rules">Dual-world rules</a>');
  expect(types).toContain('<a href="/from-javascript">from JavaScript</a>');
  expect(types).toContain('<a href="/dual-worlds">Dual worlds</a>');
  expect(types).toContain('<a href="/native-types">native types</a>');
  expect(dualWorldRules).toContain('<a href="/dual-worlds">Dual worlds</a>');
  expect(dualWorldRules).toContain('<a href="/types">types</a>');
  expect(hostIo).toContain('<a href="/host-io">host I/O</a>');
  expect(hostIo).toContain('<a href="/cli">CLI</a>');
  expect(packages).toContain('<a href="/packages">packages</a>');
  expect(packages).toContain('<a href="/cli">CLI</a>');

  const extra = ["api-cli", "tutorial", "getting-started", "beginner", "vault"];
  for (const slug of extra) {
    expect(existsSync(join(routesDir, `${slug}.tsx`))).toBe(false);
  }

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
