import type { ButtonHTMLAttributes } from "react";
import type { VariantProps } from "class-variance-authority";
import type { themeToggleVariants } from "./ThemeToggle.variants";

/**
 * Props for the light/dark token-set control.
 */
export type ThemeToggleProps = ButtonHTMLAttributes<HTMLButtonElement> &
  VariantProps<typeof themeToggleVariants>;
