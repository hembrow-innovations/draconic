import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");

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

test("semantic tokens", () => {
  const stylesDir = join(srcDir, "styles");
  const cssFiles = readdirSync(stylesDir).filter((name) => name.endsWith(".css"));
  expect(cssFiles.length).toBeGreaterThan(0);
  const css = cssFiles
    .map((name) => readFileSync(join(stylesDir, name), "utf8"))
    .join("\n");

  expect(css).toContain('@import "tailwindcss"');
  expect(css).toMatch(/@theme\s*\{/);
  expect(css).toContain("--color-canvas:");
  expect(css).toContain("--color-ink:");
  expect(css).toContain("--color-accent:");
  expect(css).toContain("--color-accent-2:");
  expect(css).toContain("--color-muted:");
  expect(css).toContain("--color-line:");

  const darkCss = css.match(/html\.dark\s*\{[\s\S]*?\}/)?.[0] ?? "";
  expect(darkCss).toContain("--color-canvas: #0c0f14");
  expect(darkCss).toContain("--color-ink: #e8eef6");
  expect(darkCss).toContain("--color-accent: #5b9fd4");
  expect(darkCss).toContain("--color-muted: #a3b4c6");
  expect(darkCss).toContain("--color-line: #2a3544");
  expect(darkCss).toContain("--color-accent-2: #7dd3a7");

  const pageSources = [
    readFileSync(join(srcDir, "routes", "__root.tsx"), "utf8"),
    readFileSync(join(srcDir, "routes", "index.tsx"), "utf8"),
  ].join("\n");
  expect(pageSources).toContain("bg-canvas");
  expect(pageSources).toContain("text-ink");

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/bg-blue-500/);
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
    expect(source, file).not.toMatch(/\bmax-w-xl\b/);
    expect(source, file).not.toMatch(/theme\.extend/);
  }
});
