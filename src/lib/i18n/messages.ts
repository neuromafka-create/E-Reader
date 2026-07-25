export type Locale = "ru" | "en";

export const LOCALES: { id: Locale; label: string }[] = [
  { id: "ru", label: "Русский" },
  { id: "en", label: "English" },
];

export type MessageKey =
  | "appTitle"
  | "library"
  | "formatsLine"
  | "addFolder"
  | "importFiles"
  | "scan"
  | "folders"
  | "remove"
  | "noBooksTitle"
  | "noBooksBody"
  | "noBooksHint"
  | "ready"
  | "unknownAuthor"
  | "lastScan"
  | "archive"
  | "unarchive"
  | "deleteBook"
  | "archiveSection"
  | "confirmDeleteBook"
  | "statusBookArchived"
  | "statusBookRestored"
  | "statusBookDeleted"
  | "noActiveBooks"
  | "language"
  | "backToLibrary"
  | "readingProgress"
  | "fontSize"
  | "fontFamily"
  | "theme"
  | "themeSepia"
  | "themeLight"
  | "themeDark"
  | "hidePanel"
  | "showPanel"
  | "bookmark"
  | "bookmarkShortcut"
  | "bookmarkAdded"
  | "bookmarkNeedSelection"
  | "pageAbbr"
  | "contents"
  | "bookmarks"
  | "noBookmarks"
  | "readerHints"
  | "formatUnavailableTitle"
  | "dropTitle"
  | "dropHint"
  | "dialogChooseFolder"
  | "dialogImportFiles"
  | "dialogBooksFilter"
  | "statusImporting"
  | "statusScanning"
  | "statusScanComplete"
  | "statusFolderRemoved"
  | "statusOpening"
  | "statusDownloading"
  | "linkImported"
  | "linkOpenedExternally"
  | "linkImportFailed"
  | "ingestNothingSkipped"
  | "ingestNothing"
  | "ingestImported"
  | "ingestFiles"
  | "ingestFolders"
  | "ingestSkipped"
  | "and";

export type Messages = Record<MessageKey, string>;

export const ru: Messages = {
  appTitle: "E-Reader",
  library: "Библиотека",
  formatsLine: "TXT · Markdown · EPUB · FB2",
  addFolder: "Добавить папку",
  importFiles: "Импорт файлов",
  scan: "Сканировать",
  folders: "Папки",
  remove: "Удалить",
  noBooksTitle: "Пока нет книг",
  noBooksBody:
    "Добавьте папку с файлами .txt, .md, .epub или .fb2 либо импортируйте файлы напрямую.",
  noBooksHint:
    "Поддерживаемые форматы: TXT, Markdown, EPUB, FB2.\nПодсказка: перетащите файлы или папки сюда, либо откройте книгу через «Открыть с помощью».",
  ready: "Готово",
  unknownAuthor: "Автор неизвестен",
  lastScan: "Последнее сканирование: +{added} · ~{updated} · −{removed} · всего {total}",
  archive: "В архив",
  unarchive: "Из архива",
  deleteBook: "Удалить",
  archiveSection: "Архив ({n})",
  confirmDeleteBook:
    "Удалить «{title}» из библиотеки? Прогресс и закладки будут потеряны. Файлы в папках библиотеки на диске не трогаем.",
  statusBookArchived: "«{title}» перенесена в архив",
  statusBookRestored: "«{title}» возвращена в библиотеку",
  statusBookDeleted: "«{title}» удалена из библиотеки",
  noActiveBooks: "Активных книг нет — они в архиве или ещё не добавлены.",
  language: "Язык",
  backToLibrary: "← Библиотека",
  readingProgress: "Прогресс чтения",
  fontSize: "Размер",
  fontFamily: "Шрифт",
  theme: "Тема",
  themeSepia: "Сепия",
  themeLight: "Светлая",
  themeDark: "Тёмная",
  hidePanel: "Скрыть панель",
  showPanel: "Панель",
  bookmark: "Закладка",
  bookmarkShortcut: "Выделите текст и нажмите B",
  bookmarkAdded: "Закладка: «{label}»",
  bookmarkNeedSelection: "Выделите фрагмент текста в книге, затем нажмите B",
  pageAbbr: "стр.",
  contents: "Оглавление",
  bookmarks: "Закладки",
  noBookmarks: "Закладок пока нет. Выделите текст и нажмите {key}.",
  readerHints: "{left}/{right} страница · {esc} библиотека · выделить + {b} закладка",
  formatUnavailableTitle: "Формат пока нельзя прочитать",
  dropTitle: "Отпустите, чтобы открыть",
  dropHint: "Книги (.epub, .fb2, .md, .txt) или папка библиотеки",
  dialogChooseFolder: "Выберите папку библиотеки",
  dialogImportFiles: "Импорт книг",
  dialogBooksFilter: "Книги",
  statusImporting: "Импорт…",
  statusScanning: "Сканирование библиотеки…",
  statusScanComplete: "Сканирование завершено: {total} книг.",
  statusFolderRemoved: "Папка удалена, библиотека пересканирована.",
  statusOpening: "Открытие «{title}»…",
  statusDownloading: "Скачивание книги…",
  linkImported: "Книга добавлена в библиотеку: «{title}»",
  linkOpenedExternally: "Ссылка открыта во внешнем браузере",
  linkImportFailed: "Не удалось скачать книгу. {detail}",
  ingestNothingSkipped: "Ничего не импортировано (пропущено путей: {skipped}).",
  ingestNothing: "Нечего импортировать",
  ingestImported: "Импортировано: {parts}",
  ingestFiles: "{n} файл(ов)",
  ingestFolders: "{n} папок",
  ingestSkipped: " (пропущено: {n})",
  and: " и ",
};

