use crate::db::Database;
use crate::error::{AppError, AppResult};
use crate::formats::{self, title_from_path};
use crate::models::{
    Book, BookContent, Bookmark, ImportFromUrlResult, IngestResult, LibraryRoot, Progress,
    ReaderSettings, ScanResult,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{AppHandle, Manager, State};
use walkdir::WalkDir;

pub struct AppState {
    pub db: Mutex<Database>,
    pub data_dir: PathBuf,
}

fn with_db<T>(state: &State<'_, AppState>, f: impl FnOnce(&Database) -> AppResult<T>) -> AppResult<T> {
    let db = state
        .db
        .lock()
        .map_err(|_| AppError::msg("Database lock poisoned"))?;
    f(&db)
}

fn covers_dir(state: &State<'_, AppState>) -> PathBuf {
    state.data_dir.join("covers")
}

fn index_book(
    state: &State<'_, AppState>,
    path: &Path,
    format: crate::models::BookFormat,
    file_size: i64,
    modified_at: Option<String>,
) -> AppResult<(bool, String)> {
    let meta = formats::extract_metadata(path, &format).unwrap_or_else(|_| formats::BookMeta {
        title: title_from_path(path),
        author: None,
        cover: None,
    });
    // BookMeta is private fields in formats - need to use the public struct
    // Actually BookMeta is pub in formats/mod.rs

    let path_str = path.to_string_lossy().to_string();
    let (is_new, id) = with_db(state, |db| {
        db.upsert_book(
            &path_str,
            &meta.title,
            meta.author.as_deref(),
            format,
            file_size,
            modified_at.as_deref(),
        )
    })?;

    if let Some(cover) = meta.cover {
        match formats::save_cover(&covers_dir(state), &id, &cover) {
            Ok(cover_path) => {
                let _ = with_db(state, |db| db.set_cover_path(&id, Some(&cover_path)));
            }
            Err(_) => {
                // Cover extraction is best-effort
            }
        }
    }

    Ok((is_new, id))
}

#[tauri::command]
pub fn list_library_roots(state: State<'_, AppState>) -> AppResult<Vec<LibraryRoot>> {
    with_db(&state, |db| db.list_library_roots())
}

#[tauri::command]
pub fn add_library_root(state: State<'_, AppState>, path: String) -> AppResult<LibraryRoot> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.is_dir() {
        return Err(AppError::msg("Path is not a directory"));
    }
    with_db(&state, |db| db.add_library_root(&path))
}

#[tauri::command]
pub fn remove_library_root(state: State<'_, AppState>, id: String) -> AppResult<()> {
    with_db(&state, |db| db.remove_library_root(&id))
}

#[tauri::command]
pub fn list_books(state: State<'_, AppState>) -> AppResult<Vec<Book>> {
    with_db(&state, |db| db.list_books())
}

#[tauri::command]
pub fn scan_library(state: State<'_, AppState>) -> AppResult<ScanResult> {
    do_scan_library(&state)
}

fn do_scan_library(state: &State<'_, AppState>) -> AppResult<ScanResult> {
    let roots = with_db(state, |db| db.list_library_roots())?;
    let mut added = 0usize;
    let mut updated = 0usize;
    let mut existing_paths = Vec::new();

    for root in roots {
        let root_path = PathBuf::from(&root.path);
        if !root_path.exists() {
            continue;
        }

        for entry in WalkDir::new(&root_path)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let Some(format) = formats::detect_format(path) else {
                continue;
            };

            let meta = entry.metadata().ok();
            let file_size = meta.as_ref().map(|m| m.len() as i64).unwrap_or(0);
            let modified_at = meta
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let dt: chrono::DateTime<chrono::Utc> = t.into();
                    dt.to_rfc3339()
                });

            existing_paths.push(path.to_string_lossy().to_string());
            let (is_new, _) = index_book(state, path, format, file_size, modified_at)?;
            if is_new {
                added += 1;
            } else {
                updated += 1;
            }
        }
    }

    let removed = with_db(state, |db| db.remove_missing_books(&existing_paths))?;
    let total = with_db(state, |db| db.list_books())?.len();

    Ok(ScanResult {
        added,
        updated,
        removed,
        total,
    })
}

#[tauri::command]
pub fn import_files(state: State<'_, AppState>, paths: Vec<String>) -> AppResult<usize> {
    let result = do_ingest_paths(&state, paths)?;
    Ok(result.files_imported)
}

/// Import files and/or add folders from OS open-with, CLI args, or drag-and-drop.
#[tauri::command]
pub fn ingest_paths(state: State<'_, AppState>, paths: Vec<String>) -> AppResult<IngestResult> {
    do_ingest_paths(&state, paths)
}

