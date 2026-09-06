import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const featuresDir = join(srcDir, "features", "home", "HomeFeatures");

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

test("home features", () => {
  expect(statSync(featuresDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "HomeFeatures.tsx",
    "HomeFeatures.types.ts",
    "HomeFeatures.variants.ts",
  ]) {
    expect(statSync(join(featuresDir, name)).isFile()).toBe(true);
  }

  const features = readFileSync(join(featuresDir, "HomeFeatures.tsx"), "utf8");
  const variants = readFileSync(
    join(featuresDir, "HomeFeatures.variants.ts"),
    "utf8",
  );
  const home = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");

  expect(home).toContain("createFileRoute");
  expect(home).toContain('"/"');
  expect(home).toContain("HomeHero");
  expect(home).toContain("HomeFeatures");
  expect(home).not.toContain("learn.md");

  expect(features).toContain("Compiles to JavaScript");
  expect(features).toContain("Compiles to native via LLVM");
  expect(features).toContain(
    "Dual worlds are JS values and native types at explicit boundaries",
  );
  expect(features.match(/<article\b/g)?.length).toBe(3);

  expect(features).not.toMatch(/playground/i);
  expect(features).not.toContain("FFI-only");
  expect(features).not.toContain("just typed JS");
  expect(features).not.toContain("docs/");
  expect(features).not.toContain("vault");
  expect(features).not.toContain("transpiler");
  expect(features).not.toContain("wasm");
  expect(features).not.toContain("Next.js");
  expect(features).not.toContain("Get started");

  expect(variants).toContain('from "class-variance-authority"');
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-body");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(features).toContain("homeFeaturesVariants");
  expect(features).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(features).not.toMatch(/\bmax-w-sm\b/);
  expect(features).not.toMatch(/bg-blue-500/);

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
