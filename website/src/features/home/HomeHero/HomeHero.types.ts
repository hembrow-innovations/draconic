import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { homeHeroVariants } from "./HomeHero.variants";

/**
 * Props for the language homepage hero. Feature grid waits for a later sitting.
 */
export type HomeHeroProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof homeHeroVariants>;
