import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { learnHubCardsVariants } from "./LearnHubCards.variants";

/**
 * Props for the Learn hub card grid of chapter links.
 */
export type LearnHubCardsProps = HTMLAttributes<HTMLDivElement> &
  VariantProps<typeof learnHubCardsVariants>;
