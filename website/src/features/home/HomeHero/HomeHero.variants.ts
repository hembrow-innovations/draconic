import { cva } from "class-variance-authority";

/**
 * Home landing band. Classes stay here so the JSX does not own a second type scale.
 */
export const homeHeroVariants = cva(
  "px-8 py-16 bg-canvas text-ink flex flex-col gap-6 max-w-3/4",
);

/**
 * Language name uses the display type role from the theme scale.
 */
export const homeHeroTitleVariants = cva("font-display text-display text-ink");

/**
 * Short pitch from Learn and CONTEXT, not the Learn hub article.
 */
export const homeHeroPitchVariants = cva("font-body text-body text-ink");

/**
 * Install and Learn sit together as the only home actions.
 */
export const homeHeroCtaClusterVariants = cva("flex items-center gap-4");

/**
 * Primary Install versus secondary Learn, painted with theme roles.
 */
export const homeHeroCtaVariants = cva(
  "inline-flex items-center justify-center min-h-11 px-5 font-body text-body no-underline",
  {
    variants: {
      variant: {
        primary: "bg-accent text-accent-foreground hover:bg-accent/90",
        secondary: "border border-line text-ink hover:bg-line",
      },
    },
    defaultVariants: { variant: "primary" },
  },
);
