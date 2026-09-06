import { readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");

test("root route from nested src", () => {
  expect(statSync(join(srcDir, "routes")).isDirectory()).toBe(true);
  expect(statSync(join(srcDir, "components")).isDirectory()).toBe(true);
  expect(statSync(join(srcDir, "styles")).isDirectory()).toBe(true);

  const dumpedScreens = readdirSync(srcDir).filter(
    (name) => name.endsWith(".tsx") && name !== "router.tsx",
  );
  expect(dumpedScreens).toEqual([]);

  const indexSource = readFileSync(join(srcDir, "routes", "index.tsx"), "utf8");
  expect(indexSource).toContain("createFileRoute");
  expect(indexSource).toContain('"/"');

  const routeTree = readFileSync(join(srcDir, "routeTree.gen.ts"), "utf8");
  expect(routeTree).toContain("./routes/index");
});
