//! Download remote book files (or product pages that link to them) into the library.

use crate::error::{AppError, AppResult};
use crate::formats;
use crate::models::BookFormat;
use regex::Regex;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const USER_AGENT: &str =
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) E-Reader/1.2 (compatible; +local-ebook-reader)";

/// Result of attempting to import a remote URL into the library.
#[derive(Debug, Clone, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFromUrlResult {
    pub success: bool,
    /// Book was saved and indexed; open this id in the reader if desired.
    pub open_book_id: Option<String>,
    pub title: Option<String>,
    pub path: Option<String>,
    pub message: String,
    /// Caller should open the URL in the system browser instead.
    pub open_externally: bool,
}

pub async fn import_from_url(url: &str, downloads_dir: &Path) -> AppResult<ImportFromUrlResult> {
    let url = url.trim();
    if !(url.starts_with("http://") || url.starts_with("https://")) {
        return Ok(ImportFromUrlResult {
            success: false,
            open_book_id: None,
            title: None,
            path: None,
            message: "Only http(s) links can be downloaded.".into(),
            open_externally: true,
        });
    }

    // Payment / non-book links → open outside the reader.
    if is_non_book_url(url) {
        return Ok(ImportFromUrlResult {
            success: false,
            open_book_id: None,
            title: None,
            path: None,
            message: "Link opened in the system browser.".into(),
            open_externally: true,
        });
    }

    let client = reqwest::Client::builder()
        .user_agent(USER_AGENT)
        .redirect(reqwest::redirect::Policy::limited(10))
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| AppError::msg(format!("HTTP client error: {e}")))?;

    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::msg(format!("Download failed: {e}")))?;

    if !response.status().is_success() {
        return Ok(ImportFromUrlResult {
            success: false,
            open_book_id: None,
            title: None,
            path: None,
            message: format!("Server returned HTTP {}.", response.status().as_u16()),
            open_externally: true,
        });
    }

    let final_url = response.url().to_string();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let content_disposition = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();

    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::msg(format!("Failed to read response body: {e}")))?;

    // Direct book file?
    if let Some(ext) = detect_book_extension(&final_url, &content_type, &content_disposition, &bytes)
    {
        return save_bytes_as_book(&bytes, &final_url, ext, downloads_dir);
    }

    // HTML product page → look for downloadable book links.
    if content_type.contains("text/html") || looks_like_html(&bytes) {
        let html = String::from_utf8_lossy(&bytes);
        let candidates = extract_book_links(&html, &final_url);
        for candidate in candidates {
            match download_book_file(&client, &candidate, downloads_dir).await {
                Ok(result) if result.success => return Ok(result),
                _ => continue,
            }
        }
        // No direct file found — let the user open the store page in a real browser.
        return Ok(ImportFromUrlResult {
            success: false,
            open_book_id: None,
            title: None,
            path: None,
            message: "No downloadable book file found on the page. Opening in browser.".into(),
            open_externally: true,
        });
    }

    Ok(ImportFromUrlResult {
        success: false,
        open_book_id: None,
        title: None,
        path: None,
        message: "This link is not a supported book file.".into(),
        open_externally: true,
    })
}

async fn download_book_file(
    client: &reqwest::Client,
    url: &str,
    downloads_dir: &Path,
) -> AppResult<ImportFromUrlResult> {
    let response = client
        .get(url)
        .send()
        .await
        .map_err(|e| AppError::msg(format!("Download failed: {e}")))?;
    if !response.status().is_success() {
        return Err(AppError::msg(format!(
            "HTTP {}",
            response.status().as_u16()
        )));
    }
    let final_url = response.url().to_string();
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_ascii_lowercase();
    let content_disposition = response
        .headers()
        .get(reqwest::header::CONTENT_DISPOSITION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let bytes = response
        .bytes()
        .await
        .map_err(|e| AppError::msg(format!("Failed to read body: {e}")))?;

    let Some(ext) =
        detect_book_extension(&final_url, &content_type, &content_disposition, &bytes)
    else {
        return Err(AppError::msg("Not a book file"));
    };
    save_bytes_as_book(&bytes, &final_url, ext, downloads_dir)
}

fn save_bytes_as_book(
    bytes: &[u8],
    source_url: &str,
    ext: &str,
    downloads_dir: &Path,
) -> AppResult<ImportFromUrlResult> {
    std::fs::create_dir_all(downloads_dir)?;
    let name = filename_from_url(source_url, ext);
    let path = unique_path(downloads_dir, &name);
    std::fs::write(&path, bytes)?;

    // Index is done by the command layer (needs AppState). Here we only save the file
    // and return path — wait, better do indexing in command. Return path for command.
    Ok(ImportFromUrlResult {
        success: true,
        open_book_id: None, // filled by command after index
        title: None,
        path: Some(path.to_string_lossy().to_string()),
        message: format!("Downloaded to {}", path.display()),
        open_externally: false,
    })
}

fn is_non_book_url(url: &str) -> bool {
    let u = url.to_ascii_lowercase();
    u.contains("paypal.com")
        || u.contains("patreon.com")
        || u.contains("boosty.to")
        || u.contains("donate")
        || u.contains("payment")
}

fn looks_like_html(bytes: &[u8]) -> bool {
    let sample = String::from_utf8_lossy(&bytes[..bytes.len().min(512)]).to_ascii_lowercase();
    sample.contains("<html") || sample.contains("<!doctype html") || sample.contains("<body")
}

fn detect_book_extension(
    url: &str,
    content_type: &str,
    content_disposition: &str,
    bytes: &[u8],
) -> Option<&'static str> {
    // Magic bytes
    if bytes.starts_with(b"PK\x03\x04") {
        // zip/epub
        if content_type.contains("epub") || url_has_ext(url, "epub") || cd_has_ext(content_disposition, "epub")
        {
            return Some("epub");
        }
        // Could still be epub without correct mime
        if looks_like_epub(bytes) {
            return Some("epub");
        }
    }
    if bytes.starts_with(b"<?xml") || bytes.starts_with(b"\xEF\xBB\xBF<?xml") {
        let head = String::from_utf8_lossy(&bytes[..bytes.len().min(400)]).to_ascii_lowercase();
        if head.contains("fictionbook") || head.contains("<fictionbook") {
            return Some("fb2");
        }
    }
    if content_type.contains("epub") || url_has_ext(url, "epub") || cd_has_ext(content_disposition, "epub")
    {
        return Some("epub");
    }
    if content_type.contains("fb2")
        || url_has_ext(url, "fb2")
        || cd_has_ext(content_disposition, "fb2")
    {
        return Some("fb2");
    }
    if url_has_ext(url, "txt") || cd_has_ext(content_disposition, "txt") {
        return Some("txt");
    }
    if url_has_ext(url, "md")
        || url_has_ext(url, "markdown")
        || cd_has_ext(content_disposition, "md")
    {
        return Some("md");
    }
    None
}

