<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import type { BookContent, Bookmark, TocEntry } from "$lib/types";
  import { api } from "$lib/api";
  import {
    activeAnchorId,
    capturePosition,
    captureTextSelection,
    encodePosition,
    encodeSelectionBookmark,
    jumpToAnchor,
    jumpToSelection,
    parseSelectionBookmark,
    restorePosition,
    selectionLabel,
    selectionQuoteLabel,
  } from "$lib/position";
  import { t, type Locale } from "$lib/i18n/index.svelte";
  import {
    getSettings,
    getTheme,
    patchPrefs,
    type ThemeId,
  } from "$lib/prefs.svelte";
  import AppChromePrefs from "$lib/components/AppChromePrefs.svelte";

  interface Props {
    content: BookContent;
    onBack: () => void;
    onLocaleChange: (locale: Locale) => void;
    onThemeChange: (theme: ThemeId) => void;
  }

  let { content, onBack, onLocaleChange, onThemeChange }: Props = $props();

  // Reader-only controls; theme/locale come from shared prefs.
  let fontSize = $state(18);
  let fontFamily = $state("Georgia, 'Times New Roman', serif");
  let lineHeight = $state(1.7);
  let maxWidth = $state(720);

  const theme = $derived(getTheme());

  let bookmarks = $state<Bookmark[]>([]);
  let showSidebar = $state(true);
  let status = $state("");
  let progressPct = $state(0);
  let activeTocId = $state<string | null>(null);
  let articleEl: HTMLElement | null = $state(null);
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let restored = false;

  const tocIds = $derived(content.toc.map((t) => t.id));
  const activeTocTitle = $derived(
    content.toc.find((t) => t.id === activeTocId)?.title ?? null,
  );

  onMount(async () => {
    progressPct = content.progress?.percentage ?? 0;
    try {
      const s = getSettings();
      fontSize = s.fontSize;
      fontFamily = s.fontFamily;
      lineHeight = s.lineHeight;
      maxWidth = s.maxWidth;
      if (content.readable) {
        bookmarks = await api.listBookmarks(content.book.id);
      }
    } catch (e) {
      status = String(e);
    }

    requestAnimationFrame(() => {
      requestAnimationFrame(() => {
        restoreNow();
      });
    });

    window.addEventListener("keydown", onKeyDown);
  });

  onDestroy(() => {
    window.removeEventListener("keydown", onKeyDown);
    if (saveTimer) clearTimeout(saveTimer);
    void persistProgress();
  });

  function restoreNow() {
    if (!articleEl || !content.readable || restored) return;
    const raw = content.progress?.position;
    if (raw) {
      const doc = restorePosition(articleEl, raw);
      progressPct = doc * 100;
    }
    updateActiveToc();
    restored = true;
  }

  function onScroll() {
    if (!articleEl) return;
    updateActiveToc();
    const max = Math.max(0, articleEl.scrollHeight - articleEl.clientHeight);
    progressPct = max > 0 ? (articleEl.scrollTop / max) * 100 : 0;

    if (saveTimer) clearTimeout(saveTimer);
    saveTimer = setTimeout(() => {
      void persistProgress();
    }, 350);
  }

  function updateActiveToc() {
    if (!articleEl) return;
    activeTocId = activeAnchorId(articleEl, tocIds);
  }

  async function persistProgress() {
    if (!articleEl || !content.readable) return;
    const pos = capturePosition(articleEl);
    const percentage = pos.doc * 100;
    progressPct = percentage;
    try {
      await api.saveProgress(content.book.id, encodePosition(pos), percentage);
    } catch (e) {
      status = String(e);
    }
  }

  async function persistReaderTypography() {
    try {
      await patchPrefs({
        fontSize,
        fontFamily,
        lineHeight,
        maxWidth,
      });
      if (articleEl && restored) {
        const pos = capturePosition(articleEl);
        requestAnimationFrame(() => {
          if (!articleEl) return;
          const max = Math.max(0, articleEl.scrollHeight - articleEl.clientHeight);
          articleEl.scrollTop = pos.doc * max;
          void persistProgress();
        });
      }
    } catch (e) {
      status = String(e);
    }
  }

  async function addBookmarkHere() {
    if (!articleEl || !content.readable) return;

    const sel = captureTextSelection(articleEl);
    if (!sel) {
      status = t("bookmarkNeedSelection");
      return;
    }

    const position = encodeSelectionBookmark(sel);
    const label = selectionLabel(sel.quote, sel.page, t("pageAbbr"));
    try {
      const bm = await api.addBookmark(content.book.id, position, label);
      bookmarks = [bm, ...bookmarks];
      status = t("bookmarkAdded", { label });
      // Clear selection so UI feels done
      window.getSelection()?.removeAllRanges();
    } catch (e) {
      status = String(e);
    }
  }

  async function removeBookmark(id: string) {
    try {
      await api.removeBookmark(id);
      bookmarks = bookmarks.filter((b) => b.id !== id);
    } catch (e) {
      status = String(e);
    }
  }

  /** Quote only (for the «…» part). */
  function bookmarkQuote(bm: Bookmark): string {
    const sel = parseSelectionBookmark(bm.position);
    if (sel?.quote) return selectionQuoteLabel(sel.quote);
    // Legacy label may be "quote · стр. N" — strip page suffix if present
    const raw = bm.label?.trim() ?? "";
    if (!raw) return bm.position;
    return raw.replace(/\s*[·•]\s*(стр\.|p\.)\s*\d+\s*$/i, "").trim() || raw;
  }

  function bookmarkPage(bm: Bookmark): number | null {
    const sel = parseSelectionBookmark(bm.position);
    if (sel && sel.page > 0) return sel.page;
    const m = bm.label?.match(/(?:стр\.|p\.)\s*(\d+)/i);
    if (m) return Number(m[1]);
    return null;
  }

  function bookmarkDisplay(bm: Bookmark): string {
    const quote = bookmarkQuote(bm);
    const page = bookmarkPage(bm);
    return selectionLabel(quote, page, t("pageAbbr"));
  }

  function jumpTo(position: string) {
    if (!articleEl) return;
    if (parseSelectionBookmark(position)) {
      jumpToSelection(articleEl, position, true);
    } else {
      restorePosition(articleEl, position);
    }
    updateActiveToc();
    void persistProgress();
  }

  function jumpToc(entry: TocEntry) {
    if (!articleEl) return;
    jumpToAnchor(articleEl, entry.id, true);
    activeTocId = entry.id;
    setTimeout(() => void persistProgress(), 400);
  }

  function pageScroll(dir: 1 | -1) {
    if (!articleEl) return;
    const step = Math.max(120, articleEl.clientHeight * 0.9);
    articleEl.scrollBy({ top: dir * step, behavior: "smooth" });
  }

  function onKeyDown(e: KeyboardEvent) {
    const tag = (e.target as HTMLElement | null)?.tagName;
    if (tag === "INPUT" || tag === "TEXTAREA" || tag === "SELECT") return;

    switch (e.key) {
      case "Escape":
        e.preventDefault();
        onBack();
        break;
      case "ArrowRight":
      case "PageDown":
      case " ":
        e.preventDefault();
        pageScroll(1);
        break;
      case "ArrowLeft":
      case "PageUp":
        e.preventDefault();
        pageScroll(-1);
        break;
      case "Home":
        if (articleEl) {
          e.preventDefault();
          articleEl.scrollTop = 0;
        }
        break;
      case "End":
        if (articleEl) {
          e.preventDefault();
          articleEl.scrollTop = articleEl.scrollHeight;
        }
        break;
      case "b":
      case "B":
        if (e.ctrlKey || e.metaKey) return;
        e.preventDefault();
        void addBookmarkHere();
        break;
      default:
        break;
    }
  }
