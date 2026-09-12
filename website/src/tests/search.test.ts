import { existsSync, readdirSync, readFileSync, statSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import {
  buildSearchIndex,
  querySearchIndex,
  searchHitHref,
  searchHitLabel,
} from "../lib/search";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const websiteDir = join(srcDir, "..");
const contentDir = join(websiteDir, "content");
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
  expect(search).toContain("public-site.search:session");
  expect(search).toContain("useLocation");
  expect(search).toContain("useEffect");
  expect(search).toContain("location.pathname");
  expect(search).toContain("location.hash");
  expect(search).toContain('setQuery("")');
  expect(search).toContain("onKeyDown");
  expect(search).toContain("Escape");
  expect(search).not.toContain("aria-current");
  expect(search).toContain('type="search"');
  expect(search).toContain("querySearchIndex");
  expect(search).toContain("searchHitLabel");
  expect(search).toContain("searchHitHref");
  expect(search).toContain("No matching pages");
  expect(search).toContain("from \"@tanstack/react-router\"");
  expect(search).toContain("Link");
  expect(search).toContain("hash=");
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

  const fromSource = querySearchIndex(index, "From source");
  const installHit = fromSource.find((hit) => hit.href === "/install");
  expect(installHit?.headings).toContain("From source");
  expect(searchHitHref(installHit!, "From source")).toBe("/install#from-source");
  expect(searchHitLabel(installHit!, "From source")).toBe(
    "Learn · Install · From source",
  );

  const todo = querySearchIndex(index, "Todo").find(
    (hit) => hit.href === "/from-javascript",
  );
  expect(todo?.headings).toContain("Todo");
  expect(searchHitHref(todo!, "Todo")).toBe("/from-javascript#todo");
  expect(searchHitLabel(todo!, "Todo")).toBe(
    "Learn · from JavaScript · Todo",
  );

  const modulesEntry = querySearchIndex(index, "Entry").find(
    (hit) => hit.href === "/modules",
  );
  expect(modulesEntry?.headings).toContain("Entry");
  expect(searchHitHref(modulesEntry!, "Entry")).toBe("/modules#entry");
  expect(searchHitLabel(modulesEntry!, "Entry")).toBe(
    "Learn · modules · Entry",
  );

  const nativeTypes = querySearchIndex(index, "Fixed structs");
  expect(nativeTypes.some((hit) => hit.href === "/native-types")).toBe(true);
  const nativePage = nativeTypes.find((hit) => hit.href === "/native-types");
  expect(nativePage?.section).toBe("learn");
  expect(nativePage?.headings).toContain("i32 and i64");
  expect(nativePage?.headings).toContain("Fixed structs");
  expect(searchHitLabel(nativePage!, "Fixed structs")).toBe(
    "Learn · native types · Fixed structs",
  );
  expect(searchHitHref(nativePage!, "Fixed structs")).toBe(
    "/native-types#fixed-structs",
  );
  expect(searchHitHref(nativePage!, "native types")).toBe("/native-types");
  expect(searchHitHref(nativePage!, "i32")).toBe("/native-types#i32-and-i64");
  expect(nativePage?.headings).toContain("i8, u8, f32, and bool");
  expect(nativePage?.headings).toContain("Fixed arrays");
  expect(nativePage?.headings).toContain("Pointers");
  expect(searchHitHref(nativePage!, "i8")).toBe(
    "/native-types#i8-u8-f32-and-bool",
  );
  expect(searchHitLabel(nativePage!, "i8")).toBe(
    "Learn · native types · i8, u8, f32, and bool",
  );
  expect(searchHitHref(nativePage!, "Fixed arrays")).toBe(
    "/native-types#fixed-arrays",
  );
  expect(searchHitLabel(nativePage!, "Fixed arrays")).toBe(
    "Learn · native types · Fixed arrays",
  );
  expect(searchHitHref(nativePage!, "Pointers")).toBe("/native-types#pointers");
  expect(searchHitLabel(nativePage!, "Pointers")).toBe(
    "Learn · native types · Pointers",
  );

  const typesObject = querySearchIndex(index, "Object types").find(
    (hit) => hit.href === "/types",
  );
  expect(typesObject?.headings).toContain("Object types");
  expect(typesObject?.headings).toContain("Unions and intersections");
  expect(typesObject?.headings).toContain("Generics");
  expect(typesObject?.headings).toContain("Fixed structs");
  expect(searchHitHref(typesObject!, "Object types")).toBe(
    "/types#object-types",
  );
  expect(searchHitLabel(typesObject!, "Object types")).toBe(
    "Reference · types · Object types",
  );
  expect(searchHitHref(typesObject!, "Unions and intersections")).toBe(
    "/types#unions-and-intersections",
  );
  expect(searchHitLabel(typesObject!, "Unions and intersections")).toBe(
    "Reference · types · Unions and intersections",
  );
  expect(searchHitHref(typesObject!, "Generics")).toBe("/types#generics");
  expect(searchHitLabel(typesObject!, "Generics")).toBe(
    "Reference · types · Generics",
  );
  const typesFixed = querySearchIndex(index, "Fixed structs").find(
    (hit) => hit.href === "/types",
  );
  expect(typesFixed?.headings).toContain("Fixed structs");
  expect(searchHitHref(typesFixed!, "Fixed structs")).toBe(
    "/types#fixed-structs",
  );
  expect(searchHitLabel(typesFixed!, "Fixed structs")).toBe(
    "Reference · types · Fixed structs",
  );
  expect(typesObject?.headings).toContain("i8, u8, f32, and bool");
  expect(typesObject?.headings).toContain("Fixed arrays");
  expect(typesObject?.headings).toContain("Pointers");
  expect(searchHitHref(typesObject!, "i8")).toBe("/types#i8-u8-f32-and-bool");
  expect(searchHitLabel(typesObject!, "i8")).toBe(
    "Reference · types · i8, u8, f32, and bool",
  );
  expect(searchHitHref(typesObject!, "Fixed arrays")).toBe(
    "/types#fixed-arrays",
  );
  expect(searchHitLabel(typesObject!, "Fixed arrays")).toBe(
    "Reference · types · Fixed arrays",
  );
  expect(searchHitHref(typesObject!, "Pointers")).toBe("/types#pointers");
  expect(searchHitLabel(typesObject!, "Pointers")).toBe(
    "Reference · types · Pointers",
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

  const packageRoot = querySearchIndex(index, "Package root").find(
    (hit) => hit.href === "/packages",
  );
  expect(packageRoot?.headings).toContain("Package root");
  expect(packageRoot?.headings).toContain("get");
  expect(packageRoot?.headings).toContain("mod tidy");
  expect(packageRoot?.headings).toContain("Flagship service");
  expect(searchHitHref(packageRoot!, "Package root")).toBe(
    "/packages#package-root",
  );
  expect(searchHitLabel(packageRoot!, "Package root")).toBe(
    "Learn · packages · Package root",
  );
  expect(searchHitHref(packageRoot!, "get")).toBe("/packages#get");
  expect(searchHitLabel(packageRoot!, "get")).toBe("Learn · packages · get");
  expect(searchHitHref(packageRoot!, "mod tidy")).toBe("/packages#mod-tidy");
  expect(searchHitLabel(packageRoot!, "mod tidy")).toBe(
    "Learn · packages · mod tidy",
  );

  const flagship = querySearchIndex(index, "Flagship service").find(
    (hit) => hit.href === "/packages",
  );
  expect(flagship?.headings).toContain("Flagship service");
  expect(searchHitHref(flagship!, "Flagship service")).toBe(
    "/packages#flagship-service",
  );
  expect(searchHitLabel(flagship!, "Flagship service")).toBe(
    "Learn · packages · Flagship service",
  );

  const cliRun = querySearchIndex(index, "run").find(
    (hit) => hit.href === "/cli",
  );
  expect(cliRun?.headings).toContain("run");
  expect(searchHitHref(cliRun!, "run")).toBe("/cli#run");
  expect(searchHitLabel(cliRun!, "run")).toBe("Reference · CLI · run");

  const cliFmt = querySearchIndex(index, "fmt").find(
    (hit) => hit.href === "/cli",
  );
  expect(cliFmt?.headings).toContain("fmt");
  expect(searchHitHref(cliFmt!, "fmt")).toBe("/cli#fmt");
  expect(searchHitLabel(cliFmt!, "fmt")).toBe("Reference · CLI · fmt");

  const cliRepl = querySearchIndex(index, "repl").find(
    (hit) => hit.href === "/cli",
  );
  expect(cliRepl?.headings).toContain("repl");
  expect(searchHitHref(cliRepl!, "repl")).toBe("/cli#repl");
  expect(searchHitLabel(cliRepl!, "repl")).toBe("Reference · CLI · repl");

  const cliExtract = querySearchIndex(index, "extract").find(
    (hit) => hit.href === "/cli",
  );
  expect(cliExtract?.headings).toContain("extract");
  expect(searchHitHref(cliExtract!, "extract")).toBe("/cli#extract");
  expect(searchHitLabel(cliExtract!, "extract")).toBe(
    "Reference · CLI · extract",
  );

  const cliDoc = querySearchIndex(index, "doc").find(
    (hit) => hit.href === "/cli",
  );
  expect(cliDoc?.headings).toContain("doc");
  expect(searchHitHref(cliDoc!, "doc")).toBe("/cli#doc");
  expect(searchHitLabel(cliDoc!, "doc")).toBe("Reference · CLI · doc");

  const cliBindgen = querySearchIndex(index, "bindgen").find(
    (hit) => hit.href === "/cli",
  );
  expect(cliBindgen?.headings).toContain("bindgen");
  expect(searchHitHref(cliBindgen!, "bindgen")).toBe("/cli#bindgen");
  expect(searchHitLabel(cliBindgen!, "bindgen")).toBe(
    "Reference · CLI · bindgen",
  );

    const learnProcessArgs = querySearchIndex(index, "processArgs").find(
      (hit) => hit.href === "/host-io",
    );
    expect(learnProcessArgs?.headings).toContain("processArgs");
    expect(searchHitHref(learnProcessArgs!, "processArgs")).toBe(
      "/host-io#processargs",
    );
    expect(searchHitLabel(learnProcessArgs!, "processArgs")).toBe(
      "Learn · host I/O · processArgs",
    );

    const referenceProcessArgs = querySearchIndex(index, "processArgs").find(
      (hit) => hit.href === "/reference-host-io",
    );
    expect(referenceProcessArgs?.headings).toContain("processArgs");
    expect(searchHitHref(referenceProcessArgs!, "processArgs")).toBe(
      "/reference-host-io#processargs",
    );
    expect(searchHitLabel(referenceProcessArgs!, "processArgs")).toBe(
      "Reference · host I/O · processArgs",
    );

    const learnPathJoin = querySearchIndex(index, "pathJoin").find(
      (hit) => hit.href === "/host-io",
    );
    expect(learnPathJoin?.headings).toContain("pathJoin");
    expect(searchHitHref(learnPathJoin!, "pathJoin")).toBe("/host-io#pathjoin");
    expect(searchHitLabel(learnPathJoin!, "pathJoin")).toBe(
      "Learn · host I/O · pathJoin",
    );

    const referencePathJoin = querySearchIndex(index, "pathJoin").find(
      (hit) => hit.href === "/reference-host-io",
    );
    expect(referencePathJoin?.headings).toContain("pathJoin");
    expect(searchHitHref(referencePathJoin!, "pathJoin")).toBe(
      "/reference-host-io#pathjoin",
    );
    expect(searchHitLabel(referencePathJoin!, "pathJoin")).toBe(
      "Reference · host I/O · pathJoin",
    );

    const learnMkdir = querySearchIndex(index, "mkdir").find(
      (hit) => hit.href === "/host-io",
    );
    expect(learnMkdir?.headings).toContain("mkdir");
    expect(searchHitHref(learnMkdir!, "mkdir")).toBe("/host-io#mkdir");
    expect(searchHitLabel(learnMkdir!, "mkdir")).toBe(
      "Learn · host I/O · mkdir",
    );

    const referenceMkdir = querySearchIndex(index, "mkdir").find(
      (hit) => hit.href === "/reference-host-io",
    );
    expect(referenceMkdir?.headings).toContain("mkdir");
    expect(searchHitHref(referenceMkdir!, "mkdir")).toBe(
      "/reference-host-io#mkdir",
    );
    expect(searchHitLabel(referenceMkdir!, "mkdir")).toBe(
      "Reference · host I/O · mkdir",
    );

    const referenceExists = querySearchIndex(index, "exists").find(
      (hit) => hit.href === "/reference-host-io",
    );
    expect(referenceExists?.headings).toContain("exists");
    expect(searchHitHref(referenceExists!, "exists")).toBe(
      "/reference-host-io#exists",
    );
    expect(searchHitLabel(referenceExists!, "exists")).toBe(
      "Reference · host I/O · exists",
    );

    const tcpListen = querySearchIndex(index, "tcpListen").find(
      (hit) => hit.href === "/host-io",
    );
  expect(tcpListen?.headings).toContain("tcpListen");
  expect(searchHitHref(tcpListen!, "tcpListen")).toBe("/host-io#tcplisten");
  expect(searchHitLabel(tcpListen!, "tcpListen")).toBe(
    "Learn · host I/O · tcpListen",
  );

  const learnEcho = querySearchIndex(index, "HTTP echo").find(
    (hit) => hit.href === "/host-io",
  );
  expect(learnEcho?.headings).toContain("HTTP echo");
  expect(searchHitHref(learnEcho!, "HTTP echo")).toBe("/host-io#http-echo");
  expect(searchHitLabel(learnEcho!, "HTTP echo")).toBe(
    "Learn · host I/O · HTTP echo",
  );

  const referenceEcho = querySearchIndex(index, "HTTP echo").find(
    (hit) => hit.href === "/reference-host-io",
  );
  expect(referenceEcho?.headings).toContain("HTTP echo");
  expect(searchHitHref(referenceEcho!, "HTTP echo")).toBe(
    "/reference-host-io#http-echo",
  );
  expect(searchHitLabel(referenceEcho!, "HTTP echo")).toBe(
    "Reference · host I/O · HTTP echo",
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

  const websiteSlugs = readdirSync(contentDir)
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
