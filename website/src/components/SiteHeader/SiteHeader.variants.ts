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
  "font-display text-ink no-underline",
);

/**
 * Primary nav cluster: Learn, Reference, GitHub.
 */
export const siteHeaderNavVariants = cva(
  "flex items-center justify-between gap-4 min-h-16",
);

/**
 * Learn, Reference, and GitHub sit opposite the wordmark.
 */
export const siteHeaderClusterVariants = cva("flex items-center gap-5");

/**
 * In-app and GitHub items share the body type role.
 */
export const siteHeaderLinkVariants = cva(
  "inline-flex items-center min-h-11 font-body text-body text-muted no-underline hover:text-ink",
);
