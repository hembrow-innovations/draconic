import { cva } from "class-variance-authority";

/**
 * Handbook chrome: section sidebar beside the article, not the home hero.
 */
export const docsShellVariants = cva(
  "flex gap-8 px-8 py-8 bg-canvas text-ink",
);

/**
 * Section nav column. Border uses the line role so the article stays the reading pane.
 */
export const docsShellAsideVariants = cva(
  "border-r border-line pr-6 font-body text-body text-ink",
);

/**
 * Chapter list inside the aside. Later sittings fill Learn and Reference hrefs.
 */
export const docsShellNavVariants = cva("flex flex-col gap-2");

/**
 * Article pane for rendered markdown plus the status Badge.
 */
export const docsShellArticleVariants = cva(
  "flex-1 min-w-0 flex flex-col gap-4 font-body text-body text-ink",
);
