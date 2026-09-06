import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { learnNavListVariants } from "./LearnNav.variants";

/**
 * Props for the Learn aside sequence. Chapter bodies wait for a later sitting.
 */
export type LearnNavProps = HTMLAttributes<HTMLUListElement> &
  VariantProps<typeof learnNavListVariants>;
