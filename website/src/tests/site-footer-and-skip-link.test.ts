import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const skipDir = join(srcDir, "components", "SkipLink");
const footerDir = join(srcDir, "components", "SiteFooter");
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

test("site footer and skip link", () => {
  expect(statSync(skipDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "SkipLink.tsx",
    "SkipLink.types.ts",
    "SkipLink.variants.ts",
  ]) {
    expect(statSync(join(skipDir, name)).isFile()).toBe(true);
  }

  expect(statSync(footerDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "SiteFooter.tsx",
    "SiteFooter.types.ts",
    "SiteFooter.variants.ts",
  ]) {
    expect(statSync(join(footerDir, name)).isFile()).toBe(true);
  }

  const skip = readFileSync(join(skipDir, "SkipLink.tsx"), "utf8");
  const skipVariants = readFileSync(join(skipDir, "SkipLink.variants.ts"), "utf8");
  const footer = readFileSync(join(footerDir, "SiteFooter.tsx"), "utf8");
  const footerVariants = readFileSync(
    join(footerDir, "SiteFooter.variants.ts"),
    "utf8",
  );
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");
  const home = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");

  expect(root).toContain("SkipLink");
  expect(root).toContain("SiteFooter");
  expect(root).toMatch(/<SkipLink\s*\/>/);
  expect(root).toMatch(/<SiteFooter\s*\/>/);
  expect(root).toMatch(/<main\b[^>]*id=["']main["']/);
  expect(root).toContain("tabIndex={-1}");
  expect(root).toContain("<Outlet");
  expect(root.indexOf("<SkipLink")).toBeLessThan(root.indexOf("<SiteHeader"));
  expect(root.indexOf("<SiteHeader")).toBeLessThan(root.indexOf("<main"));
  expect(root.indexOf("<main")).toBeLessThan(root.indexOf("<Outlet"));
  expect(root.indexOf("<Outlet")).toBeLessThan(root.indexOf("<SiteFooter"));
  expect(home).not.toContain("SkipLink");
  expect(home).not.toContain("SiteFooter");

  expect(skip).toMatch(/<a\s[^>]*href=["']#main["']/);
  expect(skip).toContain("Skip to content");
  expect(skip).toContain("skipLinkVariants");
  expect(skip).not.toMatch(/playground/i);
  expect(skip).not.toContain("docs/");

  expect(skipVariants).toContain("from \"class-variance-authority\"");
  expect(skipVariants).toContain("cva(");
  expect(skipVariants).toContain("sr-only");
  expect(skipVariants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(skipVariants).not.toMatch(/\bmax-w-sm\b/);

  expect(footer).toMatch(/<footer\b/);
  expect(footer).toContain("Draconic");
  expect(footer).toContain(`href="${githubUrl}"`);
  expect(footer).toContain(">GitHub<");
  expect(footer).toContain("siteFooterVariants");
  expect(footer).not.toMatch(/playground/i);
  expect(footer).not.toContain("docs/");
  expect(footer).not.toContain("Install");
  expect(footer).not.toContain("from JavaScript");
  expect(footer).not.toContain("from systems");
  expect(footer).not.toContain("Dual worlds");
  expect(footer).not.toContain("native types");
  expect(footer).not.toContain("host I/O");
  expect(footer).not.toContain(">Learn<");
  expect(footer).not.toContain(">Reference<");

  expect(footerVariants).toContain("from \"class-variance-authority\"");
  expect(footerVariants).toContain("cva(");
  expect(footerVariants).toContain("font-body");
  expect(footerVariants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(footerVariants).not.toMatch(/\bmax-w-sm\b/);

  expect(skip).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(skip).not.toMatch(/\bmax-w-sm\b/);
  expect(footer).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(footer).not.toMatch(/\bmax-w-sm\b/);
  expect(footer).not.toMatch(/bg-blue-500/);

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
