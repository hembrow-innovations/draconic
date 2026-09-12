import { mkdtempSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { expect, test } from "vitest";
import { listMarkdownPages, loadMarkdownPage } from "../lib/content";
import { listMarkdownPagesFromDir } from "../lib/content/loadMarkdown.fs";

const websiteDir = join(dirname(fileURLToPath(import.meta.url)), "..", "..");
const contentDir = join(websiteDir, "content");

test("markdown loader install.md", () => {
  const page = loadMarkdownPage("install");
  expect(page.slug).toBe("install");
  expect(page.title).toBe("Install");
  expect(page.section).toBe("learn");
  expect(page.status).toBe("shipped");
  expect(page.body).toContain("# Install");
  expect(page.body).toContain("Get the toolchain");
  expect(page.body.startsWith("---")).toBe(false);
  expect(page.body).not.toContain("title: Install");
});

test("markdown loader catalog from files not generate.drac", () => {
  const rootMarkdown = readdirSync(websiteDir).filter((name) =>
    name.endsWith(".md"),
  );
  expect(rootMarkdown).toEqual([]);
  const websiteSlugs = readdirSync(contentDir)
    .filter((name) => name.endsWith(".md"))
    .map((name) => name.slice(0, -3))
    .sort();
  expect(listMarkdownPages().map((page) => page.slug).sort()).toEqual(
    websiteSlugs,
  );

  const root = mkdtempSync(join(tmpdir(), "markdown-loader-"));
  try {
    writeFileSync(
      join(root, "install.md"),
      "---\ntitle: Install\nsection: learn\nstatus: shipped\n---\n\n# Install\n",
    );
    writeFileSync(
      join(root, "extra-chapter.md"),
      "---\ntitle: Extra\nsection: learn\nstatus: not-yet\n---\n\n# Extra\n",
    );
    expect(
      listMarkdownPagesFromDir(root)
        .map((page) => page.slug)
        .sort(),
    ).toEqual(["extra-chapter", "install"]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});

test("markdown loader does not load docs vault notes", () => {
  const slugs = listMarkdownPages().map((page) => page.slug);
  expect(slugs).not.toContain("purpose");
  expect(slugs).not.toContain("guides-public-docs");
  expect(slugs.every((slug) => !slug.includes("docs"))).toBe(true);
  expect(() =>
    loadMarkdownPage("../docs/specs/draconic/public-site/purpose"),
  ).toThrow();
});
