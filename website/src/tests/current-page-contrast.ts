import { expect } from "vitest";

function hexToRgb(hex: string): [number, number, number] {
  const h = hex.replace("#", "");
  return [
    parseInt(h.slice(0, 2), 16),
    parseInt(h.slice(2, 4), 16),
    parseInt(h.slice(4, 6), 16),
  ];
}

function relativeLuminance(hex: string): number {
  const channels = hexToRgb(hex).map((value) => {
    const c = value / 255;
    return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
  });
  return 0.2126 * channels[0] + 0.7152 * channels[1] + 0.0722 * channels[2];
}

function contrastRatio(foreground: string, background: string): number {
  const first = relativeLuminance(foreground);
  const second = relativeLuminance(background);
  const [hi, lo] = first > second ? [first, second] : [second, first];
  return (hi + 0.05) / (lo + 0.05);
}

function themeColor(css: string, name: string): string {
  const theme = css.match(/@theme\s*\{([\s\S]*?)\}/)?.[1] ?? "";
  return (
    theme.match(new RegExp(`--color-${name}:\\s*(#[0-9A-Fa-f]{6})`))?.[1] ?? ""
  );
}

export function assertCurrentPageInkContrast(
  css: string,
  variants: string,
): void {
  expect(variants).toContain("aria-[current=page]:text-accent-2");
  expect(variants).not.toMatch(/aria-\[current=page\]:text-muted/);
  const canvas = themeColor(css, "canvas");
  const accent2 = themeColor(css, "accent-2");
  const muted = themeColor(css, "muted");
  expect([hexToRgb(accent2), hexToRgb(canvas)]).not.toEqual([
    [47, 158, 111],
    [245, 248, 252],
  ]);
  expect(contrastRatio(accent2, canvas)).toBeGreaterThanOrEqual(4.5);
  expect(hexToRgb(accent2)).not.toEqual(hexToRgb(muted));
}
