import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import {
  buildSearchIndex,
  querySearchIndex,
  searchHitLabel,
} from "../lib/search";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const searchDir = join(srcDir, "components", "SiteSearch");
const searchLib = join(srcDir, "lib", "search");

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

test("search", () => {
  expect(statSync(searchDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "SiteSearch.tsx",
    "SiteSearch.types.ts",
    "SiteSearch.variants.ts",
  ]) {
    expect(statSync(join(searchDir, name)).isFile()).toBe(true);
  }

  const search = readFileSync(join(searchDir, "SiteSearch.tsx"), "utf8");
  const types = readFileSync(join(searchDir, "SiteSearch.types.ts"), "utf8");
  const variants = readFileSync(
    join(searchDir, "SiteSearch.variants.ts"),
    "utf8",
  );
  const header = readFileSync(
    join(srcDir, "components", "SiteHeader", "SiteHeader.tsx"),
    "utf8",
  );
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");
  const lib = readFileSync(join(searchLib, "searchIndex.ts"), "utf8");

  expect(header).toContain("SiteSearch");
  expect(header).toMatch(/<SiteSearch\s*\/>/);
  expect(header.indexOf("<SiteSearch")).toBeLessThan(header.indexOf("<LearnNav"));
  expect(header.indexOf("<ThemeToggle")).toBeLessThan(header.indexOf("<LearnNav"));
  expect(header).not.toContain("Dual worlds");
  expect(root).toMatch(/<SiteHeader\s*\/>/);
  expect(root).toContain("buildSearchIndex");
  expect(root).toContain("SearchIndexProvider");

  expect(search).toContain("public-site.search:titles-headings");
  expect(search).toContain('type="search"');
  expect(search).toContain("querySearchIndex");
  expect(search).toContain("searchHitLabel");
  expect(search).toContain("No matching pages");
  expect(search).toContain("from \"@tanstack/react-router\"");
  expect(search).toContain("Link");
  expect(search).toContain("siteSearchVariants");
  expect(search).toContain("siteSearchEmptyVariants");
  expect(search).not.toMatch(/playground/i);
  expect(search).not.toContain("docs/");
  expect(search).not.toMatch(/account/i);
  expect(search).not.toContain(".html");
  expect(search).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(search).not.toMatch(/\bmax-w-sm\b/);

  expect(types).toContain("SearchEntry");

  expect(variants).toContain('from "class-variance-authority"');
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-body");
  expect(variants).toContain("siteSearchEmptyVariants");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);
  expect(variants).not.toMatch(/\babsolute\b/);

  const headerVariants = readFileSync(
    join(srcDir, "components", "SiteHeader", "SiteHeader.variants.ts"),
    "utf8",
  );
  expect(headerVariants).toMatch(/\boverflow-y-auto\b/);
  expect(search).not.toMatch(/\babsolute\b/);

  expect(lib).toContain("listMarkdownPages");
  expect(lib).not.toContain("docs/specs");
  expect(lib).not.toMatch(/playground/i);

  const index = buildSearchIndex();
  expect(index.length).toBeGreaterThan(0);
  for (const entry of index) {
    expect(entry.href.startsWith("/")).toBe(true);
    expect(entry.href).not.toContain(".html");
    expect(entry.href).not.toContain("docs");
    expect(existsSync(join(srcDir, "routes", `${entry.href.slice(1)}.tsx`))).toBe(
      true,
    );
    expect(entry).toHaveProperty("title");
    expect(entry).toHaveProperty("headings");
    expect(entry).toHaveProperty("section");
    expect(["learn", "reference"]).toContain(entry.section);
    expect(entry).not.toHaveProperty("body");
    expect(JSON.stringify(entry)).not.toMatch(/playground/i);
    expect(JSON.stringify(entry)).not.toContain("docs/");
  }

  const dualWorlds = querySearchIndex(index, "Dual worlds");
  expect(dualWorlds.some((hit) => hit.href === "/dual-worlds")).toBe(true);
  expect(dualWorlds.some((hit) => hit.title === "Dual worlds")).toBe(true);
  const dualPage = dualWorlds.find((hit) => hit.href === "/dual-worlds");
  expect(dualPage?.headings).toContain("Dual worlds");

  const nativeTypes = querySearchIndex(index, "Fixed structs");
  expect(nativeTypes.some((hit) => hit.href === "/native-types")).toBe(true);
  const nativePage = nativeTypes.find((hit) => hit.href === "/native-types");
  expect(nativePage?.section).toBe("learn");
  expect(nativePage?.headings).toContain("i32 and i64");
  expect(nativePage?.headings).toContain("Fixed structs");
  expect(searchHitLabel(nativePage!, "Fixed structs")).toBe(
    "Learn · native types · Fixed structs",
  );

  const packageHits = querySearchIndex(index, "packages");
  const learnPackages = packageHits.find((hit) => hit.href === "/packages");
  const referencePackages = packageHits.find(
    (hit) => hit.href === "/reference-packages",
  );
  expect(learnPackages?.section).toBe("learn");
  expect(referencePackages?.section).toBe("reference");
  expect(searchHitLabel(learnPackages!, "packages")).toBe("Learn · packages");
  expect(searchHitLabel(referencePackages!, "packages")).toBe(
    "Reference · packages",
  );

  const learnHits = querySearchIndex(index, "Dual worlds").filter(
    (hit) => hit.href === "/learn",
  );
  expect(learnHits).toEqual([]);

  expect(querySearchIndex(index, "tracing GC")).toEqual([]);
  expect(querySearchIndex(index, "Ownership-only")).toEqual([]);
  expect(querySearchIndex(index, "JavaScript you already know")).toEqual([]);
  expect(querySearchIndex(index, "Public site purpose")).toEqual([]);
  expect(querySearchIndex(index, "Give someone writing a Program")).toEqual([]);

  const websiteSlugs = readdirSync(websiteDir)
    .filter((name) => name.endsWith(".md"))
    .map((name) => name.slice(0, -3));
  const indexedHrefs = new Set(index.map((entry) => entry.href));
  for (const slug of websiteSlugs) {
    expect(indexedHrefs.has(`/${slug}`)).toBe(true);
  }

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
