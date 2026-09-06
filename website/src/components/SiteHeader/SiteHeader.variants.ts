import { cva } from "class-variance-authority";

/**
 * Site header chrome. Classes stay here so the JSX does not own a second type scale.
 */
export const siteHeaderVariants = cva(
  "px-8 bg-canvas text-ink border-b border-line",
);

/**
 * Wordmark uses the display type role from the theme scale.
 */
export const siteHeaderWordmarkVariants = cva(
  "font-display text-ink no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);

/**
 * Primary nav cluster: Learn, Reference, GitHub.
 */
export const siteHeaderNavVariants = cva(
  "flex flex-wrap items-center justify-between gap-4 min-h-16",
);

/**
 * Learn, Reference, and GitHub sit opposite the wordmark.
 * Closed on small viewports until the disclosure opens; always shown from md.
 */
export const siteHeaderClusterVariants = cva("gap-5", {
  variants: {
    open: {
      true: "flex w-full flex-col md:w-auto md:flex-row md:items-center",
      false: "hidden md:flex md:items-center",
    },
  },
  defaultVariants: { open: false },
});

/**
 * Small-viewport control that discloses the same primary links. Hidden from md.
 */
export const siteHeaderMenuButtonVariants = cva(
  "inline-flex items-center min-h-11 font-body text-body text-muted hover:text-ink md:hidden focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);

/**
 * In-app and GitHub items share the body type role.
 */
export const siteHeaderLinkVariants = cva(
  "inline-flex items-center min-h-11 font-body text-body text-muted no-underline hover:text-ink focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);
