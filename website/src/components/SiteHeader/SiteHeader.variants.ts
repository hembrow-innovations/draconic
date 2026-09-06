import { cva } from "class-variance-authority";

/**
 * Two-column ODM shell wrapping the sticky side nav and the main column.
 */
export const siteShellVariants = cva("shell flex min-h-dvh bg-canvas text-ink");

/**
 * Reading column beside the sticky side nav.
 */
export const siteMainVariants = cva("main flex min-w-0 flex-1 flex-col");

/**
 * Sticky side nav chrome. Classes stay here so the JSX does not own a second type scale.
 */
export const siteHeaderVariants = cva(
  "side sticky top-0 h-dvh w-1/4 shrink-0 overflow-y-auto border-r border-line bg-canvas px-6 py-8 text-ink",
);

/**
 * Wordmark uses the display type role from the theme scale.
 */
export const siteHeaderWordmarkVariants = cva(
  "font-display text-ink no-underline focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);

/**
 * Short language tagline under the wordmark, reused from the home pitch.
 */
export const siteHeaderTaglineVariants = cva("font-body text-body text-muted");

/**
 * Primary nav cluster: Learn, Reference, GitHub.
 */
export const siteHeaderNavVariants = cva("flex flex-col gap-4");

/**
 * Learn, Reference, and GitHub sit under the wordmark.
 * Closed on small viewports until the disclosure opens; always shown from md.
 */
export const siteHeaderClusterVariants = cva("gap-3", {
  variants: {
    open: {
      true: "flex w-full flex-col",
      false: "hidden md:flex md:flex-col",
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
