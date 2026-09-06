import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { homeFeaturesVariants } from "./HomeFeatures.variants";

/**
 * Props for the language homepage feature grid.
 */
export type HomeFeaturesProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof homeFeaturesVariants>;
