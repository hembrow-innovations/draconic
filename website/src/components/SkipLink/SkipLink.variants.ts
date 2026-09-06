import { cva } from "class-variance-authority";

/**
 * Hidden until focused so the first tab stop is not a visual dump on every page.
 */
export const skipLinkVariants = cva(
  "sr-only focus:not-sr-only focus:absolute focus:left-4 focus:top-4 focus:z-10 focus:bg-canvas focus:text-ink focus:px-3 focus:py-2 font-body no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);
