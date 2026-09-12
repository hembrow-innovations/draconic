import { readdirSync, readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import {
  loadMarkdownPage,
  pageDescriptionFromBody,
  pageShareHead,
} from "../lib/content";

const srcDir = join(dirname(fileURLToPath(import.meta.url)), "..");
const routesDir = join(srcDir, "routes");
const homePitch =
  "JavaScript you already know. Native types when you need them. One language, two backends.";
const articleLead =
  "This landing assumes you already write JavaScript or TypeScript. Learn will not teach ECMAScript from scratch.";
const siteOrigin = "https://hembrow-innovations.github.io/draconic";

function routeSource(name: string): string {
  return readFileSync(join(routesDir, name), "utf8");
}

function firstProseLine(body: string): string {
  for (const line of body.split(/\r?\n/)) {
    const trimmed = line.trim();
    if (
      trimmed === "" ||
      trimmed.startsWith("#") ||
      trimmed.startsWith("```") ||
      trimmed.startsWith("- ") ||
      trimmed.startsWith("* ")
    ) {
      continue;
    }
    return trimmed.replace(/\[([^\]]+)\]\([^)]*\)/g, "$1");
  }
  throw new Error("markdown body has no prose line");
}

function metaContent(
  head: ReturnType<typeof pageShareHead>,
  key: "name" | "property",
  value: string,
): string {
  const tag = head.meta.find((entry) => entry[key] === value);
  expect(tag?.content, value).toEqual(expect.any(String));
  expect(tag?.content, value).not.toBe("");
  return tag?.content ?? "";
}

function canonicalHref(head: ReturnType<typeof pageShareHead>): string {
  const link = head.links.find((entry) => entry.rel === "canonical");
  expect(link?.href).toEqual(expect.any(String));
  expect(link?.href).not.toBe("");
  return link?.href ?? "";
}

test("meta description", () => {
  const learnPage = loadMarkdownPage("learn");
  const articlePage = loadMarkdownPage("from-javascript");
  const home = routeSource("index.tsx");
  const learn = routeSource("learn.tsx");
  const article = routeSource("from-javascript.tsx");
  const hero = readFileSync(
    join(srcDir, "features", "home", "HomeHero", "HomeHero.tsx"),
    "utf8",
  );

  expect(hero).toContain(homePitch);
  expect(learnPage.title).toBe("Learn");
  expect(articlePage.title).toBe("from JavaScript");
  expect(firstProseLine(learnPage.body)).toBe(homePitch);
  expect(firstProseLine(articlePage.body)).toBe(articleLead);

  expect(home).toContain("public-site.chrome:meta-description");
  expect(learn).toContain("public-site.chrome:meta-description");
  expect(article).toContain("public-site.chrome:meta-description");
  expect(home).toContain("pageShareHead");
  expect(learn).toContain("pageShareHead");
  expect(article).toContain("pageShareHead");

  const homeHead = pageShareHead({
    title: "Draconic",
    path: "/",
    description: homePitch,
  });
  const learnHead = pageShareHead({
    title: `${learnPage.title} · Draconic`,
    path: "/learn",
    description: pageDescriptionFromBody(learnPage.body),
  });
  const articleHead = pageShareHead({
    title: `${articlePage.title} · Draconic`,
    path: "/from-javascript",
    description: pageDescriptionFromBody(articlePage.body),
  });

  expect(metaContent(homeHead, "name", "description")).toBe(homePitch);
  expect(metaContent(learnHead, "name", "description")).toBe(homePitch);
  expect(metaContent(articleHead, "name", "description")).toBe(articleLead);

  expect(canonicalHref(homeHead)).toBe(`${siteOrigin}/`);
  expect(canonicalHref(learnHead)).toBe(`${siteOrigin}/learn`);
  expect(canonicalHref(articleHead)).toBe(`${siteOrigin}/from-javascript`);
  expect(
    new Set([
      canonicalHref(homeHead),
      canonicalHref(learnHead),
      canonicalHref(articleHead),
    ]).size,
  ).toBe(3);

  expect(metaContent(homeHead, "property", "og:title")).toBe("Draconic");
  expect(metaContent(learnHead, "property", "og:title")).toContain("Learn");
  expect(metaContent(articleHead, "property", "og:title")).toContain(
    "from JavaScript",
  );
  expect(metaContent(homeHead, "property", "og:description")).toBe(homePitch);
  expect(metaContent(learnHead, "property", "og:description")).toBe(homePitch);
  expect(metaContent(articleHead, "property", "og:description")).toBe(
    articleLead,
  );
  expect(metaContent(homeHead, "property", "og:url")).toBe(`${siteOrigin}/`);
  expect(metaContent(learnHead, "property", "og:url")).toBe(
    `${siteOrigin}/learn`,
  );
  expect(metaContent(articleHead, "property", "og:url")).toBe(
    `${siteOrigin}/from-javascript`,
  );

  const markdownRoutes = readdirSync(routesDir).filter(
    (name) =>
      name.endsWith(".tsx") && name !== "__root.tsx" && name !== "index.tsx",
  );
  for (const name of markdownRoutes) {
    const slug = name.replace(/\.tsx$/, "");
    const page = loadMarkdownPage(slug);
    const source = routeSource(name);
    const lead = firstProseLine(page.body);
    expect(pageDescriptionFromBody(page.body), name).toBe(lead);
    expect(source, name).toContain("pageShareHead");
    expect(source, name).toContain("public-site.chrome:meta-description");
    expect(source, name).toContain("pageDescriptionFromBody");
    expect(source, name).not.toMatch(/<h1\b/);
    expect(source, name).not.toContain("twitter:");
    expect(source, name).not.toContain("og:image");
    expect(source, name).not.toContain("theme-color");
    const head = pageShareHead({
      title: `${page.title} · Draconic`,
      path: `/${page.slug}`,
      description: pageDescriptionFromBody(page.body),
    });
    expect(metaContent(head, "name", "description"), name).toBe(lead);
    expect(canonicalHref(head), name).toBe(`${siteOrigin}/${page.slug}`);
    expect(metaContent(head, "property", "og:title"), name).toContain(
      page.title,
    );
  }

  expect(home).not.toMatch(/<h1\b/);
  expect(home).not.toContain("twitter:");
  expect(home).not.toContain("og:image");
  expect(home).not.toContain("theme-color");
  expect(home).toContain(homePitch);
});
