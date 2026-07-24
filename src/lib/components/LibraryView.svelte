<script lang="ts">
  import type { Book, LibraryRoot, ScanResult } from "$lib/types";
  import { api } from "$lib/api";
  import { t } from "$lib/i18n/index.svelte";
  import type { Locale } from "$lib/i18n/index.svelte";
  import type { ThemeId } from "$lib/prefs.svelte";
  import AppChromePrefs from "$lib/components/AppChromePrefs.svelte";

  interface Props {
    books: Book[];
    roots: LibraryRoot[];
    busy: boolean;
    status: string;
    onAddFolder: () => void;
    onImportFiles: () => void;
    onScan: () => void;
    onRemoveRoot: (id: string) => void;
    onOpenBook: (book: Book) => void;
    onLocaleChange: (locale: Locale) => void;
    onThemeChange: (theme: ThemeId) => void;
    lastScan: ScanResult | null;
  }

  let {
    books,
    roots,
    busy,
    status,
    onAddFolder,
    onImportFiles,
    onScan,
    onRemoveRoot,
    onOpenBook,
    onLocaleChange,
    onThemeChange,
    lastScan,
  }: Props = $props();

  let covers = $state<Record<string, string>>({});

  $effect(() => {
    const ids = books.map((b) => b.id);
    void loadCovers(ids);
  });

  async function loadCovers(ids: string[]) {
    for (const id of ids) {
      if (covers[id]) continue;
      try {
        const url = await api.getCoverDataUrl(id);
        if (url) {
          covers = { ...covers, [id]: url };
        }
      } catch {
        // ignore missing covers
      }
    }
  }

  function formatLabel(format: string): string {
    switch (format) {
      case "markdown":
        return "MD";
      case "txt":
        return "TXT";
      case "epub":
        return "EPUB";
      case "fb2":
        return "FB2";
      default:
        return format.toUpperCase();
    }
  }
</script>

