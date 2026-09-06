import { cva } from "class-variance-authority";

/**
 * Theme control in site chrome. Same token roles as the rest of the header.
 */
export const themeToggleVariants = cva(
  "inline-flex items-center min-h-11 font-body text-body text-muted hover:text-ink transition-colors focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-accent",
);
