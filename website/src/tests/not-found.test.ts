import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const notFoundDir = join(srcDir, "components", "NotFound");
const routesDir = join(srcDir, "routes");

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

test("not found", () => {
  expect(statSync(notFoundDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "NotFound.tsx",
    "NotFound.types.ts",
    "NotFound.variants.ts",
  ]) {
    expect(statSync(join(notFoundDir, name)).isFile()).toBe(true);
  }

  const notFound = readFileSync(join(notFoundDir, "NotFound.tsx"), "utf8");
  const variants = readFileSync(
    join(notFoundDir, "NotFound.variants.ts"),
    "utf8",
  );
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");
  const header = readFileSync(
    join(srcDir, "components", "SiteHeader", "SiteHeader.tsx"),
    "utf8",
  );

  expect(root).toContain("public-site.chrome:odm-shell");
  expect(root).toContain("public-site.chrome:not-found");
  expect(root).toContain("notFoundComponent");
  expect(root).toContain("NotFound");
  expect(root).toMatch(/notFoundComponent:\s*NotFound/);
  expect(root).toContain("SkipLink");
  expect(root).toContain("SiteHeader");
  expect(root).toContain("SiteFooter");
  expect(root).toMatch(/<main\b[^>]*id=["']main["']/);
  expect(root).toContain("<Outlet");
  expect(root.indexOf("<SkipLink")).toBeLessThan(root.indexOf("<SiteHeader"));
  expect(root.indexOf("<SiteHeader")).toBeLessThan(root.indexOf("<main"));
  expect(root.indexOf("<main")).toBeLessThan(root.indexOf("<Outlet"));
  expect(root.indexOf("<Outlet")).toBeLessThan(root.indexOf("<SiteFooter"));
  expect(root).not.toContain("DocsShell");
  expect(existsSync(join(routesDir, "$.tsx"))).toBe(false);
  expect(root).not.toMatch(/statusCode:\s*200/);

  expect(notFound).toContain("public-site.chrome:not-found");
  expect(notFound).toMatch(/<h1\b/);
  expect(notFound).toMatch(/Not found/);
  expect(notFound).not.toMatch(/<p>\s*Not Found\s*<\/p>/);
  expect(notFound).toContain("from \"@tanstack/react-router\"");
  expect(notFound).toContain("Link");
  expect(notFound).toMatch(/to=["']\/(?:learn)?["']/);
  expect(notFound).toMatch(/>(?:Home|Learn)</);
  expect(notFound).toMatch(/<title>/);
  expect(notFound).toMatch(/Not found/);
  expect(notFound).not.toMatch(/<title>\s*Draconic\s*<\/title>/);
  expect(notFound).not.toContain("aria-current");
  expect(notFound).not.toContain("SkipLink");
  expect(notFound).not.toContain("SiteHeader");
  expect(notFound).not.toContain("SiteFooter");
  expect(notFound).not.toContain("DocsShell");
  expect(notFound).not.toMatch(/playground/i);
  expect(notFound).not.toContain("docs/");
  expect(notFound).not.toContain("activeProps");

  expect(header).toContain("activeProps");
  expect(header).toContain('"aria-current": "page"');
  expect(header).not.toMatch(/aria-current=["']page["']/);

  expect(variants).toContain("from \"class-variance-authority\"");
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-display");
  expect(variants).toContain("font-body");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(notFound).toContain("notFoundVariants");
  expect(notFound).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(notFound).not.toMatch(/\bmax-w-sm\b/);
  expect(notFound).not.toMatch(/bg-blue-500/);

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
