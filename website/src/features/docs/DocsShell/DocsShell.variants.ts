import { cva } from "class-variance-authority";

/**
 * Handbook article chrome in the main column, not a second sidebar.
 */
export const docsShellVariants = cva("px-8 py-8 bg-canvas text-ink");

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
  "flex-1 min-w-0 flex flex-col gap-4 font-body text-body text-ink [&_h1]:font-display [&_h1]:text-display [&_h1]:text-ink [&>div>p:first-child]:text-muted [&_h2]:mt-8 [&_h2]:border-t [&_h2]:border-line [&_h2]:pt-8 [&_h2:first-of-type]:mt-0 [&_h2:first-of-type]:border-t-0 [&_h2:first-of-type]:pt-0 [&_pre]:bg-code [&_pre]:p-4 [&_pre]:font-mono [&_pre]:text-mono [&_code]:bg-code [&_code]:font-mono [&_code]:text-mono [&_a]:text-link",
);

/**
 * Related-link footer slot under the article prose.
 */
export const docsShellFooterVariants = cva("w-full");