fn do_ingest_paths(state: &State<'_, AppState>, paths: Vec<String>) -> AppResult<IngestResult> {
    let mut files_imported = 0usize;
    let mut folders_added = 0usize;
    let mut skipped = 0usize;
    let mut book_ids = Vec::new();
    let mut need_scan = false;

    for path_str in paths {
        let trimmed = path_str.trim();
        let path = PathBuf::from(trimmed);
        if trimmed.is_empty() || !path.exists() {
            skipped += 1;
            continue;
        }

        if path.is_dir() {
            let path_s = path.to_string_lossy().to_string();
            with_db(state, |db| db.add_library_root(&path_s))?;
            folders_added += 1;
            need_scan = true;
            continue;
        }

        if !path.is_file() {
            skipped += 1;
            continue;
        }

        let Some(format) = formats::detect_format(&path) else {
            skipped += 1;
            continue;
        };

        let meta = std::fs::metadata(&path).ok();
        let file_size = meta.as_ref().map(|m| m.len() as i64).unwrap_or(0);
        let modified_at = meta.and_then(|m| m.modified().ok()).map(|t| {
            let dt: chrono::DateTime<chrono::Utc> = t.into();
            dt.to_rfc3339()
        });
        let (_is_new, id) = index_book(state, &path, format, file_size, modified_at)?;
        book_ids.push(id);
        files_imported += 1;
    }

    if need_scan {
        let _ = do_scan_library(state)?;
    }

    let open_book_id = book_ids.first().cloned();
    let message = match (files_imported, folders_added, skipped) {
        (0, 0, s) if s > 0 => format!("Nothing imported ({s} path(s) skipped)."),
        (f, d, s) => {
            let mut parts = Vec::new();
            if f > 0 {
                parts.push(format!("{f} file(s)"));
            }
            if d > 0 {
                parts.push(format!("{d} folder(s)"));
            }
            let base = if parts.is_empty() {
                "Nothing to import".into()
            } else {
                format!("Imported {}", parts.join(" and "))
            };
            if s > 0 {
                format!("{base} ({s} skipped).")
            } else {
                format!("{base}.")
            }
        }
    };

    Ok(IngestResult {
        files_imported,
        folders_added,
        book_ids,
        open_book_id,
        skipped,
        message,
    })
}

/// Paths passed to the process at launch (open-with / CLI).
/// Filters out flags and non-book junk; keeps files and directories.
#[tauri::command]
pub fn get_launch_paths() -> Vec<String> {
    collect_launch_paths()
}

pub fn collect_launch_paths() -> Vec<String> {
    std::env::args()
        .skip(1)
        .filter_map(|arg| {
            let trimmed = arg.trim();
            if trimmed.is_empty() || trimmed.starts_with('-') {
                return None;
            }
            // Skip npm/tauri tooling noise when present
            let lower = trimmed.to_ascii_lowercase();
            if lower.contains("node_modules")
                || lower.ends_with("tauri.js")
                || lower.ends_with("cli.js")
            {
                return None;
            }

            let path = PathBuf::from(trimmed);
            if !path.exists() {
                return None;
            }
            if path.is_dir() {
                return Some(path.to_string_lossy().to_string());
            }
            if path.is_file() && formats::detect_format(&path).is_some() {
                return Some(path.to_string_lossy().to_string());
            }
            None
        })
        .collect()
}

/// Download a remote URL (direct file or product page with download links)
/// into the library folder and index it.
#[tauri::command]
pub async fn import_from_url(
    state: State<'_, AppState>,
    url: String,
) -> AppResult<ImportFromUrlResult> {
    let downloads = state.data_dir.join("downloads");
    // Do not hold DB locks across await.
    let result = crate::download::import_from_url(&url, &downloads).await?;

    if !result.success {
        return Ok(ImportFromUrlResult {
            success: false,
            open_book_id: None,
            title: None,
            path: result.path,
            message: result.message,
            open_externally: result.open_externally,
        });
    }

    let Some(path_str) = result.path.clone() else {
        return Ok(ImportFromUrlResult {
            success: false,
            open_book_id: None,
            title: None,
            path: None,
            message: "Download succeeded but path is missing.".into(),
            open_externally: false,
        });
    };

    let path = PathBuf::from(&path_str);
    let Some(format) = formats::detect_format(&path) else {
        return Ok(ImportFromUrlResult {
            success: false,
            open_book_id: None,
            title: None,
            path: Some(path_str),
            message: "Downloaded file is not a supported book format.".into(),
            open_externally: false,
        });
    };

    let meta = std::fs::metadata(&path).ok();
    let file_size = meta.as_ref().map(|m| m.len() as i64).unwrap_or(0);
    let modified_at = meta.and_then(|m| m.modified().ok()).map(|t| {
        let dt: chrono::DateTime<chrono::Utc> = t.into();
        dt.to_rfc3339()
    });

    let (_is_new, id) = index_book(&state, &path, format, file_size, modified_at)?;
    let book = with_db(&state, |db| db.get_book(&id))?;
    let title = book.title;

    Ok(ImportFromUrlResult {
        success: true,
        open_book_id: Some(id),
        title: Some(title.clone()),
        path: Some(path_str),
        message: format!("Book added to library: {title}"),
        open_externally: false,
    })
}

