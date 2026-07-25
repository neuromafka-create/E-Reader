import {
  catalogs,
  detectDefaultLocale,
  isLocale,
  type Locale,
  type MessageKey,
  LOCALES,
} from "./messages";
import type { IngestResult } from "$lib/types";

export type { Locale, MessageKey };
export { LOCALES };

// Fixed initial locale for SSR/prerender + first client paint.
// Detecting navigator language here caused hydration mismatches in the
// installed Tauri build (blank white window). Real preference is applied
// in loadPrefs() after mount.
let locale: Locale = $state("ru");
let ready = $state(false);

export function getLocale(): Locale {
  return locale;
}

export function isI18nReady(): boolean {
  return ready;
}

export function setLocale(next: Locale) {
  locale = next;
  if (typeof document !== "undefined") {
    document.documentElement.lang = next;
  }
}

export function markI18nReady() {
  ready = true;
}

export function t(key: MessageKey, params?: Record<string, string | number>): string {
  const dict = catalogs[locale] ?? catalogs.ru;
  let text = dict[key] ?? catalogs.en[key] ?? key;
  if (params) {
    for (const [k, v] of Object.entries(params)) {
      text = text.replaceAll(`{${k}}`, String(v));
    }
  }
  return text;
}

export function applyLocaleFromSettings(value: string | null | undefined) {
  if (isLocale(value)) {
    setLocale(value);
  } else {
    setLocale(detectDefaultLocale());
  }
  markI18nReady();
}

/** Build a localized status string from structured ingest result. */
export function formatIngestMessage(result: IngestResult): string {
  const { filesImported, foldersAdded, skipped } = result;

  if (filesImported === 0 && foldersAdded === 0) {
    if (skipped > 0) {
      return t("ingestNothingSkipped", { skipped });
    }
    return t("ingestNothing");
  }

  const parts: string[] = [];
  if (filesImported > 0) {
    parts.push(t("ingestFiles", { n: filesImported }));
  }
  if (foldersAdded > 0) {
    parts.push(t("ingestFolders", { n: foldersAdded }));
  }

  let msg = t("ingestImported", { parts: parts.join(t("and")) });
  if (skipped > 0) {
    msg += t("ingestSkipped", { n: skipped });
  }
  if (!msg.trim().endsWith(".")) {
    msg = `${msg}.`;
  }
  return msg.replace(/\.\.+$/, ".");
}
