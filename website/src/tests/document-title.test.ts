import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { loadMarkdownPage } from "../lib/content";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const routesDir = join(srcDir, "routes");

function routeSource(name: string): string {
  return readFileSync(join(routesDir, name), "utf8");
}

function documentTitle(source: string, pageTitle: string): string {
  if (!/\bhead:/.test(source)) {
    return "Draconic";
  }
  const template = source.match(/title:\s*`([^`]+)`/);
  if (template?.[1] !== undefined) {
    return template[1]
      .replace(/\$\{loaderData\?\.title\s*\?\?[^}]+\}/g, pageTitle)
      .replace(/\$\{loaderData\?\.title\}/g, pageTitle)
      .replace(/\$\{loaderData\.title\}/g, pageTitle);
  }
  const literals = [...source.matchAll(/title:\s*"([^"]+)"/g)].map(
    (match) => match[1],
  );
  const last = literals.at(-1);
  if (last !== undefined) {
    return last;
  }
  if (source.includes("loaderData.title")) {
    return pageTitle;
  }
  return "Draconic";
}

test("document title", () => {
  const learnPage = loadMarkdownPage("learn");
  const referencePage = loadMarkdownPage("reference");
  const articlePage = loadMarkdownPage("from-javascript");
  const learn = routeSource("learn.tsx");
  const reference = routeSource("reference.tsx");
  const article = routeSource("from-javascript.tsx");
  const root = routeSource("__root.tsx");
  const home = routeSource("index.tsx");

  expect(learn).toContain("public-site.chrome:document-title");
  expect(reference).toContain("public-site.chrome:document-title");
  expect(article).toContain("public-site.chrome:document-title");

  const learnTitle = documentTitle(learn, learnPage.title);
  const referenceTitle = documentTitle(reference, referencePage.title);
  const articleTitle = documentTitle(article, articlePage.title);

  expect(learnPage.title).toBe("Learn");
  expect(referencePage.title).toBe("Reference");
  expect(articlePage.title).toBe("from JavaScript");
  expect(
    [learnTitle, referenceTitle, articleTitle].every(
      (title) => title === "Draconic",
    ),
  ).toBe(false);
  expect(new Set([learnTitle, referenceTitle, articleTitle]).size).toBe(3);
  expect(learnTitle).toContain("Learn");
  expect(learnTitle).not.toBe("Draconic");
  expect(referenceTitle).toContain("Reference");
  expect(referenceTitle).not.toBe("Draconic");
  expect(articleTitle).toContain("from JavaScript");
  expect(articleTitle).not.toBe("Draconic");

  expect(root).toContain('title: "Draconic"');
  expect(documentTitle(home, "Draconic")).toBe("Draconic");

  const markdownRoutes = readdirSync(routesDir).filter(
    (name) =>
      name.endsWith(".tsx") && name !== "__root.tsx" && name !== "index.tsx",
  );
  for (const name of markdownRoutes) {
    const slug = name.replace(/\.tsx$/, "");
    const page = loadMarkdownPage(slug);
    const source = routeSource(name);
    const title = documentTitle(source, page.title);
    expect(title, name).not.toBe("Draconic");
    expect(title, name).toContain(page.title);
    expect(source, name).toContain("public-site.chrome:document-title");
    expect(source, name).not.toMatch(/<h1\b/);
  }
});