export const en: Messages = {
  appTitle: "E-Reader",
  library: "Library",
  formatsLine: "TXT · Markdown · EPUB · FB2",
  addFolder: "Add folder",
  importFiles: "Import files",
  scan: "Scan",
  folders: "Folders",
  remove: "Remove",
  noBooksTitle: "No books yet",
  noBooksBody:
    "Add a folder with .txt, .md, .epub or .fb2 files, or import files directly.",
  noBooksHint:
    "Supported formats: TXT, Markdown, EPUB, FB2.\nTip: drag & drop files/folders here, or open a book with the system “Open with”.",
  ready: "Ready",
  unknownAuthor: "Unknown author",
  lastScan: "Last scan: +{added} · ~{updated} · −{removed} · total {total}",
  archive: "Archive",
  unarchive: "Restore",
  deleteBook: "Delete",
  archiveSection: "Archive ({n})",
  confirmDeleteBook:
    "Remove “{title}” from the library? Progress and bookmarks will be lost. Files in library folders are not deleted from disk.",
  statusBookArchived: "“{title}” moved to archive",
  statusBookRestored: "“{title}” restored to library",
  statusBookDeleted: "“{title}” removed from library",
  noActiveBooks: "No active books — they are archived or none have been added yet.",
  language: "Language",
  backToLibrary: "← Library",
  readingProgress: "Reading progress",
  fontSize: "Size",
  fontFamily: "Font",
  theme: "Theme",
  themeSepia: "Sepia",
  themeLight: "Light",
  themeDark: "Dark",
  hidePanel: "Hide panel",
  showPanel: "Panel",
  bookmark: "Bookmark",
  bookmarkShortcut: "Select text and press B",
  bookmarkAdded: "Bookmark: “{label}”",
  bookmarkNeedSelection: "Select a text passage in the book, then press B",
  pageAbbr: "p.",
  contents: "Contents",
  bookmarks: "Bookmarks",
  noBookmarks: "No bookmarks yet. Select text and press {key}.",
  readerHints: "{left}/{right} page · {esc} library · select + {b} bookmark",
  formatUnavailableTitle: "Format not readable yet",
  dropTitle: "Drop to open",
  dropHint: "Books (.epub, .fb2, .md, .txt) or a library folder",
  dialogChooseFolder: "Choose library folder",
  dialogImportFiles: "Import book files",
  dialogBooksFilter: "Books",
  statusImporting: "Importing…",
  statusScanning: "Scanning library…",
  statusScanComplete: "Scan complete: {total} books.",
  statusFolderRemoved: "Folder removed and library rescanned.",
  statusOpening: "Opening {title}…",
  statusDownloading: "Downloading book…",
  linkImported: "Book added to library: “{title}”",
  linkOpenedExternally: "Link opened in the system browser",
  linkImportFailed: "Could not download the book. {detail}",
  ingestNothingSkipped: "Nothing imported ({skipped} path(s) skipped).",
  ingestNothing: "Nothing to import",
  ingestImported: "Imported {parts}",
  ingestFiles: "{n} file(s)",
  ingestFolders: "{n} folder(s)",
  ingestSkipped: " ({n} skipped)",
  and: " and ",
};

export const catalogs: Record<Locale, Messages> = { ru, en };

export function isLocale(value: string | null | undefined): value is Locale {
  return value === "ru" || value === "en";
}

export function detectDefaultLocale(): Locale {
  if (typeof navigator !== "undefined") {
    const lang = (navigator.language || "").toLowerCase();
    if (lang.startsWith("ru")) return "ru";
  }
  return "ru";
}
