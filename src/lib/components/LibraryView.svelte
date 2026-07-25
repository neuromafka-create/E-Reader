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
    onArchiveBook: (book: Book, archived: boolean) => void;
    onDeleteBook: (book: Book) => void;
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
    onArchiveBook,
    onDeleteBook,
    onLocaleChange,
    onThemeChange,
    lastScan,
  }: Props = $props();

  let covers = $state<Record<string, string>>({});

  const activeBooks = $derived(books.filter((b) => !b.archived));
  const archivedBooks = $derived(books.filter((b) => b.archived));

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

  function stopAnd(e: Event, fn: () => void) {
    e.stopPropagation();
    e.preventDefault();
    fn();
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
    {#if activeBooks.length === 0}
      <p class="empty-active">{t("noActiveBooks")}</p>
    {:else}
      <div class="grid">
        {#each activeBooks as book (book.id)}
          <article class="card">
            <button type="button" class="card-main" onclick={() => onOpenBook(book)}>
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
            <div class="card-actions">
              <button
                type="button"
                class="ghost-action"
                disabled={busy}
                onclick={(e) => stopAnd(e, () => onArchiveBook(book, true))}
                >{t("archive")}</button
              >
              <button
                type="button"
                class="ghost-action danger"
                disabled={busy}
                onclick={(e) => stopAnd(e, () => onDeleteBook(book))}
                >{t("deleteBook")}</button
              >
            </div>
          </article>
        {/each}
      </div>
    {/if}

    {#if archivedBooks.length > 0}
      <details class="archive-panel">
        <summary>{t("archiveSection", { n: archivedBooks.length })}</summary>
        <ul class="archive-list">
          {#each archivedBooks as book (book.id)}
            <li>
              <button
                type="button"
                class="archive-open"
                disabled={busy}
                onclick={() => onOpenBook(book)}
                title={book.path}
              >
                <span class="archive-format">{formatLabel(book.format)}</span>
                <span class="archive-title">{book.title}</span>
                <span class="archive-author">{book.author ?? t("unknownAuthor")}</span>
                {#if book.progressPercentage > 0}
                  <span class="archive-progress">{Math.round(book.progressPercentage)}%</span>
                {/if}
              </button>
              <div class="archive-actions">
                <button
                  type="button"
                  class="ghost-action"
                  disabled={busy}
                  onclick={() => onArchiveBook(book, false)}>{t("unarchive")}</button
                >
                <button
                  type="button"
                  class="ghost-action danger"
                  disabled={busy}
                  onclick={() => onDeleteBook(book)}>{t("deleteBook")}</button
                >
              </div>
            </li>
          {/each}
        </ul>
      </details>
    {/if}
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

  .empty-active {
    margin: 1rem 0 1.5rem;
    color: var(--muted);
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

  .card-main {
    display: flex;
    flex-direction: column;
    text-align: left;
    padding: 0;
    flex: 1;
    background: transparent;
    color: inherit;
    border: none;
    border-radius: 0;
    font-weight: inherit;
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
    padding: 0.9rem 1rem 0.65rem;
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

  .card-actions {
    display: flex;
    gap: 0.35rem;
    padding: 0 0.75rem 0.75rem;
  }

  .ghost-action {
    flex: 1;
    background: var(--surface-2);
    color: var(--text);
    border: 1px solid var(--border);
    border-radius: 8px;
    padding: 0.35rem 0.5rem;
    font-size: 0.8rem;
    font-weight: 600;
  }

  .ghost-action.danger {
    color: #b91c1c;
  }

  :global([data-theme="dark"]) .ghost-action.danger {
    color: #fca5a5;
  }

  .archive-panel {
    margin-top: 2rem;
    border: 1px solid var(--border);
    border-radius: 14px;
    background: var(--surface);
    padding: 0.65rem 1rem 0.85rem;
  }

  .archive-panel summary {
    cursor: pointer;
    font-weight: 700;
    font-size: 0.95rem;
    color: var(--muted);
    user-select: none;
    list-style-position: outside;
  }

  .archive-panel summary:hover {
    color: var(--text);
  }

  .archive-list {
    list-style: none;
    margin: 0.75rem 0 0;
    padding: 0;
    display: grid;
    gap: 0.4rem;
  }

  .archive-list li {
    display: flex;
    flex-wrap: wrap;
    align-items: center;
    gap: 0.5rem;
    padding: 0.45rem 0.5rem;
    border-radius: 10px;
    border: 1px solid transparent;
  }

  .archive-list li:hover {
    background: color-mix(in srgb, var(--surface-2) 80%, transparent);
    border-color: var(--border);
  }

  .archive-open {
    flex: 1;
    min-width: 12rem;
    display: flex;
    flex-wrap: wrap;
    align-items: baseline;
    gap: 0.45rem 0.75rem;
    text-align: left;
    background: transparent;
    color: inherit;
    border: none;
    border-radius: 8px;
    padding: 0.25rem 0.35rem;
    font-weight: 500;
  }

  .archive-format {
    font-size: 0.7rem;
    font-weight: 700;
    letter-spacing: 0.06em;
    color: var(--muted);
    min-width: 2.5rem;
  }

  .archive-title {
    font-weight: 600;
  }

  .archive-author {
    color: var(--muted);
    font-size: 0.88rem;
  }

  .archive-progress {
    font-size: 0.78rem;
    font-weight: 600;
    color: var(--accent);
  }

  .archive-actions {
    display: flex;
    gap: 0.35rem;
    flex-shrink: 0;
  }

  .archive-actions .ghost-action {
    flex: 0 0 auto;
  }
</style>