</script>

<section class="reader" data-theme={theme}>
  <header class="bar">
    <button type="button" class="ghost" onclick={onBack}>{t("backToLibrary")}</button>
    <div class="title-block">
      <h1>{content.book.title}</h1>
      <span class="format">
        {content.book.format}
        {#if content.book.author}
          · {content.book.author}
        {/if}
      </span>
    </div>
    <div class="controls">
      <span class="progress-pill" title={activeTocTitle ?? t("readingProgress")}>
        {Math.round(progressPct)}%
      </span>
      <label>
        {t("fontSize")}
        <input
          type="range"
          min="14"
          max="28"
          bind:value={fontSize}
          onchange={persistReaderTypography}
        />
      </label>
      <AppChromePrefs
        compact
        onLocaleChange={onLocaleChange}
        onThemeChange={onThemeChange}
      />
      <button type="button" class="ghost" onclick={() => (showSidebar = !showSidebar)}>
        {showSidebar ? t("hidePanel") : t("showPanel")}
      </button>
      {#if content.readable}
        <button type="button" onclick={addBookmarkHere} title={t("bookmarkShortcut")}
          >{t("bookmark")}</button
        >
      {/if}
    </div>
  </header>

  <div class="progress-track" aria-hidden="true">
    <div class="progress-fill" style={`width: ${progressPct}%`}></div>
  </div>

  {#if status}
    <p class="status">{status}</p>
  {/if}

  <div class="body">
    {#if showSidebar}
      <aside class="sidebar">
        {#if content.toc.length > 0}
          <h2>{t("contents")}</h2>
          <ul class="toc">
            {#each content.toc as entry (entry.id + entry.title)}
              <li style={`padding-left: ${(entry.level - 1) * 0.75}rem`}>
                <button
                  type="button"
                  class="link"
                  class:active={activeTocId === entry.id}
                  onclick={() => jumpToc(entry)}>{entry.title}</button
                >
              </li>
            {/each}
          </ul>
        {/if}

        <h2>{t("bookmarks")}</h2>
        {#if bookmarks.length === 0}
          <p class="muted">{@html t("noBookmarks", { key: "<kbd>B</kbd>" })}</p>
        {:else}
          <ul class="bookmarks">
            {#each bookmarks as bm (bm.id)}
              <li>
                <button
                  type="button"
                  class="link quote"
                  title={bookmarkDisplay(bm)}
                  onclick={() => jumpTo(bm.position)}
                >
                  <span class="quote-text">«{bookmarkQuote(bm)}»</span>
                  {#if bookmarkPage(bm)}
                    <span class="quote-page">{t("pageAbbr")} {bookmarkPage(bm)}</span>
                  {/if}
                </button>
                <button type="button" class="tiny" onclick={() => removeBookmark(bm.id)}
                  >×</button
                >
              </li>
            {/each}
          </ul>
        {/if}

        <p class="hints">
          {@html t("readerHints", {
            left: "<kbd>←</kbd>",
            right: "<kbd>→</kbd>",
            esc: "<kbd>Esc</kbd>",
            b: "<kbd>B</kbd>",
          })}
        </p>
      </aside>
    {/if}

    {#if content.readable}
      <article
        class="page"
        bind:this={articleEl}
        onscroll={onScroll}
        style={`
          font-family: ${fontFamily};
          font-size: ${fontSize}px;
          line-height: ${lineHeight};
          max-width: ${maxWidth}px;
        `}
      >
        {@html content.html}
      </article>
    {:else}
      <div class="unavailable">
        <h2>{t("formatUnavailableTitle")}</h2>
        <p>{content.message}</p>
      </div>
    {/if}
  </div>
</section>

<style>
  .reader {
    height: 100%;
    display: flex;
    flex-direction: column;
    background: var(--reader-bg);
    color: var(--reader-text);
  }

  .bar {
    display: flex;
    align-items: center;
    gap: 1rem;
    padding: 0.75rem 1rem;
    border-bottom: 1px solid var(--reader-border);
    background: color-mix(in srgb, var(--reader-panel) 90%, transparent);
    backdrop-filter: blur(8px);
  }

  .title-block {
    flex: 1;
    min-width: 0;
  }

  .title-block h1 {
    margin: 0;
    font-size: 1rem;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }

  .format {
    font-size: 0.75rem;
    color: var(--reader-muted);
    text-transform: uppercase;
    letter-spacing: 0.06em;
  }

  .controls {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  .progress-pill {
    font-size: 0.8rem;
    font-weight: 700;
    padding: 0.25rem 0.55rem;
    border-radius: 999px;
    background: color-mix(in srgb, var(--reader-accent) 18%, transparent);
    color: var(--reader-accent);
    min-width: 2.75rem;
    text-align: center;
  }

  .progress-track {
    height: 3px;
    background: var(--reader-border);
  }

  .progress-fill {
    height: 100%;
    background: var(--reader-accent);
    transition: width 0.12s linear;
  }

  label {
    display: flex;
    align-items: center;
    gap: 0.35rem;
    font-size: 0.85rem;
    color: var(--reader-muted);
  }

  input[type="range"] {
    accent-color: var(--reader-accent);
  }

  button {
    border: 1px solid transparent;
    border-radius: 8px;
    padding: 0.4rem 0.75rem;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    background: var(--reader-accent);
    color: white;
  }

  button.ghost {
    background: transparent;
    color: var(--reader-text);
    border-color: var(--reader-border);
  }

  button.link {
    background: transparent;
    color: inherit;
    border: none;
    padding: 0.15rem 0.25rem;
    text-align: left;
    font-weight: 500;
    border-radius: 6px;
    width: 100%;
  }

  button.link.active {
    background: color-mix(in srgb, var(--reader-accent) 16%, transparent);
    color: var(--reader-accent);
    font-weight: 700;
  }

  button.tiny {
    background: transparent;
    color: var(--reader-muted);
    border: none;
    padding: 0 0.25rem;
  }

  .status {
    margin: 0;
    padding: 0.4rem 1rem;
    font-size: 0.85rem;
    color: var(--reader-muted);
    border-bottom: 1px solid var(--reader-border);
  }

  .body {
    flex: 1;
    min-height: 0;
    display: grid;
    grid-template-columns: auto 1fr;
  }

  .sidebar {
    width: 280px;
    overflow: auto;
    border-right: 1px solid var(--reader-border);
    background: var(--reader-panel);
    padding: 1rem;
  }

  .sidebar h2 {
    margin: 0 0 0.5rem;
    font-size: 0.8rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--reader-muted);
  }

  .toc,
  .bookmarks {
    list-style: none;
    margin: 0 0 1.25rem;
    padding: 0;
    display: grid;
    gap: 0.25rem;
  }

  .bookmarks li {
    display: flex;
    justify-content: space-between;
    gap: 0.5rem;
    align-items: flex-start;
  }

  .bookmarks .quote {
    font-weight: 500;
    line-height: 1.35;
    display: flex;
    flex-direction: column;
    align-items: flex-start;
    gap: 0.2rem;
  }

  .quote-text {
    font-style: italic;
    display: -webkit-box;
    -webkit-line-clamp: 3;
    line-clamp: 3;
    -webkit-box-orient: vertical;
    overflow: hidden;
  }

  .quote-page {
    font-style: normal;
    font-size: 0.78em;
    font-weight: 700;
    color: var(--reader-muted);
    letter-spacing: 0.02em;
  }

  .page :global(.bookmark-highlight) {
    background: color-mix(in srgb, var(--reader-accent) 35%, transparent);
    border-radius: 2px;
    box-decoration-break: clone;
    -webkit-box-decoration-break: clone;
  }

  .muted {
    color: var(--reader-muted);
    font-size: 0.9rem;
  }

  .hints {
    margin: 1rem 0 0;
    font-size: 0.78rem;
    color: var(--reader-muted);
    line-height: 1.5;
  }

  .hints :global(kbd),
  .muted :global(kbd) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.85em;
    border: 1px solid var(--reader-border);
    border-bottom-width: 2px;
    border-radius: 4px;
    padding: 0 0.3em;
    background: var(--reader-bg);
  }

  .page {
    overflow: auto;
    padding: 2.5rem 2rem 4rem;
    margin: 0 auto;
    width: 100%;
  }

  .page :global(article) {
    max-width: 100%;
  }

  .page :global(h1),
  .page :global(h2),
  .page :global(h3),
  .page :global(h4) {
    line-height: 1.25;
    margin: 1.6em 0 0.6em;
  }

  .page :global(p) {
    margin: 0 0 1em;
  }

  .page :global(pre) {
    overflow: auto;
    padding: 0.9rem;
    border-radius: 10px;
    background: color-mix(in srgb, var(--reader-text) 8%, transparent);
  }

  .page :global(code) {
    font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
    font-size: 0.92em;
  }

  .page :global(blockquote) {
    margin: 1em 0;
    padding-left: 1em;
    border-left: 3px solid var(--reader-border);
    color: var(--reader-muted);
  }

  .page :global(img) {
    max-width: 100%;
    height: auto;
  }

  .page :global(.epub-chapter),
  .page :global(.fb2-section) {
    margin-bottom: 2rem;
  }

  .page :global(.fb2-poem),
  .page :global(.fb2-stanza) {
    margin: 1rem 0;
  }

  .page :global(.fb2-verse) {
    margin: 0.15rem 0;
  }

  .page :global(.fb2-cite) {
    margin: 1em 0;
    padding-left: 1em;
    border-left: 3px solid var(--reader-border);
  }

  .page :global(a) {
    color: var(--reader-accent);
  }

  .unavailable {
    margin: auto;
    text-align: center;
    padding: 3rem;
    color: var(--reader-muted);
  }
</style>
