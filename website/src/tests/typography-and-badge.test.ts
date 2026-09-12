import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");

function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  return [
    parseInt(h.slice(0, 2), 16),
    parseInt(h.slice(2, 4), 16),
    parseInt(h.slice(4, 6), 16),
  ];
}

function relativeLuminance(hex: string): number {
  const channels = hexToRgb(hex).map((value) => {
    const c = value / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrastRatio(foreground: string, background: string): number {
  const first = relativeLuminance(foreground);
  const second = relativeLuminance(background);
  const [hi, lo] = first > second ? [first, second] : [second, first];
  return (hi + 0.05) / (lo + 0.05);
}

function blockColor(block: string, name: string): string {
  return (
    block.match(new RegExp(`--color-${name}:\\s*(#[0-9A-Fa-f]{6})`))?.[1] ?? ""
  );
}

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
  expect(variants).toContain("bg-accent");
  expect(variants).toContain("text-accent-foreground");
  expect(variants).toContain("text-mono");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  const lightTheme = css.match(/@theme\s*\{([\s\S]*?)\}/)?.[1] ?? "";
  const darkTheme = css.match(/html\.dark\s*\{([\s\S]*?)\}/)?.[1] ?? "";
  const lightFill = blockColor(lightTheme, "accent");
  const lightInk = blockColor(lightTheme, "accent-foreground");
  const darkFill = blockColor(darkTheme, "accent");
  const darkInk = blockColor(darkTheme, "accent-foreground");
  expect([hexToRgb(lightInk), hexToRgb(lightFill)]).not.toEqual([
    [248, 251, 255],
    [49, 120, 198],
  ]);
  expect(contrastRatio(lightInk, lightFill)).toBeGreaterThanOrEqual(4.5);
  expect(contrastRatio(darkInk, darkFill)).toBeGreaterThanOrEqual(4.5);

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
