import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const headerDir = join(srcDir, "components", "SiteHeader");
const skipDir = join(srcDir, "components", "SkipLink");
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

test("mobile a11y", () => {
  const header = readFileSync(join(headerDir, "SiteHeader.tsx"), "utf8");
  const variants = readFileSync(join(headerDir, "SiteHeader.variants.ts"), "utf8");
  const skip = readFileSync(join(skipDir, "SkipLink.tsx"), "utf8");
  const skipVariants = readFileSync(join(skipDir, "SkipLink.variants.ts"), "utf8");
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");

  expect(header).toContain("public-site.a11y:keyboard-small");
  expect(header).toContain('type="button"');
  expect(header).toContain("aria-expanded");
  expect(header).toContain("aria-controls");
  expect(header).toContain("useState");
  expect(header).toMatch(/onClick/);
  expect(header).toContain("Escape");
  expect(header).not.toMatch(/onMouseEnter/);
  expect(header).not.toMatch(/onMouseOver/);
  expect(header).toContain(">Menu<");
  expect(header).toMatch(/to=["']\/["']/);
  expect(header).toContain("Draconic");
  expect(header).toMatch(/to=["']\/learn["']/);
  expect(header).toContain(">Learn<");
  expect(header).toMatch(/to=["']\/reference["']/);
  expect(header).toContain(">Reference<");
  expect(header).toContain(`href="${githubUrl}"`);
  expect(header).toContain(">GitHub<");
  expect(header).not.toMatch(/playground/i);
  expect(header).not.toContain("docs/");

  expect(variants).toContain("siteHeaderMenuButtonVariants");
  expect(variants).toContain("md:hidden");
  expect(variants).toContain("hidden md:flex");
  expect(variants).toContain("focus-visible:ring-2");
  expect(variants).toContain("focus-visible:ring-accent");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(header).toContain("siteHeaderMenuButtonVariants");
  expect(header).toContain("siteHeaderClusterVariants");

  expect(root).toContain("SkipLink");
  expect(root).toMatch(/<SkipLink\s*\/>/);
  expect(root.indexOf("<SkipLink")).toBeLessThan(root.indexOf("<SiteHeader"));
  expect(skip).toMatch(/<a\s[^>]*href=["']#main["']/);
  expect(skip).toContain("Skip to content");
  expect(skipVariants).toContain("focus-visible:ring-2");
  expect(skipVariants).toContain("focus-visible:ring-accent");

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
