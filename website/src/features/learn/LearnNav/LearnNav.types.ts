import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { learnNavListVariants } from "./LearnNav.variants";

/**
 * Props for the Learn sequence in the site side nav.
 */
export type LearnNavProps = HTMLAttributes<HTMLUListElement> &
  VariantProps<typeof learnNavListVariants>;
