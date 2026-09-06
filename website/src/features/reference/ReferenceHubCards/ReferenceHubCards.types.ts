import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { referenceHubCardsVariants } from "./ReferenceHubCards.variants";

/**
 * Props for the Reference hub card grid of working-page links.
 */
export type ReferenceHubCardsProps = HTMLAttributes<HTMLDivElement> &
  VariantProps<typeof referenceHubCardsVariants>;