fn looks_like_epub(bytes: &[u8]) -> bool {
    // crude: PK zip and "mimetype" / "epub" somewhere in first 4k
    if !bytes.starts_with(b"PK") {
        return false;
    }
    let head = &bytes[..bytes.len().min(8192)];
    head.windows(8).any(|w| w == b"mimetype") || head.windows(4).any(|w| w == b"epub")
}

fn url_has_ext(url: &str, ext: &str) -> bool {
    let path = url.split('?').next().unwrap_or(url).to_ascii_lowercase();
    path.ends_with(&format!(".{ext}"))
}

fn cd_has_ext(cd: &str, ext: &str) -> bool {
    cd.to_ascii_lowercase().contains(&format!(".{ext}"))
}

fn extract_book_links(html: &str, base_url: &str) -> Vec<String> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"(?i)href\s*=\s*["']([^"']+)["']"#).expect("href regex")
    });

    let mut out = Vec::new();
    for caps in re.captures_iter(html) {
        let href = caps.get(1).map(|m| m.as_str()).unwrap_or("");
        if href.is_empty() || href.starts_with('#') || href.starts_with("mailto:") {
            continue;
        }
        let lower = href.to_ascii_lowercase();
        let is_book_ext = [".epub", ".fb2", ".fb2.zip", ".txt", ".md"]
            .iter()
            .any(|e| lower.contains(e));
        let is_downloadish = lower.contains("download")
            || lower.contains("getfile")
            || lower.contains("/dl/")
            || lower.contains("attachment");
        if !(is_book_ext || is_downloadish) {
            continue;
        }
        if let Some(abs) = resolve_url(base_url, href) {
            if !out.contains(&abs) {
                out.push(abs);
            }
        }
    }
    // Prefer explicit book extensions first
    out.sort_by_key(|u| {
        let l = u.to_ascii_lowercase();
        if l.contains(".epub") {
            0
        } else if l.contains(".fb2") {
            1
        } else {
            2
        }
    });
    out
}

fn resolve_url(base: &str, href: &str) -> Option<String> {
    if href.starts_with("http://") || href.starts_with("https://") {
        return Some(href.to_string());
    }
    let base = reqwest::Url::parse(base).ok()?;
    base.join(href).ok().map(|u| u.to_string())
}

fn filename_from_url(url: &str, ext: &str) -> String {
    let path = url.split('?').next().unwrap_or(url);
    let name = path.rsplit('/').next().unwrap_or("book");
    let name = urlencoding_decode(name);
    if name.to_ascii_lowercase().ends_with(&format!(".{ext}")) {
        sanitize_filename(&name)
    } else {
        sanitize_filename(&format!("{name}.{ext}"))
    }
}

fn urlencoding_decode(s: &str) -> String {
    // minimal percent-decode
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(a), Some(b)) = (from_hex(bytes[i + 1]), from_hex(bytes[i + 2])) {
                out.push((a << 4) | b);
                i += 3;
                continue;
            }
        }
        if bytes[i] == b'+' {
            out.push(b' ');
        } else {
            out.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn from_hex(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn sanitize_filename(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|c| match c {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.');
    if cleaned.is_empty() {
        "book".into()
    } else {
        cleaned.chars().take(120).collect()
    }
}

fn unique_path(dir: &Path, filename: &str) -> PathBuf {
    let candidate = dir.join(filename);
    if !candidate.exists() {
        return candidate;
    }
    let stem = Path::new(filename)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("book");
    let ext = Path::new(filename)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("bin");
    for n in 2..1000 {
        let p = dir.join(format!("{stem}-{n}.{ext}"));
        if !p.exists() {
            return p;
        }
    }
    dir.join(format!(
        "{stem}-{}.{}",
        uuid::Uuid::new_v4(),
        ext
    ))
}

/// Used by callers that only need format detection from path.
#[allow(dead_code)]
pub fn format_for_path(path: &Path) -> Option<BookFormat> {
    formats::detect_format(path)
}
