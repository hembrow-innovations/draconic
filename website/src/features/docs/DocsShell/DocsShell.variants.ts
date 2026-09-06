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
 * Section label above the article heading, letterspaced like ODM kickers.
 */
export const docsShellKickerVariants = cva(
  "kicker font-body text-mono uppercase tracking-widest text-muted",
);

/**
 * Article pane: Badge, ODM prose (code surface, h2 rules, muted lede), not a new markdown language.
 */
export const docsShellArticleVariants = cva(
  "flex-1 min-w-0 flex flex-col gap-4 font-body text-body text-ink [&_h1]:font-display [&_h1]:text-display [&_h1]:text-ink [&_h1+p]:text-muted [&_h2]:mt-8 [&_h2]:border-t [&_h2]:border-line [&_h2]:pt-8 [&_h2:first-of-type]:mt-0 [&_h2:first-of-type]:border-t-0 [&_h2:first-of-type]:pt-0 [&_pre]:bg-code [&_pre]:p-4 [&_pre]:font-mono [&_pre]:text-mono [&_code]:bg-code [&_code]:font-mono [&_code]:text-mono [&_a]:text-link",
);

/**
 * Related-link footer slot under the article prose.
 */
export const docsShellFooterVariants = cva("w-full");