#[tauri::command]
pub fn open_book(state: State<'_, AppState>, id: String) -> AppResult<BookContent> {
    let book = with_db(&state, |db| {
        db.mark_opened(&id)?;
        db.get_book(&id)
    })?;

    let progress = with_db(&state, |db| db.get_progress(&id))?;
    let path = Path::new(&book.path);

    if !path.exists() {
        return Err(AppError::msg("Book file is missing on disk"));
    }

    if !book.format.is_readable() {
        return Ok(BookContent {
            book,
            html: String::new(),
            toc: Vec::new(),
            progress,
            readable: false,
            message: Some("This format is not supported for reading yet.".into()),
        });
    }

    let parsed = formats::parse_book(path, &book.format)?;
    let mut book = book;

    if let Some(title) = parsed.title.clone() {
        if !title.is_empty() {
            book.title = title;
        }
    }
    if let Some(author) = parsed.author.clone() {
        book.author = Some(author);
    }

    let _ = with_db(&state, |db| {
        db.upsert_book(
            &book.path,
            &book.title,
            book.author.as_deref(),
            book.format.clone(),
            book.file_size,
            book.modified_at.as_deref(),
        )
    });

    if let Some(cover) = &parsed.cover {
        if let Ok(cover_path) = formats::save_cover(&covers_dir(&state), &book.id, cover) {
            let _ = with_db(&state, |db| db.set_cover_path(&book.id, Some(&cover_path)));
            book.cover_path = Some(cover_path);
        }
    }

    Ok(BookContent {
        book,
        html: parsed.html,
        toc: parsed.toc,
        progress,
        readable: true,
        message: None,
    })
}

#[tauri::command]
pub fn get_cover_data_url(state: State<'_, AppState>, book_id: String) -> AppResult<Option<String>> {
    let book = with_db(&state, |db| db.get_book(&book_id))?;
    let Some(cover_path) = book.cover_path else {
        return Ok(None);
    };
    let path = PathBuf::from(&cover_path);
    if !path.exists() {
        return Ok(None);
    }
    let bytes = std::fs::read(&path)?;
    let mime = match path.extension().and_then(|e| e.to_str()).unwrap_or("") {
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "svg" => "image/svg+xml",
        "avif" => "image/avif",
        _ => "application/octet-stream",
    };
    Ok(Some(format!(
        "data:{mime};base64,{}",
        B64.encode(bytes)
    )))
}

#[tauri::command]
pub fn save_progress(
    state: State<'_, AppState>,
    book_id: String,
    position: String,
    percentage: f64,
) -> AppResult<Progress> {
    with_db(&state, |db| db.save_progress(&book_id, &position, percentage))
}

#[tauri::command]
pub fn get_progress(state: State<'_, AppState>, book_id: String) -> AppResult<Option<Progress>> {
    with_db(&state, |db| db.get_progress(&book_id))
}

#[tauri::command]
pub fn list_bookmarks(state: State<'_, AppState>, book_id: String) -> AppResult<Vec<Bookmark>> {
    with_db(&state, |db| db.list_bookmarks(&book_id))
}

#[tauri::command]
pub fn add_bookmark(
    state: State<'_, AppState>,
    book_id: String,
    position: String,
    label: Option<String>,
) -> AppResult<Bookmark> {
    with_db(&state, |db| {
        db.add_bookmark(&book_id, &position, label.as_deref())
    })
}

#[tauri::command]
pub fn remove_bookmark(state: State<'_, AppState>, id: String) -> AppResult<()> {
    with_db(&state, |db| db.remove_bookmark(&id))
}

#[tauri::command]
pub fn get_reader_settings(state: State<'_, AppState>) -> AppResult<ReaderSettings> {
    with_db(&state, |db| db.get_reader_settings())
}

#[tauri::command]
pub fn save_reader_settings(
    state: State<'_, AppState>,
    settings: ReaderSettings,
) -> AppResult<()> {
    with_db(&state, |db| db.save_reader_settings(&settings))
}

#[tauri::command]
pub fn get_supported_formats() -> Vec<&'static str> {
    vec!["txt", "markdown", "epub", "fb2"]
}

pub fn init_state(app: &AppHandle) -> AppResult<AppState> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::msg(format!("Cannot resolve app data dir: {e}")))?;
    std::fs::create_dir_all(&dir)?;
    std::fs::create_dir_all(dir.join("covers"))?;
    let db_path = dir.join("library.db");
    let db = Database::open(&db_path)?;
    Ok(AppState {
        db: Mutex::new(db),
        data_dir: dir,
    })
}
