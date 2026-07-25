# Changelog

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
