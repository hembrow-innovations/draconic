import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { homeSampleVariants } from "./HomeSample.variants";

/**
 * Props for the language homepage hello Program sample.
 */
export type HomeSampleProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof homeSampleVariants>;
