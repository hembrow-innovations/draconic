import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { referencePagerVariants } from "./ReferencePager.variants";

/**
 * Props for the Reference article-footer prev/next chrome.
 */
export type ReferencePagerProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof referencePagerVariants> & {
    slug: string;
  };
