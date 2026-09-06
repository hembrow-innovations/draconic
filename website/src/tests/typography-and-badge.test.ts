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

test("typography and badge", () => {
  const css = readFileSync(join(srcDir, "styles", "theme.css"), "utf8");
  expect(css).toMatch(/@theme\s*\{/);
  expect(css).toContain("--font-display:");
  expect(css).toContain("--font-body:");
  expect(css).toContain("--font-mono:");
  expect(css).toContain("--text-display:");
  expect(css).toContain("--text-body:");
  expect(css).toContain("--text-mono:");

  const badgeDir = join(srcDir, "components", "Badge");
  expect(statSync(badgeDir).isDirectory()).toBe(true);
  for (const name of ["index.ts", "Badge.tsx", "Badge.types.ts", "Badge.variants.ts"]) {
    expect(statSync(join(badgeDir, name)).isFile()).toBe(true);
  }

  const variants = readFileSync(join(badgeDir, "Badge.variants.ts"), "utf8");
  expect(variants).toContain("from \"class-variance-authority\"");
  expect(variants).toContain("cva(");
  expect(variants).toContain("shipped");
  expect(variants).toContain("not-yet");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  const component = readFileSync(join(badgeDir, "Badge.tsx"), "utf8");
  expect(component).toContain("badgeVariants");
  expect(component).not.toMatch(/className=\{[^}]*variant\s*===/);
  expect(component).not.toMatch(/bg-blue-500/);
  expect(component).not.toMatch(/#[0-9A-Fa-f]{3,8}/);

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
