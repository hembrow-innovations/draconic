import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { homeHeroVariants } from "./HomeHero.variants";

/**
 * Props for the language homepage hero with kicker, CTAs, and path steps.
 */
export type HomeHeroProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof homeHeroVariants>;
