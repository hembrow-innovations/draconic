import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { homeSampleVariants } from "./HomeSample.variants";

/**
 * Props for the language homepage hello and typed greet Program samples.
 */
export type HomeSampleProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof homeSampleVariants>;
