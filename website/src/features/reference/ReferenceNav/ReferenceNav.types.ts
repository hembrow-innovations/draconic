import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { referenceNavListVariants } from "./ReferenceNav.variants";

/**
 * Props for the Reference sequence in the site side nav.
 */
export type ReferenceNavProps = HTMLAttributes<HTMLUListElement> &
  VariantProps<typeof referenceNavListVariants>;
