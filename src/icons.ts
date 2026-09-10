export const nerdIconHex = {
  folder: "f07b",
  folderOpen: "f07c",
  chevronRight: "f054",
  chevronDown: "f078",
  grip: "f142",
  plus: "f067",
  trash: "f1f8",
  refresh: "f2f1",
  palette: "e22b",
  image: "f03e",
  modsFolder: "f07c",
  packagedMods: "f0ffa",
} as const;

export function normalizeIconHex(value: string): string | null {
  const normalized = value.trim().replace(/^U\+/i, "").replace(/^0x/i, "").replace(/^\\u/i, "");
  if (!/^[0-9a-fA-F]{1,6}$/.test(normalized)) return null;
  const codePoint = Number.parseInt(normalized, 16);
  if (codePoint > 0x10ffff || (codePoint >= 0xd800 && codePoint <= 0xdfff)) return null;
  return normalized.toLowerCase();
}

export function nerdGlyph(hex: string | null | undefined, fallback = nerdIconHex.folder): string {
  const normalized = normalizeIconHex(hex ?? "") ?? fallback;
  return String.fromCodePoint(Number.parseInt(normalized, 16));
}

export function normalizeColorHex(value: string): string | null {
  const trimmed = value.trim();
  if (!trimmed) return null;
  const withHash = trimmed.startsWith("#") ? trimmed : `#${trimmed}`;
  if (!/^#[0-9a-fA-F]{6}$/.test(withHash)) return null;
  return withHash.toUpperCase();
}
