import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const headerDir = join(srcDir, "components", "SiteHeader");
const githubUrl = "https://github.com/hembrow-innovations/draconic";

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

test("site header primary nav", () => {
  expect(statSync(headerDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "SiteHeader.tsx",
    "SiteHeader.types.ts",
    "SiteHeader.variants.ts",
  ]) {
    expect(statSync(join(headerDir, name)).isFile()).toBe(true);
  }

  const header = readFileSync(join(headerDir, "SiteHeader.tsx"), "utf8");
  const variants = readFileSync(join(headerDir, "SiteHeader.variants.ts"), "utf8");
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");
  const home = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");
  const learn = readFileSync(join(srcDir, "routes", "learn.tsx"), "utf8");

  expect(header).toContain("public-site.chrome:primary-nav");
  expect(header).toContain("public-site.chrome:odm-shell");
  expect(header).toContain("public-site.chrome:current-page");
  expect(header).toContain("public-site.search:titles-headings");

  expect(root).toContain("public-site.chrome:favicon");
  expect(root).toMatch(/rel:\s*["']icon["']/);
  expect(root).toMatch(/href:\s*["']\/favicon\.ico["']/);
  const favicon = readFileSync(join(websiteDir, "public", "favicon.ico"));
  expect(favicon.subarray(0, 4)).toEqual(Buffer.from([0, 0, 1, 0]));
  expect(favicon.byteLength).toBeGreaterThan(16);

  expect(root).toContain("SiteHeader");
  expect(root).toMatch(/<SiteHeader\s*\/>/);
  expect(root).toContain("siteShellVariants");
  expect(root).toContain("siteMainVariants");
  expect(root).toContain("<Outlet");
  expect(root).toMatch(/<main\b[^>]*id=["']main["']/);
  expect(root.indexOf("<SkipLink")).toBeLessThan(
    root.indexOf("className={siteShellVariants()}"),
  );
  expect(root.indexOf("className={siteShellVariants()}")).toBeLessThan(
    root.indexOf("<SiteHeader"),
  );
  expect(root.indexOf("<SiteHeader")).toBeLessThan(root.indexOf("<main"));
  expect(root.indexOf("<main")).toBeLessThan(root.indexOf("<Outlet"));
  expect(home).not.toContain("SiteHeader");
  expect(home).not.toContain("DocsShell");
  expect(learn).not.toContain("SiteHeader");
  expect(root).not.toContain("DocsShell");

  expect(header).toContain("<aside");
  expect(header).not.toMatch(/<header\b/);
  expect(header).toContain("from \"@tanstack/react-router\"");
  expect(header).toContain("Link");
  expect(header).toMatch(/to=["']\/["']/);
  expect(header).toContain("Draconic");
  expect(header).toContain("One language, two backends.");
  expect(header).toMatch(/to=["']\/learn["']/);
  expect(header).toContain(">Learn<");
  expect(header).toMatch(/to=["']\/reference["']/);
  expect(header).toContain(">Reference<");
  expect(header).toContain(`href="${githubUrl}"`);
  expect(header).toContain(">GitHub<");
  expect(header).toMatch(/<a\s[^>]*href="https:\/\/github.com\/hembrow-innovations\/draconic"/);
  expect(header).toContain('"aria-current": "page"');
  expect(variants).toContain("aria-[current=page]:text-accent-2");
  expect(variants).toContain("text-muted");
  expect(variants).not.toMatch(/aria-\[current=page\]:text-muted/);
  expect(
    variants.match(/aria-\[current=page\]:text-accent-2/g)?.length,
  ).toBeGreaterThanOrEqual(2);
  expect(header).toContain("<SiteSearch");
  expect(header).toContain("<ThemeToggle");
  expect(header.indexOf("<SiteSearch")).toBeLessThan(header.indexOf("<LearnNav"));
  expect(header.indexOf("<SiteSearch")).toBeLessThan(
    header.indexOf("<ReferenceNav"),
  );
  expect(header.indexOf("<ThemeToggle")).toBeLessThan(header.indexOf("<LearnNav"));
  expect(header.indexOf("<ThemeToggle")).toBeLessThan(
    header.indexOf("<ReferenceNav"),
  );

  expect(header).not.toMatch(/playground/i);
  expect(header).not.toContain("docs/");
  expect(header).not.toContain("Install");
  expect(header).not.toContain("from JavaScript");
  expect(header).not.toContain("from systems");
  expect(header).not.toContain("Dual worlds");
  expect(header).not.toContain("native types");
  expect(header).not.toContain("host I/O");

  expect(variants).toContain("from \"class-variance-authority\"");
  expect(variants).toContain("cva(");
  expect(variants).toContain("siteShellVariants");
  expect(variants).toContain("siteMainVariants");
  expect(variants).toContain("font-display");
  expect(variants).toContain("font-body");
  expect(variants).toContain("sticky");
  expect(variants).toMatch(/\bside\b/);
  expect(variants).toMatch(/\bshell\b/);
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(header).toContain("siteHeaderVariants");
  expect(header).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(header).not.toMatch(/\bmax-w-sm\b/);
  expect(header).not.toMatch(/bg-blue-500/);

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
