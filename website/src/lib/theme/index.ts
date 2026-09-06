export type ThemeName = "light" | "dark";

export const THEME_STORAGE_KEY = "theme:v1";
export const THEME_DARK_CLASS = "dark";

type ThemeRecord = { theme: ThemeName };

/**
 * Narrow unknown storage payloads to a theme name.
 *
 * @param value - Parsed JSON or any other unknown
 * @returns Whether the value is `light` or `dark`
 */
export function isThemeName(value: unknown): value is ThemeName {
  return value === "light" || value === "dark";
}

/**
 * Read the versioned theme record from localStorage.
 *
 * @returns Stored theme, or null when missing or invalid
 */
export function readStoredTheme(): ThemeName | null {
  try {
    const raw = localStorage.getItem(THEME_STORAGE_KEY);
    if (!raw) {
      return null;
    }
    const parsed: unknown = JSON.parse(raw);
    if (
      typeof parsed === "object" &&
      parsed !== null &&
      "theme" in parsed &&
      isThemeName((parsed as ThemeRecord).theme)
    ) {
      return (parsed as ThemeRecord).theme;
    }
    return null;
  } catch {
    return null;
  }
}

/**
 * Persist only the theme name under the versioned key.
 *
 * @param theme - Light or dark token set
 */
export function writeStoredTheme(theme: ThemeName): void {
  try {
    const record: ThemeRecord = { theme };
    localStorage.setItem(THEME_STORAGE_KEY, JSON.stringify(record));
  } catch {
    return;
  }
}

/**
 * Swap the document element's dark class so `@theme` roles resolve from the matching set.
 *
 * @param theme - Light or dark token set
 */
export function applyTheme(theme: ThemeName): void {
  const root = document.documentElement;
  if (theme === "dark") {
    root.classList.add(THEME_DARK_CLASS);
  } else {
    root.classList.remove(THEME_DARK_CLASS);
  }
}

/**
 * Flip light and dark.
 *
 * @param theme - Current token set
 * @returns The other token set
 */
export function toggleTheme(theme: ThemeName): ThemeName {
  return theme === "dark" ? "light" : "dark";
}
