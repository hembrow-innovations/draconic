import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const toggleDir = join(srcDir, "components", "ThemeToggle");
const colorRoles = [
  "--color-canvas",
  "--color-ink",
  "--color-accent",
  "--color-accent-foreground",
  "--color-muted",
  "--color-line",
];

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

function tokenValues(css: string, token: string): string[] {
  const escaped = token.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return [...css.matchAll(new RegExp(`${escaped}:\\s*([^;]+);`, "g"))].map(
    (match) => match[1].trim(),
  );
}

test("theme toggle", () => {
  const css = readFileSync(join(srcDir, "styles", "theme.css"), "utf8");
  expect(css).toMatch(/@theme\s*\{/);
  expect(css).toMatch(/html\.dark\s*\{/);
  for (const role of colorRoles) {
    const values = tokenValues(css, role);
    expect(values.length, role).toBeGreaterThanOrEqual(2);
    expect(values[0], role).not.toBe(values[1]);
  }

  expect(statSync(toggleDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "ThemeToggle.tsx",
    "ThemeToggle.types.ts",
    "ThemeToggle.variants.ts",
  ]) {
    expect(statSync(join(toggleDir, name)).isFile()).toBe(true);
  }

  const toggle = readFileSync(join(toggleDir, "ThemeToggle.tsx"), "utf8");
  const variants = readFileSync(
    join(toggleDir, "ThemeToggle.variants.ts"),
    "utf8",
  );
  const header = readFileSync(
    join(srcDir, "components", "SiteHeader", "SiteHeader.tsx"),
    "utf8",
  );
  const root = readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8");
  const badgeVariants = readFileSync(
    join(srcDir, "components", "Badge", "Badge.variants.ts"),
    "utf8",
  );
  const headerVariants = readFileSync(
    join(srcDir, "components", "SiteHeader", "SiteHeader.variants.ts"),
    "utf8",
  );

  expect(header).toContain("ThemeToggle");
  expect(header).toMatch(/<ThemeToggle\s*\/>/);
  expect(toggle).toContain("<button");
  expect(toggle).toContain('type="button"');
  expect(toggle).toContain("themeToggleVariants");
  expect(toggle).not.toMatch(/account/i);
  expect(toggle).not.toMatch(/playground/i);
  expect(variants).toContain("from \"class-variance-authority\"");
  expect(variants).toContain("cva(");
  expect(variants).toContain("transition-colors");
  expect(variants).not.toMatch(/dark:(bg|text)-/);

  expect(badgeVariants).toContain("bg-accent");
  expect(badgeVariants).toContain("text-accent-foreground");
  expect(headerVariants).toContain("bg-canvas");
  expect(headerVariants).toContain("text-ink");
  expect(root).toContain("bg-canvas");
  expect(root).toContain("text-ink");
  expect(root).toContain("theme:v1");
  expect(root).toContain('classList.add("dark")');
  expect(root).not.toMatch(/<html[^>]*className=/);

  const product = walkProductFiles(srcDir);
  const joined = product.map((file) => readFileSync(file, "utf8")).join("\n");
  expect(joined).toContain("theme:v1");
  expect(joined).not.toMatch(/userId|password|signup|login/i);

  for (const file of product) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/bg-blue-500/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
    expect(source, file).not.toMatch(/dark:(bg|text)-/);
  }
});
