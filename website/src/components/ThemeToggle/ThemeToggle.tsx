import { useEffect, useState } from "react";
import {
  applyTheme,
  readStoredTheme,
  toggleTheme,
  writeStoredTheme,
  type ThemeName,
} from "../../lib/theme";
import type { ThemeToggleProps } from "./ThemeToggle.types";
import { themeToggleVariants } from "./ThemeToggle.variants";

/**
 * Swaps light and dark by toggling the document class that selects `@theme` token sets.
 *
 * @param props - Native button attributes
 * @returns Theme toggle
 */
export function ThemeToggle({ className, ...props }: ThemeToggleProps) {
  const [theme, setTheme] = useState<ThemeName>("light");

  useEffect(() => {
    const initial =
      readStoredTheme() ??
      (document.documentElement.classList.contains("dark") ? "dark" : "light");
    setTheme(initial);
    applyTheme(initial);
  }, []);

  function onToggle() {
    const next = toggleTheme(theme);
    setTheme(next);
    applyTheme(next);
    writeStoredTheme(next);
  }

  return (
    <button
      type="button"
      className={themeToggleVariants({ className })}
      onClick={onToggle}
      aria-pressed={theme === "dark"}
      aria-label={
        theme === "dark" ? "Switch to light theme" : "Switch to dark theme"
      }
      {...props}
    >
      {theme === "dark" ? "Light" : "Dark"}
    </button>
  );
}
