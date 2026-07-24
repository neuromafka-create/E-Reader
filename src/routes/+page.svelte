<script lang="ts">
  import { onDestroy, onMount } from "svelte";
  import { open } from "@tauri-apps/plugin-dialog";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { api } from "$lib/api";
  import type { Book, BookContent, LibraryRoot, ScanResult } from "$lib/types";
  import LibraryView from "$lib/components/LibraryView.svelte";
  import ReaderView from "$lib/components/ReaderView.svelte";
  import {
    formatIngestMessage,
    getLocale,
    t,
    type Locale,
  } from "$lib/i18n/index.svelte";
  import {
    getTheme,
    loadPrefs,
    setUiLocale,
    setUiTheme,
    type ThemeId,
  } from "$lib/prefs.svelte";

  let books = $state<Book[]>([]);
  let roots = $state<LibraryRoot[]>([]);
  let busy = $state(false);
  let status = $state("");
  let lastScan = $state<ScanResult | null>(null);
  let content = $state<BookContent | null>(null);
  let dragActive = $state(false);

  // Re-render chrome when shared prefs change
  const _locale = $derived(getLocale());
  const _theme = $derived(getTheme());

  let unlistenDrag: (() => void) | null = null;

  onMount(() => {
    void bootstrap();
    return () => {
      unlistenDrag?.();
    };
  });

  onDestroy(() => {
    unlistenDrag?.();
  });

  async function bootstrap() {
    await loadPrefs();
    if (typeof document !== "undefined") {
      document.title = t("appTitle");
    }
    await refresh();
    await setupDragDrop();
    await handleLaunchPaths();
  }

  async function onLocaleChange(locale: Locale) {
    try {
      await setUiLocale(locale);
      if (typeof document !== "undefined") {
        document.title = t("appTitle");
      }
    } catch (e) {
      status = String(e);
    }
  }

  async function onThemeChange(theme: ThemeId) {
    try {
      await setUiTheme(theme);
    } catch (e) {
      status = String(e);
    }
  }

  async function setupDragDrop() {
    try {
      const win = getCurrentWindow();
      unlistenDrag = await win.onDragDropEvent((event) => {
        const payload = event.payload;
        if (payload.type === "enter" || payload.type === "over") {
          dragActive = true;
        } else if (payload.type === "leave") {
          dragActive = false;
        } else if (payload.type === "drop") {
          dragActive = false;
          const paths = payload.paths ?? [];
          if (paths.length) {
            void ingestAndMaybeOpen(paths, { openFirst: true });
          }
        }
      });
    } catch (e) {
      console.warn("Drag-drop unavailable:", e);
    }
  }

  async function handleLaunchPaths() {
    try {
      const paths = await api.getLaunchPaths();
      if (paths.length === 0) return;
      await ingestAndMaybeOpen(paths, { openFirst: true });
    } catch (e) {
      status = String(e);
    }
  }

  async function refresh() {
    try {
      roots = await api.listLibraryRoots();
      books = await api.listBooks();
    } catch (e) {
      status = String(e);
    }
  }

  async function ingestAndMaybeOpen(
    paths: string[],
    opts: { openFirst: boolean },
  ) {
    if (!paths.length) return;
    busy = true;
    status = t("statusImporting");
    try {
      const result = await api.ingestPaths(paths);
      await refresh();
      status = formatIngestMessage(result);

      if (opts.openFirst && result.openBookId) {
        content = await api.openBook(result.openBookId);
        status = "";
        await refresh();
      }
    } catch (e) {
      status = String(e);
    } finally {
      busy = false;
    }
  }

  async function addFolder() {
    const selected = await open({
      directory: true,
      multiple: false,
      title: t("dialogChooseFolder"),
    });
    if (!selected || Array.isArray(selected)) return;

    await ingestAndMaybeOpen([selected], { openFirst: false });
  }

  async function importFiles() {
    const selected = await open({
      multiple: true,
      title: t("dialogImportFiles"),
      filters: [
        {
          name: t("dialogBooksFilter"),
          extensions: ["txt", "md", "markdown", "epub", "fb2"],
        },
      ],
    });
    if (!selected) return;
    const paths = Array.isArray(selected) ? selected : [selected];
    await ingestAndMaybeOpen(paths, { openFirst: paths.length === 1 });
  }

  async function scan() {
    busy = true;
    status = t("statusScanning");
    try {
      lastScan = await api.scanLibrary();
      await refresh();
      status = t("statusScanComplete", { total: lastScan.total });
    } catch (e) {
      status = String(e);
    } finally {
      busy = false;
    }
  }

  async function removeRoot(id: string) {
    busy = true;
    try {
      await api.removeLibraryRoot(id);
      lastScan = await api.scanLibrary();
      await refresh();
      status = t("statusFolderRemoved");
    } catch (e) {
      status = String(e);
    } finally {
      busy = false;
    }
  }

  async function openBook(book: Book) {
    busy = true;
    status = t("statusOpening", { title: book.title });
    try {
      content = await api.openBook(book.id);
      status = "";
      await refresh();
    } catch (e) {
      status = String(e);
    } finally {
      busy = false;
    }
  }

  async function backToLibrary() {
    content = null;
    await refresh();
  }
</script>

<div
  class="app-root"
  class:drag-active={dragActive}
  data-locale={_locale}
  data-theme={_theme}
>
  {#if content}
    <ReaderView
      {content}
      onBack={backToLibrary}
      onLocaleChange={onLocaleChange}
      onThemeChange={onThemeChange}
    />
  {:else}
    <LibraryView
      {books}
      {roots}
      {busy}
      {status}
      {lastScan}
      onAddFolder={addFolder}
      onImportFiles={importFiles}
      onScan={scan}
      onRemoveRoot={removeRoot}
      onOpenBook={openBook}
      onLocaleChange={onLocaleChange}
      onThemeChange={onThemeChange}
    />
  {/if}

  {#if dragActive}
    <div class="drop-overlay" aria-live="polite">
      <div class="drop-card">
        <strong>{t("dropTitle")}</strong>
        <p>{t("dropHint")}</p>
      </div>
    </div>
  {/if}
</div>

<style>
  .app-root {
    height: 100%;
    position: relative;
    background: var(--bg);
    color: var(--text);
  }

  .drop-overlay {
    position: absolute;
    inset: 0;
    z-index: 50;
    display: grid;
    place-items: center;
    background: color-mix(in srgb, var(--accent) 28%, transparent);
    border: 3px dashed var(--accent);
    pointer-events: none;
  }

  .drop-card {
    background: var(--drop-card-bg);
    color: var(--drop-card-text);
    padding: 1.5rem 2rem;
    border-radius: 16px;
    text-align: center;
    box-shadow: 0 12px 40px rgba(0, 0, 0, 0.18);
    max-width: 22rem;
  }

  .drop-card strong {
    font-size: 1.15rem;
  }

  .drop-card p {
    margin: 0.45rem 0 0;
    color: var(--drop-card-muted);
    font-size: 0.95rem;
  }
</style>
