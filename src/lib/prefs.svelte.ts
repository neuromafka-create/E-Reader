import type { ReaderSettings } from "$lib/types";
import { api } from "$lib/api";
import {
  applyLocaleFromSettings,
  getLocale,
  setLocale,
  type Locale,
} from "$lib/i18n/index.svelte";

export type ThemeId = "sepia" | "light" | "dark";

export const THEMES: ThemeId[] = ["sepia", "light", "dark"];

/** Built-in reading typefaces (CSS font-family stacks). */
export const FONT_FAMILIES: { value: string; label: string }[] = [
  { value: "Georgia, 'Times New Roman', serif", label: "Georgia" },
  { value: "'Times New Roman', Times, serif", label: "Times New Roman" },
  { value: "'Palatino Linotype', Palatino, 'Book Antiqua', serif", label: "Palatino" },
  { value: "Cambria, Georgia, serif", label: "Cambria" },
  { value: "Arial, Helvetica, sans-serif", label: "Arial" },
  { value: "Verdana, Geneva, sans-serif", label: "Verdana" },
  { value: "Calibri, 'Segoe UI', sans-serif", label: "Calibri" },
  { value: "'Segoe UI', system-ui, sans-serif", label: "Segoe UI" },
  { value: "Consolas, 'Courier New', monospace", label: "Consolas" },
];

export const FONT_SIZE_MIN = 12;
export const FONT_SIZE_MAX = 40;
export const FONT_SIZE_DEFAULT = 18;

export function clampFontSize(value: number): number {
  if (!Number.isFinite(value)) return FONT_SIZE_DEFAULT;
  return Math.min(FONT_SIZE_MAX, Math.max(FONT_SIZE_MIN, Math.round(value)));
}

export function isTheme(value: string | null | undefined): value is ThemeId {
  return value === "sepia" || value === "light" || value === "dark";
}

let theme: ThemeId = $state("sepia");
let settings: ReaderSettings = $state(defaultSettings());
let loaded = $state(false);

export function defaultSettings(): ReaderSettings {
  return {
    fontFamily: FONT_FAMILIES[0].value,
    fontSize: FONT_SIZE_DEFAULT,
    lineHeight: 1.7,
    theme: "sepia",
    maxWidth: 720,
    locale: "ru",
  };
}

export function getTheme(): ThemeId {
  return theme;
}

export function getSettings(): ReaderSettings {
  return settings;
}

export function isPrefsLoaded(): boolean {
  return loaded;
}

export function applyTheme(next: ThemeId) {
  theme = next;
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.dataset.theme = next;
  root.style.colorScheme = next === "dark" ? "dark" : "light";
}

/** Load settings from SQLite and apply locale + theme app-wide. */
export async function loadPrefs(): Promise<ReaderSettings> {
  try {
    const remote = await api.getReaderSettings();
    settings = Object.assign({}, defaultSettings(), remote);
    settings.fontSize = clampFontSize(settings.fontSize);
  } catch {
    settings = defaultSettings();
  }

  applyLocaleFromSettings(settings.locale);
  applyTheme(isTheme(settings.theme) ? settings.theme : "sepia");
  loaded = true;

  if (typeof document !== "undefined") {
    // title set by caller via t()
  }
  return settings;
}

/** Merge patch into shared settings, apply locale/theme, persist. */
export async function patchPrefs(
  patch: Partial<ReaderSettings>,
): Promise<ReaderSettings> {
  const next: ReaderSettings = {
    ...settings,
    ...patch,
  };

  if (patch.fontSize !== undefined) {
    next.fontSize = clampFontSize(Number(patch.fontSize));
  }
  if (patch.locale) {
    setLocale(patch.locale as Locale);
    next.locale = getLocale();
  }
  if (patch.theme && isTheme(patch.theme)) {
    applyTheme(patch.theme);
    next.theme = patch.theme;
  }

  settings = next;
  // Apply UI immediately even if persistence fails (e.g. ACL / IPC issues).
  try {
    await api.saveReaderSettings(settings);
  } catch (e) {
    console.error("saveReaderSettings failed", e);
  }
  return settings;
}

export async function setUiLocale(locale: Locale): Promise<void> {
  // Apply in-memory first so the select always updates the UI.
  setLocale(locale);
  await patchPrefs({ locale });
}

export async function setUiTheme(next: ThemeId): Promise<void> {
  applyTheme(next);
  await patchPrefs({ theme: next });
}
