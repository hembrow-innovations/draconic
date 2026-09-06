import type { HTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { learnPagerVariants } from "./LearnPager.variants";

/**
 * Props for the Learn article-footer prev/next chrome.
 */
export type LearnPagerProps = HTMLAttributes<HTMLElement> &
  VariantProps<typeof learnPagerVariants> & {
    slug: string;
  };
