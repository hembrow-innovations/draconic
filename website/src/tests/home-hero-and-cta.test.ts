import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const heroDir = join(srcDir, "features", "home", "HomeHero");

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

test("home hero and cta", () => {
  expect(statSync(heroDir).isDirectory()).toBe(true);
  for (const name of [
    "index.ts",
    "HomeHero.tsx",
    "HomeHero.types.ts",
    "HomeHero.variants.ts",
  ]) {
    expect(statSync(join(heroDir, name)).isFile()).toBe(true);
  }

  const hero = readFileSync(join(heroDir, "HomeHero.tsx"), "utf8");
  const variants = readFileSync(join(heroDir, "HomeHero.variants.ts"), "utf8");
  const home = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");
  const learnHub = readFileSync(join(websiteDir, "content", "learn.md"), "utf8");

  expect(home).toContain("createFileRoute");
  expect(home).toContain('"/"');
  expect(home).toContain("HomeHero");
  expect(home).not.toContain("learn.md");
  expect(home).not.toContain("Start at");
  expect(home).not.toContain("from JavaScript");
  expect(home).not.toContain("from systems");

  expect(home).not.toContain("DocsShell");
  expect(hero).toContain("public-site.home:landing");
  expect(hero).toContain("from \"@tanstack/react-router\"");
  expect(hero).toContain("Link");
  expect(hero).toContain("JavaScript you already know");
  expect(hero).toContain("Native types when you need them");
  expect(hero).toContain("One language, two backends");
  expect(hero).toContain("<h1");
  expect(hero).toContain("homeHeroKickerVariants");
  expect(hero).toContain("homeHeroLeadVariants");
  expect(hero).toContain("homeHeroCtaClusterVariants");
  expect(hero).toContain("homeHeroPathVariants");
  expect(hero).toMatch(/to=["']\/install["']/);
  expect(hero).toContain(">Install<");
  expect(hero).toMatch(/to=["']\/learn["']/);
  expect(hero).toContain(">Learn<");
  expect(hero).toMatch(/to=["']\/reference["']/);
  expect(hero).toContain(">Reference<");
  expect(hero).toContain(">1<");
  expect(hero).toContain(">2<");
  expect(hero).toContain(">3<");
  expect(hero).toContain("<ol");
  expect(hero).toMatch(/homeHeroCtaVariants\(\{\s*variant:\s*["']primary["']\s*\}\)/);
  expect(hero).toMatch(/homeHeroCtaVariants\(\{\s*variant:\s*["']ghost["']\s*\}\)/);
  expect(hero).not.toContain(".html");
  expect(hero).not.toContain("Start at");
  expect(hero).not.toContain("from JavaScript");
  expect(hero).not.toContain("from systems");
  expect(hero).not.toContain("Those landings join");
  expect(hero).not.toContain("Keep [Reference]");
  expect(hero).not.toContain("Keep Reference");
  expect(hero).not.toMatch(/playground/i);
  expect(hero).not.toContain("docs/");
  expect(hero).not.toContain("vault");
  expect(hero).not.toContain("Get started");
  expect(hero).not.toContain("getting started");
  expect(hero).not.toContain("tutorial");
  expect(hero).not.toContain("language book");
  expect(hero).not.toContain("transpiler");
  expect(hero).not.toContain("JS backend");
  expect(hero).not.toContain("LLVM backend");

  expect(learnHub).toContain("# Learn");
  expect(learnHub).toContain("Start at [Install](install.html)");
  expect(hero).not.toBe(learnHub);

  expect(variants).toContain("from \"class-variance-authority\"");
  expect(variants).toContain("cva(");
  expect(variants).toContain("font-display");
  expect(variants).toContain("font-body");
  expect(variants).toContain("uppercase");
  expect(variants).toMatch(/\btracking-widest\b/);
  expect(variants).toContain("cta-row");
  expect(variants).toContain("kicker");
  expect(variants).toContain("ghost:");
  expect(variants).toContain("path");
  expect(variants).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(variants).not.toMatch(/\bmax-w-sm\b/);

  expect(hero).toContain("homeHeroVariants");
  expect(hero).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
  expect(hero).not.toMatch(/\bmax-w-sm\b/);
  expect(hero).not.toMatch(/bg-blue-500/);

  for (const file of walkProductFiles(srcDir)) {
    const source = readFileSync(file, "utf8");
    expect(source, file).not.toMatch(/#[0-9A-Fa-f]{3,8}/);
    expect(source, file).not.toMatch(/\bmax-w-sm\b/);
  }
});
