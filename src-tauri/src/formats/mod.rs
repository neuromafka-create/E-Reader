mod epub;
mod fb2;
mod html_util;
mod markdown;
mod txt;

use crate::error::AppResult;
use crate::models::{BookFormat, TocEntry};
use std::path::Path;

#[derive(Clone)]
pub struct CoverImage {
    pub bytes: Vec<u8>,
    pub mime: String,
}

#[derive(Clone)]
pub struct BookMeta {
    pub title: String,
    pub author: Option<String>,
    pub cover: Option<CoverImage>,
}

pub struct ParsedBook {
    pub html: String,
    pub toc: Vec<TocEntry>,
    pub title: Option<String>,
    pub author: Option<String>,
    pub cover: Option<CoverImage>,
}

pub fn detect_format(path: &Path) -> Option<BookFormat> {
    path.extension()
        .and_then(|e| e.to_str())
        .and_then(BookFormat::from_extension)
}

pub fn title_from_path(path: &Path) -> String {
    path.file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("Untitled")
        .to_string()
}

pub fn extract_metadata(path: &Path, format: &BookFormat) -> AppResult<BookMeta> {
    match format {
        BookFormat::Txt | BookFormat::Markdown => Ok(BookMeta {
            title: title_from_path(path),
            author: None,
            cover: None,
        }),
        BookFormat::Epub => epub::metadata(path),
        BookFormat::Fb2 => fb2::metadata(path),
    }
}

pub fn parse_book(path: &Path, format: &BookFormat) -> AppResult<ParsedBook> {
    match format {
        BookFormat::Txt => {
            let mut parsed = txt::parse(path)?;
            parsed.title = Some(title_from_path(path));
            Ok(parsed)
        }
        BookFormat::Markdown => markdown::parse(path),
        BookFormat::Epub => epub::parse(path),
        BookFormat::Fb2 => fb2::parse(path),
    }
}

pub fn save_cover(
    covers_dir: &Path,
    book_id: &str,
    cover: &CoverImage,
) -> AppResult<String> {
    std::fs::create_dir_all(covers_dir)?;
    let ext = html_util::extension_for_mime(&cover.mime);
    let filename = format!("{book_id}.{ext}");
    let path = covers_dir.join(&filename);
    std::fs::write(&path, &cover.bytes)?;
    Ok(path.to_string_lossy().to_string())
}

