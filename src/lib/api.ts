import { invoke } from "@tauri-apps/api/core";
import type {
  Book,
  BookContent,
  Bookmark,
  ImportFromUrlResult,
  IngestResult,
  LibraryRoot,
  Progress,
  ReaderSettings,
  ScanResult,
} from "./types";

export const api = {
  listLibraryRoots: () => invoke<LibraryRoot[]>("list_library_roots"),
  addLibraryRoot: (path: string) =>
    invoke<LibraryRoot>("add_library_root", { path }),
  removeLibraryRoot: (id: string) =>
    invoke<void>("remove_library_root", { id }),
  listBooks: () => invoke<Book[]>("list_books"),
  setBookArchived: (id: string, archived: boolean) =>
    invoke<Book>("set_book_archived", { id, archived }),
  deleteBook: (id: string) => invoke<void>("delete_book", { id }),
  scanLibrary: () => invoke<ScanResult>("scan_library"),
  importFiles: (paths: string[]) =>
    invoke<number>("import_files", { paths }),
  ingestPaths: (paths: string[]) =>
    invoke<IngestResult>("ingest_paths", { paths }),
  getLaunchPaths: () => invoke<string[]>("get_launch_paths"),
  openBook: (id: string) => invoke<BookContent>("open_book", { id }),
  importFromUrl: (url: string) =>
    invoke<ImportFromUrlResult>("import_from_url", { url }),
  getCoverDataUrl: (bookId: string) =>
    invoke<string | null>("get_cover_data_url", { bookId }),
  saveProgress: (bookId: string, position: string, percentage: number) =>
    invoke<Progress>("save_progress", { bookId, position, percentage }),
  listBookmarks: (bookId: string) =>
    invoke<Bookmark[]>("list_bookmarks", { bookId }),
  addBookmark: (bookId: string, position: string, label?: string | null) =>
    invoke<Bookmark>("add_bookmark", { bookId, position, label: label ?? null }),
  removeBookmark: (id: string) => invoke<void>("remove_bookmark", { id }),
  getReaderSettings: () => invoke<ReaderSettings>("get_reader_settings"),
  saveReaderSettings: (settings: ReaderSettings) =>
    invoke<void>("save_reader_settings", { settings }),
  getSupportedFormats: () => invoke<string[]>("get_supported_formats"),
};
