import { cva } from "class-variance-authority";

/**
 * Feature cards under the home pitch. Auto-fit so three facts wrap without a fixed column count.
 */
export const homeFeaturesVariants = cva(
  "px-8 py-12 bg-canvas text-ink grid gap-6 max-w-3/4 [grid-template-columns:repeat(auto-fit,minmax(min(100%,12rem),1fr))]",
);

/**
 * One glossary fact. Surface and ink come from theme roles.
 */
export const homeFeatureCardVariants = cva(
  "card border border-line p-6 bg-elevated text-ink",
);

/**
 * Fact copy uses the body type role from the theme scale.
 */
export const homeFeatureFactVariants = cva("font-body text-body text-ink");
