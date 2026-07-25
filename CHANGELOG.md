# Changelog

## 1.2.0 — 2026-07-25

### Added
- **Library archive**: move books out of the main card grid into a compact collapsible section; restore anytime
- **Delete from library**: remove a book from the catalog (progress and bookmarks cleared); confirmation dialog
- Path exclusions so deleted folder-library books are not re-added on scan; re-import clears the exclusion
- Managed downloads under app data are removed from disk on delete; user library folder files are left intact

### Changed
- Scan no longer drops books that were imported outside library root folders
- Version bump across npm, Cargo, and Tauri config to **1.2.0**

### Notes
- Archive is soft-hide only; the file stays on disk and can still be opened from the archive list

## 0.1.0 — 2026-07-23

### Added
- Desktop shell on Rust + Tauri 2 + SvelteKit
- Library: folder scan, file import, SQLite catalog
- Reading for **TXT**, **Markdown**, **EPUB**, **FB2**
- Table of contents, bookmarks, themes
- Reader typography: font family picker and numeric font size (12–40 px), persisted in settings
- Cover extraction for EPUB/FB2
- Reading positions: document fraction + nearest anchor (stable across font changes)
- Legacy `scroll:px` positions still restore
- Windows packaging targets: NSIS installer + MSI
- File associations for epub, fb2, md, txt
- **Open with** / CLI launch paths → import and auto-open first book
- **Drag-and-drop** files and folders onto the window (with drop overlay)
- **UI localization**: Russian (default) and English, language picker, preference saved in settings
- In-reader link handling: download book files into the library; open other web links in the system browser

### Notes
- Progress is continuous-scroll based (not fixed page numbers)
- Cloud sync is intentionally out of scope for 0.1
