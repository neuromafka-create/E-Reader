<script lang="ts">
  import {
    LOCALES,
    getLocale,
    t,
    type Locale,
  } from "$lib/i18n/index.svelte";
  import {
    THEMES,
    getTheme,
    type ThemeId,
  } from "$lib/prefs.svelte";

  interface Props {
    disabled?: boolean;
    compact?: boolean;
    onLocaleChange: (locale: Locale) => void;
    onThemeChange: (theme: ThemeId) => void;
  }

  let {
    disabled = false,
    compact = false,
    onLocaleChange,
    onThemeChange,
  }: Props = $props();

  const locale = $derived(getLocale());
  const theme = $derived(getTheme());

  function themeLabel(id: ThemeId): string {
    switch (id) {
      case "sepia":
        return t("themeSepia");
      case "light":
        return t("themeLight");
      case "dark":
        return t("themeDark");
    }
  }

  function onLang(event: Event) {
    onLocaleChange((event.target as HTMLSelectElement).value as Locale);
  }

  function onTheme(event: Event) {
    onThemeChange((event.target as HTMLSelectElement).value as ThemeId);
  }
</script>

<div class="prefs" class:compact>
  <label>
    <span>{t("language")}</span>
    <select value={locale} onchange={onLang} {disabled}>
      {#each LOCALES as item (item.id)}
        <option value={item.id}>{item.label}</option>
      {/each}
    </select>
  </label>
  <label>
    <span>{t("theme")}</span>
    <select value={theme} onchange={onTheme} {disabled}>
      {#each THEMES as id (id)}
        <option value={id}>{themeLabel(id)}</option>
      {/each}
    </select>
  </label>
</div>

<style>
  .prefs {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  label {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.9rem;
    color: var(--muted);
    font-weight: 600;
  }

  .compact label {
    font-size: 0.85rem;
    font-weight: 500;
    color: var(--reader-muted, var(--muted));
  }

  select {
    border: 1px solid var(--border);
    border-radius: 10px;
    padding: 0.45rem 0.6rem;
    font: inherit;
    background: var(--surface);
    color: var(--text);
  }

  .compact select {
    border-radius: 8px;
    padding: 0.25rem 0.4rem;
    border-color: var(--reader-border, var(--border));
    background: var(--reader-bg, var(--surface));
    color: var(--reader-text, var(--text));
  }
</style>
