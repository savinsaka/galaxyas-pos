import { api } from "$lib/api";

export interface ThemeDef {
  key: string;
  label: string;
  swatch: string;
}

export const THEMES: ThemeDef[] = [
  { key: "baby-blue", label: "Baby Blue (Default)", swatch: "#4a90d9" },
  { key: "dark", label: "Dark Mode", swatch: "#1b2733" },
  { key: "forest", label: "Forest", swatch: "#2f9e6b" },
  { key: "sunset", label: "Sunset", swatch: "#e08a3b" },
  { key: "grape", label: "Grape", swatch: "#8b5fbf" },
];

const DEFAULT_THEME = "baby-blue";

export function applyTheme(key: string) {
  if (key === DEFAULT_THEME || !THEMES.some((t) => t.key === key)) {
    document.documentElement.removeAttribute("data-theme");
  } else {
    document.documentElement.setAttribute("data-theme", key);
  }
}

/** Dipanggil sekali saat boot aplikasi — ambil tema tersimpan & terapkan. */
export async function loadAndApplyTheme(): Promise<string> {
  try {
    const s = await api.getSettings();
    const key = s.theme || DEFAULT_THEME;
    applyTheme(key);
    return key;
  } catch {
    applyTheme(DEFAULT_THEME);
    return DEFAULT_THEME;
  }
}

export async function saveTheme(key: string): Promise<void> {
  await api.updateSetting("theme", key);
  applyTheme(key);
}
