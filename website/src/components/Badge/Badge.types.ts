import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { badgeVariants } from "./Badge.variants";

/**
 * Props for the status Badge primitive. Variant is the teaching-page status look.
 */
export type BadgeProps = HTMLAttributes<HTMLSpanElement> &
  VariantProps<typeof badgeVariants>;
