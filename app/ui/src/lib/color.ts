import type { Rgb } from "./types";

export function toHex(c: Rgb): string {
  const h = (v: number) => Math.round(v).toString(16).padStart(2, "0");
  return `#${h(c.r)}${h(c.g)}${h(c.b)}`.toUpperCase();
}

export function fromHex(s: string): Rgb {
  const v = parseInt(s.replace("#", ""), 16) || 0;
  return { r: (v >> 16) & 255, g: (v >> 8) & 255, b: v & 255 };
}

export function scale(c: Rgb, f: number): Rgb {
  const k = Math.min(1, Math.max(0, f));
  return { r: Math.round(c.r * k), g: Math.round(c.g * k), b: Math.round(c.b * k) };
}

export function lerp(a: Rgb, b: Rgb, t: number): Rgb {
  const k = Math.min(1, Math.max(0, t));
  return {
    r: Math.round(a.r + (b.r - a.r) * k),
    g: Math.round(a.g + (b.g - a.g) * k),
    b: Math.round(a.b + (b.b - a.b) * k),
  };
}

export function hsv(h: number, s: number, v: number): Rgb {
  const hh = (((h % 360) + 360) % 360) / 60;
  const c = v * s;
  const x = c * (1 - Math.abs((hh % 2) - 1));
  const m = v - c;
  const [r, g, b] =
    hh < 1 ? [c, x, 0] : hh < 2 ? [x, c, 0] : hh < 3 ? [0, c, x] : hh < 4 ? [0, x, c] : hh < 5 ? [x, 0, c] : [c, 0, x];
  return { r: Math.round((r + m) * 255), g: Math.round((g + m) * 255), b: Math.round((b + m) * 255) };
}

/** A color readable on top of `hex` (for text on swatches). */
export function contrastText(hex: string): string {
  const c = fromHex(hex);
  const lum = 0.2126 * c.r + 0.7152 * c.g + 0.0722 * c.b;
  return lum > 150 ? "#111" : "#fff";
}
