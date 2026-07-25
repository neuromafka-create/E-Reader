use crate::error::{AppError, AppResult};
use crate::models::{Book, BookFormat, Bookmark, LibraryRoot, Progress, ReaderSettings};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use uuid::Uuid;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open(path: &Path) -> AppResult<Self> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let conn = Connection::open(path)?;
        conn.execute_batch(
            "
            PRAGMA foreign_keys = ON;
            PRAGMA journal_mode = WAL;
            ",
        )?;
        let db = Self { conn };
        db.migrate()?;
        Ok(db)
    }

    fn migrate(&self) -> AppResult<()> {
        self.conn.execute_batch(
            "
            CREATE TABLE IF NOT EXISTS library_roots (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                added_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS books (
                id TEXT PRIMARY KEY,
                path TEXT NOT NULL UNIQUE,
                title TEXT NOT NULL,
                author TEXT,
                format TEXT NOT NULL,
                cover_path TEXT,
                file_size INTEGER NOT NULL DEFAULT 0,
                modified_at TEXT,
                added_at TEXT NOT NULL,
                last_opened_at TEXT
            );

            CREATE TABLE IF NOT EXISTS progress (
                book_id TEXT PRIMARY KEY REFERENCES books(id) ON DELETE CASCADE,
                position TEXT NOT NULL,
                percentage REAL NOT NULL DEFAULT 0,
                updated_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS bookmarks (
                id TEXT PRIMARY KEY,
                book_id TEXT NOT NULL REFERENCES books(id) ON DELETE CASCADE,
                position TEXT NOT NULL,
                label TEXT,
                created_at TEXT NOT NULL
            );

            CREATE TABLE IF NOT EXISTS settings (
                key TEXT PRIMARY KEY,
                value TEXT NOT NULL
            );

            CREATE INDEX IF NOT EXISTS idx_books_format ON books(format);
            CREATE INDEX IF NOT EXISTS idx_bookmarks_book ON bookmarks(book_id);

            CREATE TABLE IF NOT EXISTS book_exclusions (
                path TEXT PRIMARY KEY,
                excluded_at TEXT NOT NULL
            );
            ",
        )?;
        self.ensure_column("books", "archived", "INTEGER NOT NULL DEFAULT 0")?;
        Ok(())
    }

    fn ensure_column(&self, table: &str, column: &str, decl: &str) -> AppResult<()> {
        let pragma = format!("PRAGMA table_info({table})");
        let mut stmt = self.conn.prepare(&pragma)?;
        let names = stmt
            .query_map([], |row| row.get::<_, String>(1))?
            .collect::<Result<Vec<_>, _>>()?;
        if !names.iter().any(|n| n == column) {
            self.conn
                .execute(&format!("ALTER TABLE {table} ADD COLUMN {column} {decl}"), [])?;
        }
        Ok(())
    }

    fn map_book_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Book> {
        let format_str: String = row.get(4)?;
        let archived_i: i64 = row.get(11)?;
        Ok(Book {
            id: row.get(0)?,
            path: row.get(1)?,
            title: row.get(2)?,
            author: row.get(3)?,
            format: BookFormat::parse(&format_str).unwrap_or(BookFormat::Txt),
            cover_path: row.get(5)?,
            file_size: row.get(6)?,
            modified_at: row.get(7)?,
            added_at: row.get(8)?,
            last_opened_at: row.get(9)?,
            progress_percentage: row.get(10)?,
            archived: archived_i != 0,
        })
    }

    pub fn list_library_roots(&self) -> AppResult<Vec<LibraryRoot>> {
        let mut stmt = self
            .conn
            .prepare("SELECT id, path, added_at FROM library_roots ORDER BY added_at")?;
        let rows = stmt.query_map([], |row| {
            Ok(LibraryRoot {
                id: row.get(0)?,
                path: row.get(1)?,
                added_at: row.get(2)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn add_library_root(&self, path: &str) -> AppResult<LibraryRoot> {
        let now = Utc::now().to_rfc3339();
        let root = LibraryRoot {
            id: Uuid::new_v4().to_string(),
            path: path.to_string(),
            added_at: now.clone(),
        };
        self.conn.execute(
            "INSERT INTO library_roots (id, path, added_at) VALUES (?1, ?2, ?3)
             ON CONFLICT(path) DO NOTHING",
            params![root.id, root.path, root.added_at],
        )?;
        // Return existing if conflict
        if let Some(existing) = self
            .conn
            .query_row(
                "SELECT id, path, added_at FROM library_roots WHERE path = ?1",
                params![path],
                |row| {
                    Ok(LibraryRoot {
                        id: row.get(0)?,
                        path: row.get(1)?,
                        added_at: row.get(2)?,
                    })
                },
            )
            .optional()?
        {
            return Ok(existing);
        }
        Ok(root)
    }

    pub fn remove_library_root(&self, id: &str) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM library_roots WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn list_books(&self) -> AppResult<Vec<Book>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT b.id, b.path, b.title, b.author, b.format, b.cover_path, b.file_size,
                   b.modified_at, b.added_at, b.last_opened_at,
                   COALESCE(p.percentage, 0.0),
                   COALESCE(b.archived, 0)
            FROM books b
            LEFT JOIN progress p ON p.book_id = b.id
            ORDER BY COALESCE(b.last_opened_at, b.added_at) DESC
            ",
        )?;
        let rows = stmt.query_map([], Self::map_book_row)?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn get_book(&self, id: &str) -> AppResult<Book> {
        self.conn
            .query_row(
                "
                SELECT b.id, b.path, b.title, b.author, b.format, b.cover_path, b.file_size,
                       b.modified_at, b.added_at, b.last_opened_at,
                       COALESCE(p.percentage, 0.0),
                       COALESCE(b.archived, 0)
                FROM books b
                LEFT JOIN progress p ON p.book_id = b.id
                WHERE b.id = ?1
                ",
                params![id],
                Self::map_book_row,
            )
            .map_err(|e| match e {
                rusqlite::Error::QueryReturnedNoRows => AppError::msg("Book not found"),
                other => other.into(),
            })
    }

    pub fn is_path_excluded(&self, path: &str) -> AppResult<bool> {
        let found: Option<i64> = self
            .conn
            .query_row(
                "SELECT 1 FROM book_exclusions WHERE path = ?1",
                params![path],
                |row| row.get(0),
            )
            .optional()?;
        Ok(found.is_some())
    }

    pub fn clear_path_exclusion(&self, path: &str) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM book_exclusions WHERE path = ?1", params![path])?;
        Ok(())
    }

    pub fn set_book_archived(&self, id: &str, archived: bool) -> AppResult<Book> {
        let flag = if archived { 1 } else { 0 };
        let n = self.conn.execute(
            "UPDATE books SET archived = ?1 WHERE id = ?2",
            params![flag, id],
        )?;
        if n == 0 {
            return Err(AppError::msg("Book not found"));
        }
        self.get_book(id)
    }

    /// Remove book from catalog. Returns the removed book (for file cleanup by caller).
    pub fn delete_book(&self, id: &str) -> AppResult<Book> {
        let book = self.get_book(id)?;
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "INSERT INTO book_exclusions (path, excluded_at) VALUES (?1, ?2)
             ON CONFLICT(path) DO UPDATE SET excluded_at = excluded.excluded_at",
            params![book.path, now],
        )?;
        self.conn
            .execute("DELETE FROM books WHERE id = ?1", params![id])?;
        Ok(book)
    }

    pub fn upsert_book(
        &self,
        path: &str,
        title: &str,
        author: Option<&str>,
        format: BookFormat,
        file_size: i64,
        modified_at: Option<&str>,
    ) -> AppResult<(bool, String)> {
        let existing: Option<(String,)> = self
            .conn
            .query_row(
                "SELECT id FROM books WHERE path = ?1",
                params![path],
                |row| Ok((row.get(0)?,)),
            )
            .optional()?;

        if let Some((id,)) = existing {
            // Preserve archived flag on metadata refresh.
            self.conn.execute(
                "
                UPDATE books
                SET title = ?1, author = ?2, format = ?3, file_size = ?4, modified_at = ?5
                WHERE id = ?6
                ",
                params![title, author, format.as_str(), file_size, modified_at, id],
            )?;
            Ok((false, id))
        } else {
            let id = Uuid::new_v4().to_string();
            let now = Utc::now().to_rfc3339();
            self.conn.execute(
                "
                INSERT INTO books (id, path, title, author, format, cover_path, file_size, modified_at, added_at, archived)
                VALUES (?1, ?2, ?3, ?4, ?5, NULL, ?6, ?7, ?8, 0)
                ",
                params![
                    id,
                    path,
                    title,
                    author,
                    format.as_str(),
                    file_size,
                    modified_at,
                    now
                ],
            )?;
            Ok((true, id))
        }
    }

    pub fn set_cover_path(&self, book_id: &str, cover_path: Option<&str>) -> AppResult<()> {
        self.conn.execute(
            "UPDATE books SET cover_path = ?1 WHERE id = ?2",
            params![cover_path, book_id],
        )?;
        Ok(())
    }

    /// Drop catalog entries for files that used to live under library roots but
    /// are no longer found. Imported / downloaded books outside roots are kept.
    pub fn remove_missing_root_books(
        &self,
        existing_paths: &[String],
        root_paths: &[String],
    ) -> AppResult<usize> {
        if root_paths.is_empty() {
            return Ok(0);
        }

        let mut stmt = self.conn.prepare("SELECT id, path FROM books")?;
        let catalog: Vec<(String, String)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<Result<Vec<_>, _>>()?;

        let existing: std::collections::HashSet<&str> =
            existing_paths.iter().map(String::as_str).collect();
        let mut removed = 0usize;

        for (id, path) in catalog {
            if !path_is_under_any_root(&path, root_paths) {
                continue;
            }
            if existing.contains(path.as_str()) {
                continue;
            }
            self.conn
                .execute("DELETE FROM books WHERE id = ?1", params![id])?;
            removed += 1;
        }

        Ok(removed)
    }

    pub fn mark_opened(&self, book_id: &str) -> AppResult<()> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "UPDATE books SET last_opened_at = ?1 WHERE id = ?2",
            params![now, book_id],
        )?;
        Ok(())
    }

    pub fn get_progress(&self, book_id: &str) -> AppResult<Option<Progress>> {
        self.conn
            .query_row(
                "SELECT book_id, position, percentage, updated_at FROM progress WHERE book_id = ?1",
                params![book_id],
                |row| {
                    Ok(Progress {
                        book_id: row.get(0)?,
                        position: row.get(1)?,
                        percentage: row.get(2)?,
                        updated_at: row.get(3)?,
                    })
                },
            )
            .optional()
            .map_err(Into::into)
    }

    pub fn save_progress(&self, book_id: &str, position: &str, percentage: f64) -> AppResult<Progress> {
        let now = Utc::now().to_rfc3339();
        self.conn.execute(
            "
            INSERT INTO progress (book_id, position, percentage, updated_at)
            VALUES (?1, ?2, ?3, ?4)
            ON CONFLICT(book_id) DO UPDATE SET
                position = excluded.position,
                percentage = excluded.percentage,
                updated_at = excluded.updated_at
            ",
            params![book_id, position, percentage, now],
        )?;
        Ok(Progress {
            book_id: book_id.to_string(),
            position: position.to_string(),
            percentage,
            updated_at: now,
        })
    }

    pub fn list_bookmarks(&self, book_id: &str) -> AppResult<Vec<Bookmark>> {
        let mut stmt = self.conn.prepare(
            "
            SELECT id, book_id, position, label, created_at
            FROM bookmarks
            WHERE book_id = ?1
            ORDER BY created_at DESC
            ",
        )?;
        let rows = stmt.query_map(params![book_id], |row| {
            Ok(Bookmark {
                id: row.get(0)?,
                book_id: row.get(1)?,
                position: row.get(2)?,
                label: row.get(3)?,
                created_at: row.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    pub fn add_bookmark(
        &self,
        book_id: &str,
        position: &str,
        label: Option<&str>,
    ) -> AppResult<Bookmark> {
        let bookmark = Bookmark {
            id: Uuid::new_v4().to_string(),
            book_id: book_id.to_string(),
            position: position.to_string(),
            label: label.map(str::to_string),
            created_at: Utc::now().to_rfc3339(),
        };
        self.conn.execute(
            "
            INSERT INTO bookmarks (id, book_id, position, label, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5)
            ",
            params![
                bookmark.id,
                bookmark.book_id,
                bookmark.position,
                bookmark.label,
                bookmark.created_at
            ],
        )?;
        Ok(bookmark)
    }

    pub fn remove_bookmark(&self, id: &str) -> AppResult<()> {
        self.conn
            .execute("DELETE FROM bookmarks WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn get_reader_settings(&self) -> AppResult<ReaderSettings> {
        let value: Option<String> = self
            .conn
            .query_row(
                "SELECT value FROM settings WHERE key = 'reader'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        match value {
            // Missing `locale` in older saves is filled by #[serde(default)]
            Some(json) => Ok(serde_json::from_str(&json).unwrap_or_default()),
            None => Ok(ReaderSettings::default()),
        }
    }

    pub fn save_reader_settings(&self, settings: &ReaderSettings) -> AppResult<()> {
        let json = serde_json::to_string(settings).map_err(|e| AppError::msg(e.to_string()))?;
        self.conn.execute(
            "
            INSERT INTO settings (key, value) VALUES ('reader', ?1)
            ON CONFLICT(key) DO UPDATE SET value = excluded.value
            ",
            params![json],
        )?;
        Ok(())
    }
}

fn path_is_under_any_root(path: &str, roots: &[String]) -> bool {
    let path_norm = normalize_path_key(path);
    roots.iter().any(|root| {
        let root_norm = normalize_path_key(root);
        path_norm == root_norm
            || path_norm.starts_with(&(root_norm.clone() + "/"))
            || path_norm.starts_with(&(root_norm + "\\"))
    })
}

fn normalize_path_key(path: &str) -> String {
    let mut s = path.replace('/', "\\");
    while s.ends_with('\\') && s.len() > 1 {
        s.pop();
    }
    // Case-insensitive compare on Windows.
    #[cfg(windows)]
    {
        s = s.to_ascii_lowercase();
    }
    s
}
