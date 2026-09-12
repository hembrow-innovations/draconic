import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const sampleDir = join(srcDir, "features", "home", "HomeSample");

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

test("home sample", () => {
  expect(statSync(sampleDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "HomeSample.tsx",
    "HomeSample.types.ts",
    "HomeSample.variants.ts",
  ]) {
    expect(statSync(join(sampleDir, name)).isFile()).toBe(true);
  }

  const sample = readFileSync(join(sampleDir, "HomeSample.tsx"), "utf8");
  const variants = readFileSync(
    join(sampleDir, "HomeSample.variants.ts"),
    "utf8",
  );
  const home = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");
  const install = readFileSync(join(websiteDir, "content", "install.md"), "utf8");
  const types = readFileSync(join(websiteDir, "content", "types.md"), "utf8");
  const nativeTypes = readFileSync(
    join(websiteDir, "content", "native-types.md"),
    "utf8",
  );

  expect(home).toContain("createFileRoute");
  expect(home).toContain('"/"');
  expect(home).toContain("HomeHero");
  expect(home).toContain("HomeSample");
  expect(home).toContain("HomeFeatures");
  expect(home).not.toContain("learn.md");
  expect(home).not.toContain("DocsShell");

  expect(sample).toContain("public-site.home:landing");
  expect(sample).toContain("from \"@tanstack/react-router\"");
  expect(sample).toContain("Link");
  expect(sample).toContain("A Program");
  expect(sample).toContain("hello.drac");
  expect(sample).toContain("It builds today");
  expect(sample).toContain("let console = globalThis.console;");
  expect(sample).toContain('console.log("hello from Draconic")');
  expect(sample).toContain("<pre");
  expect(sample).toContain("<code");
  expect(sample).toMatch(/to=["']\/install["']/);
  expect(install).toContain("let console = globalThis.console;");
  expect(install).toContain('console.log("hello from Draconic")');

  expect(sample).toContain("greet.drac");
  expect(sample).toContain("function greet(name: string): string");
  expect(sample).toContain('console.log(greet("from Draconic"))');
  expect(sample).toContain("draconic check greet.drac");
  expect(sample).toMatch(/to=["']\/types["']/);
  expect(sample).toContain(">types<");
  expect(types).toContain("function greet(name: string): string");
  expect(types).toContain("draconic check greet.drac");

  expect(sample).toContain("width.drac");
  expect(sample).toContain("let count: i32 = 41");
  expect(sample).toContain("let wide: i64 = 42");
  expect(sample).toContain("console.log(count as number, wide as number)");
  expect(sample).toContain("draconic build --target native width.drac -o width");
  expect(sample).toMatch(/to=["']\/native-types["']/);
  expect(sample).toContain(">native types<");
  expect(nativeTypes).toContain("let count: i32 = 41");
  expect(nativeTypes).toContain("let wide: i64 = 42");
  expect(nativeTypes).toContain("console.log(count as number, wide as number)");
  expect(sample.match(/<pre/g)?.length).toBeGreaterThanOrEqual(4);

  expect(sample).not.toContain("DocsShell");
  expect(sample).not.toMatch(/playground/i);
  expect(sample).not.toContain("Get started");
  expect(sample).not.toContain("getting started");
  expect(sample).not.toContain("tutorial");
  expect(sample).not.toContain("docs/");
  expect(sample).not.toContain("vault");
  expect(sample).not.toContain("from JavaScript");
  expect(sample).not.toContain("from systems");
  expect(sample).not.toContain("Copy");
  expect(sample).not.toContain("clipboard");
  expect(sample).not.toContain("CodeFence");

  expect(variants).toContain('from "class-variance-authority"');
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-body");
  expect(variants).toContain("font-mono");
  expect(variants).toContain("bg-code");
  expect(variants).toContain("kicker");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(sample).toContain("homeSampleVariants");
  expect(sample).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(sample).not.toMatch(/\bmax-w-sm\b/);
  expect(sample).not.toMatch(/bg-blue-500/);

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
