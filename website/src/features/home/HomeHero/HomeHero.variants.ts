import { cva } from "class-variance-authority";

/**
 * Home landing band. Classes stay here so the JSX does not own a second type scale.
 */
export const homeHeroVariants = cva(
  "px-8 py-16 bg-canvas text-ink flex flex-col gap-6 max-w-3/4",
);

/**
 * Section label above the language name, letterspaced like ODM kickers.
 */
export const homeHeroKickerVariants = cva(
  "kicker font-body text-mono uppercase tracking-widest text-muted",
);

/**
 * Language name uses the display type role from the theme scale.
 */
export const homeHeroTitleVariants = cva("font-display text-display text-ink");

/**
 * Locked short pitch, presented as the lead under the title.
 */
export const homeHeroLeadVariants = cva("font-body text-body text-muted");

/**
 * Second locked pitch sentence, still body type, not a Learn dump.
 */
export const homeHeroPitchVariants = cva("font-body text-body text-ink");

/**
 * Install and Learn sit together as the only home actions.
 */
export const homeHeroCtaClusterVariants = cva(
  "cta-row flex items-center gap-4",
);

/**
 * Primary Install versus ghost Learn, painted with theme roles.
 */
export const homeHeroCtaVariants = cva(
  "inline-flex items-center justify-center min-h-11 px-5 font-body text-body no-underline",
  {
    variants: {
      variant: {
        primary: "bg-accent text-accent-foreground hover:bg-accent/90",
        ghost: "border border-line text-ink hover:bg-soft",
      },
    },
    defaultVariants: { variant: "primary" },
  },
);

/**
 * Numbered Install then Learn then Reference path under the CTA row.
 */
export const homeHeroPathVariants = cva(
  "path m-0 flex list-none flex-col gap-3 p-0",
);

/**
 * One numbered step in the language path.
 */
export const homeHeroPathStepVariants = cva(
  "flex items-center gap-3 font-body text-body text-ink",
);

/**
 * Visible step index, not a Learn chapter number.
 */
export const homeHeroPathIndexVariants = cva(
  "inline-flex items-center justify-center font-body text-mono text-accent-2",
);

/**
 * Path labels reuse existing routes, not new IA.
 */
export const homeHeroPathLinkVariants = cva(
  "font-body text-body text-link no-underline hover:text-ink",
);
