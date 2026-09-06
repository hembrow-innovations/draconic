import type { AnchorHTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { skipLinkVariants } from "./SkipLink.variants";

/**
 * Props for the skip-to-content control. The target stays `#main`.
 */
export type SkipLinkProps = AnchorHTMLAttributes<HTMLAnchorElement> &
  VariantProps<typeof skipLinkVariants>;
