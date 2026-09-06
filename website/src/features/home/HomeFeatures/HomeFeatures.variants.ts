import { cva } from "class-variance-authority";

/**
 * Three-fact grid under the home pitch. Classes stay here so the JSX does not own a second type scale.
 */
export const homeFeaturesVariants = cva(
  "px-8 py-12 bg-canvas text-ink grid grid-cols-3 gap-6 max-w-3/4",
);

/**
 * One glossary fact. Surface and ink come from theme roles.
 */
export const homeFeatureCardVariants = cva(
  "border border-line p-6 bg-canvas text-ink",
);

/**
 * Fact copy uses the body type role from the theme scale.
 */
export const homeFeatureFactVariants = cva("font-body text-body text-ink");
