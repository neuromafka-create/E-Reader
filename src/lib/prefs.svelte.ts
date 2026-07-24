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

export function isTheme(value: string | null | undefined): value is ThemeId {
  return value === "sepia" || value === "light" || value === "dark";
}

let theme: ThemeId = $state("sepia");
let settings: ReaderSettings = $state(defaultSettings());
let loaded = $state(false);

export function defaultSettings(): ReaderSettings {
  return {
    fontFamily: "Georgia, 'Times New Roman', serif",
    fontSize: 18,
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

  if (patch.locale) {
    setLocale(patch.locale as Locale);
    next.locale = getLocale();
  }
  if (patch.theme && isTheme(patch.theme)) {
    applyTheme(patch.theme);
    next.theme = patch.theme;
  }

  settings = next;
  await api.saveReaderSettings(settings);
  return settings;
}

export async function setUiLocale(locale: Locale): Promise<void> {
  setLocale(locale);
  await patchPrefs({ locale });
}

export async function setUiTheme(next: ThemeId): Promise<void> {
  await patchPrefs({ theme: next });
}