<section class="library">
  <header class="toolbar">
    <div>
      <h1>{t("library")}</h1>
      <p class="subtitle">{t("formatsLine")}</p>
    </div>
    <div class="actions">
      <AppChromePrefs
        disabled={busy}
        onLocaleChange={onLocaleChange}
        onThemeChange={onThemeChange}
      />
      <button type="button" onclick={onAddFolder} disabled={busy}>{t("addFolder")}</button>
      <button type="button" class="secondary" onclick={onImportFiles} disabled={busy}
        >{t("importFiles")}</button
      >
      <button type="button" class="secondary" onclick={onScan} disabled={busy}>{t("scan")}</button>
    </div>
  </header>

  {#if status}
    <p class="status" class:busy>{status}</p>
  {/if}

  {#if lastScan}
    <p class="scan-meta">
      {t("lastScan", {
        added: lastScan.added,
        updated: lastScan.updated,
        removed: lastScan.removed,
        total: lastScan.total,
      })}
    </p>
  {/if}

  {#if roots.length > 0}
    <div class="roots">
      <h2>{t("folders")}</h2>
      <ul>
        {#each roots as root (root.id)}
          <li>
            <span title={root.path}>{root.path}</span>
            <button type="button" class="link" onclick={() => onRemoveRoot(root.id)}
              >{t("remove")}</button
            >
          </li>
        {/each}
      </ul>
    </div>
  {/if}

  {#if books.length === 0}
    <div class="empty">
      <h2>{t("noBooksTitle")}</h2>
      <p>{t("noBooksBody")}</p>
      <p class="hint" style="white-space: pre-line">{t("noBooksHint")}</p>
    </div>
  {:else}
    <div class="grid">
      {#each books as book (book.id)}
        <button type="button" class="card" onclick={() => onOpenBook(book)}>
          <div class="cover" data-format={book.format}>
            {#if covers[book.id]}
              <img src={covers[book.id]} alt="" />
            {:else}
              <span>{formatLabel(book.format)}</span>
            {/if}
          </div>
          <div class="meta">
            <h3>{book.title}</h3>
            <p class="author">{book.author ?? t("unknownAuthor")}</p>
            <div class="row">
              <span class="badge">{t("ready")}</span>
              {#if book.progressPercentage > 0}
                <span class="progress">{Math.round(book.progressPercentage)}%</span>
              {/if}
            </div>
          </div>
        </button>
      {/each}
    </div>
  {/if}
</section>

<style>
  .library {
    height: 100%;
    overflow: auto;
    padding: 1.5rem 2rem 3rem;
    background: var(--bg);
    color: var(--text);
  }

  .toolbar {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: flex-start;
    margin-bottom: 1rem;
  }

  h1 {
    margin: 0;
    font-size: 1.75rem;
    letter-spacing: -0.02em;
  }

  h2 {
    margin: 0 0 0.5rem;
    font-size: 0.95rem;
    text-transform: uppercase;
    letter-spacing: 0.06em;
    color: var(--muted);
  }

  .subtitle {
    margin: 0.25rem 0 0;
    color: var(--muted);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    align-items: center;
  }

  button {
    border: 1px solid transparent;
    border-radius: 10px;
    padding: 0.55rem 0.95rem;
    font: inherit;
    font-weight: 600;
    cursor: pointer;
    background: var(--accent);
    color: white;
  }

  button:disabled {
    opacity: 0.55;
    cursor: not-allowed;
  }

  button.secondary {
    background: var(--surface-2);
    color: var(--text);
    border-color: var(--border);
  }

  button.link {
    background: transparent;
    color: var(--accent);
    border: none;
    padding: 0;
    font-weight: 500;
  }

  .status {
    margin: 0 0 0.75rem;
    color: var(--muted);
  }

  .status.busy {
    color: var(--accent);
  }

  .scan-meta {
    margin: 0 0 1rem;
    font-size: 0.9rem;
    color: var(--muted);
  }

  .roots {
    margin-bottom: 1.5rem;
    padding: 1rem;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
  }

  .roots ul {
    list-style: none;
    margin: 0;
    padding: 0;
    display: grid;
    gap: 0.5rem;
  }

  .roots li {
    display: flex;
    justify-content: space-between;
    gap: 1rem;
    align-items: center;
    font-size: 0.92rem;
  }

  .roots li span {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .empty {
    margin-top: 3rem;
    text-align: center;
    color: var(--muted);
  }

  .empty h2 {
    color: var(--text);
    text-transform: none;
    letter-spacing: 0;
    font-size: 1.35rem;
  }

  .hint {
    font-size: 0.92rem;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(220px, 1fr));
    gap: 1rem;
  }

  .card {
    display: flex;
    flex-direction: column;
    text-align: left;
    padding: 0;
    overflow: hidden;
    background: var(--surface);
    border: 1px solid var(--border);
    border-radius: 16px;
    color: inherit;
    transition:
      transform 0.15s ease,
      border-color 0.15s ease;
  }

  .card:hover {
    transform: translateY(-2px);
    border-color: var(--accent);
  }

  .cover {
    height: 140px;
    display: grid;
    place-items: center;
    background: linear-gradient(145deg, #3d4f6f, #1f2937);
    color: white;
    font-weight: 700;
    letter-spacing: 0.08em;
    overflow: hidden;
  }

  .cover img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .cover[data-format="markdown"] {
    background: linear-gradient(145deg, #0f766e, #115e59);
  }

  .cover[data-format="txt"] {
    background: linear-gradient(145deg, #57534e, #292524);
  }

  .cover[data-format="epub"] {
    background: linear-gradient(145deg, #1d4ed8, #1e3a8a);
  }

  .cover[data-format="fb2"] {
    background: linear-gradient(145deg, #b45309, #7c2d12);
  }

  .meta {
    padding: 0.9rem 1rem 1rem;
  }

  .meta h3 {
    margin: 0;
    font-size: 1rem;
    line-height: 1.35;
  }

  .author {
    margin: 0.35rem 0 0.7rem;
    color: var(--muted);
    font-size: 0.9rem;
  }

  .row {
    display: flex;
    justify-content: space-between;
    align-items: center;
    gap: 0.5rem;
  }

  .badge,
  .progress {
    font-size: 0.78rem;
    font-weight: 600;
  }

  .badge {
    color: var(--muted);
  }

  .progress {
    color: var(--accent);
  }
</style>
