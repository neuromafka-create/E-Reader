export type BookFormat = "txt" | "markdown" | "epub" | "fb2";

export interface Book {
  id: string;
  path: string;
  title: string;
  author: string | null;
  format: BookFormat;
  coverPath: string | null;
  fileSize: number;
  modifiedAt: string | null;
  addedAt: string;
  lastOpenedAt: string | null;
  progressPercentage: number;
}

export interface LibraryRoot {
  id: string;
  path: string;
  addedAt: string;
}

export interface Progress {
  bookId: string;
  position: string;
  percentage: number;
  updatedAt: string;
}

export interface Bookmark {
  id: string;
  bookId: string;
  position: string;
  label: string | null;
  createdAt: string;
}

export interface TocEntry {
  id: string;
  title: string;
  level: number;
}

export interface BookContent {
  book: Book;
  html: string;
  toc: TocEntry[];
  progress: Progress | null;
  readable: boolean;
  message: string | null;
}

export interface ScanResult {
  added: number;
  updated: number;
  removed: number;
  total: number;
}

export interface IngestResult {
  filesImported: number;
  foldersAdded: number;
  bookIds: string[];
  openBookId: string | null;
  skipped: number;
  message: string;
}

export interface ReaderSettings {
  fontFamily: string;
  fontSize: number;
  lineHeight: number;
  theme: string;
  maxWidth: number;
  /** UI language: "ru" | "en" */
  locale: string;
}
