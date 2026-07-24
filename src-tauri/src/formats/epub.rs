use super::html_util::{
    ensure_heading_ids, extract_body_inner, extract_headings, first_heading_text, mime_from_path,
};
use super::{BookMeta, CoverImage, ParsedBook};
use crate::error::{AppError, AppResult};
use crate::models::TocEntry;
use base64::{engine::general_purpose::STANDARD as B64, Engine};
use regex::Regex;
use rbook::ebook::toc::TocEntryKind;
use rbook::epub::metadata::EpubVersion;
use rbook::epub::rewrite::{EpubRewriteOptions, PathRewrite};
use rbook::epub::toc::EpubTocEntry;
use rbook::Epub;
use std::collections::HashMap;
use std::path::Path;
use std::sync::OnceLock;

pub fn parse(path: &Path) -> AppResult<ParsedBook> {
    let epub = Epub::open(path).map_err(|e| AppError::msg(format!("Failed to open EPUB: {e}")))?;

    let title = epub
        .metadata()
        .title()
        .map(|t| t.value().to_string())
        .filter(|s| !s.trim().is_empty());

    let author = epub
        .metadata()
        .creators()
        .next()
        .map(|c| c.value().to_string())
        .filter(|s| !s.trim().is_empty());

    let cover = extract_cover(&epub);

    let image_map = build_image_map(&epub);
    let mut chapter_hrefs: Vec<String> = Vec::new();
    let mut chapter_bodies: Vec<String> = Vec::new();
    let mut html = String::from(r#"<article class="epub-book">"#);

    let rewrite = EpubRewriteOptions::default().rewrite_paths(PathRewrite::root_relative());
    let reader = epub.reader_builder().rewrite(rewrite).create();

    for (index, item) in reader.enumerate() {
        let item = item.map_err(|e| AppError::msg(format!("EPUB chapter error: {e}")))?;
        let href = item.manifest_entry().href().as_ref().to_string();
        chapter_hrefs.push(normalize_href(&href));

        let mut body = extract_body_inner(item.content());
        body = rewrite_image_sources(&body, &image_map);
        body = strip_external_stylesheets(&body);
        // Guarantee heading ids so in-document TOC jumps work (single-file novels etc.)
        let (body, _) = ensure_heading_ids(&body, &format!("c{index}-"));
        chapter_bodies.push(body.clone());

        html.push_str(&format!(
            r#"<section class="epub-chapter" id="chapter-{index}" data-href="{href}">"#
        ));
        html.push_str(&body);
        html.push_str("</section>");
    }
    html.push_str("</article>");

    let toc = build_toc(&epub, &chapter_hrefs, &chapter_bodies);

    Ok(ParsedBook {
        html,
        toc,
        title,
        author,
        cover,
    })
}

pub fn metadata(path: &Path) -> AppResult<BookMeta> {
    let epub = Epub::options()
        .skip_toc(true)
        .skip_spine(true)
        .open(path)
        .map_err(|e| AppError::msg(format!("Failed to read EPUB metadata: {e}")))?;

    let title = epub
        .metadata()
        .title()
        .map(|t| t.value().to_string())
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| super::title_from_path(path));

    let author = epub
        .metadata()
        .creators()
        .next()
        .map(|c| c.value().to_string())
        .filter(|s| !s.trim().is_empty());

    Ok(BookMeta {
        title,
        author,
        cover: extract_cover(&epub),
    })
}

fn extract_cover(epub: &Epub) -> Option<CoverImage> {
    let cover = epub.manifest().cover_image()?;
    let bytes = cover.read_bytes().ok()?;
    if bytes.is_empty() {
        return None;
    }
    let mime = cover.kind().as_str().to_string();
    let mime = if mime.starts_with("image/") {
        mime
    } else {
        mime_from_path(cover.href().as_ref()).to_string()
    };
    Some(CoverImage { bytes, mime })
}

fn build_image_map(epub: &Epub) -> HashMap<String, String> {
    let mut map = HashMap::new();
    for image in epub.manifest().images() {
        let Ok(bytes) = image.read_bytes() else {
            continue;
        };
        if bytes.is_empty() {
            continue;
        }
        let kind = image.kind();
        let kind_str = kind.as_str();
        let mime = if kind_str.starts_with("image/") {
            kind_str.to_string()
        } else {
            mime_from_path(image.href().as_ref()).to_string()
        };
        let data_url = format!("data:{mime};base64,{}", B64.encode(bytes));
        let href = normalize_href(image.href().as_ref());
        map.insert(href.clone(), data_url.clone());
        // also store without leading slash
        if let Some(stripped) = href.strip_prefix('/') {
            map.insert(stripped.to_string(), data_url);
        }
    }
    map
}

fn rewrite_image_sources(html: &str, images: &HashMap<String, String>) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"(?i)(\b(?:src|href)\s*=\s*)(["'])([^"']+)(["'])"#).expect("image attr regex")
    });

    re.replace_all(html, |caps: &regex::Captures| {
        let prefix = &caps[1];
        let q1 = &caps[2];
        let value = &caps[3];
        let q2 = &caps[4];
        if value.starts_with("data:") || value.starts_with("http://") || value.starts_with("https://") {
            return caps[0].to_string();
        }
        let key = normalize_href(value);
        let data = images
            .get(&key)
            .or_else(|| images.get(value))
            .or_else(|| key.strip_prefix('/').and_then(|s| images.get(s)));
        match data {
            Some(data) => format!("{prefix}{q1}{data}{q2}"),
            None => caps[0].to_string(),
        }
    })
    .into_owned()
}

fn strip_external_stylesheets(html: &str) -> String {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| {
        Regex::new(r#"(?is)<link\b[^>]*rel\s*=\s*["']?stylesheet["']?[^>]*/?>"#)
            .expect("stylesheet regex")
    });
    re.replace_all(html, "").into_owned()
}

fn normalize_href(href: &str) -> String {
    let href = href.split('#').next().unwrap_or(href).trim();
    let href = href.replace('\\', "/");
    if href.is_empty() {
        return String::new();
    }
    // Collapse ".." segments lightly for matching
    let mut parts = Vec::new();
    for part in href.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            parts.pop();
        } else {
            parts.push(part);
        }
    }
    format!("/{}", parts.join("/"))
}

fn build_toc(epub: &Epub, chapter_hrefs: &[String], chapter_bodies: &[String]) -> Vec<TocEntry> {
    // Prefer the richest nav/NCX variant (some books only fill one of them).
    let mut candidates: Vec<Vec<TocEntry>> = Vec::new();

    if let Some(root) = epub.toc().contents() {
        let mut t = Vec::new();
        flatten_toc(&root, 1, chapter_hrefs, &mut t);
        candidates.push(t);
    }
    for ver in [EpubVersion::EPUB2, EpubVersion::EPUB3] {
        if let Some(root) = epub.toc().by_kind_version(TocEntryKind::Toc, ver) {
            let mut t = Vec::new();
            flatten_toc(&root, 1, chapter_hrefs, &mut t);
            if !t.is_empty() {
                candidates.push(t);
            }
        }
    }

    let from_nav = candidates
        .into_iter()
        .max_by_key(|t| t.len())
        .unwrap_or_default();

    // Spine-level: one entry per XHTML file
    let from_spine = toc_from_spine_files(chapter_bodies);
    // In-document: every h1–h3 (critical for single-file EPUBs with many chapters)
    let from_headings = toc_from_internal_headings(chapter_bodies);

    // Pick the richest useful TOC.
    // Typical broken case: NCX has only "Пролог", body.xhtml has 70+ h1 chapters.
    let mut best = from_nav;
    for alt in [from_headings, from_spine] {
        if alt.len() > best.len() {
            best = alt;
        }
    }
    if best.is_empty() {
        // Absolute fallback: numbered spine items
        for i in 0..chapter_bodies.len() {
            best.push(TocEntry {
                id: format!("chapter-{i}"),
                title: format!("{}", i + 1),
                level: 1,
            });
        }
    }
    best
}

/// One TOC entry per spine XHTML (first heading text if available).
fn toc_from_spine_files(bodies: &[String]) -> Vec<TocEntry> {
    bodies
        .iter()
        .enumerate()
        .map(|(i, body)| {
            let title = first_heading_text(body).unwrap_or_else(|| format!("{}", i + 1));
            TocEntry {
                id: format!("chapter-{i}"),
                title,
                level: 1,
            }
        })
        .collect()
}

/// TOC from all h1–h3 inside chapter HTML (ids preferred for in-page navigation).
///
/// Used for books that put the whole novel into one `body.xhtml` with many headings,
/// while NCX only lists a prologue (e.g. «Весна для Снежной Королевы»).
fn toc_from_internal_headings(bodies: &[String]) -> Vec<TocEntry> {
    let mut toc = Vec::new();
    let mut auto = 0u32;
    for (chapter_idx, body) in bodies.iter().enumerate() {
        let headings = extract_headings(body);
        // Skip trivial nav pages that only say "Contents"
        if headings.len() <= 1 && bodies.len() > 1 {
            if let Some(h) = headings.first() {
                let t = h.text.to_ascii_lowercase();
                if t == "contents" || t == "содержание" || t == "оглавление" {
                    continue;
                }
            }
        }
        for h in headings {
            auto += 1;
            let id = h
                .id
                .filter(|s| !s.is_empty())
                .unwrap_or_else(|| format!("chapter-{chapter_idx}-h{auto}"));
            // Prefer real chapter titles; skip decorative emoji-only noise if any
            let title = h.text.trim().to_string();
            if title.is_empty() {
                continue;
            }
            toc.push(TocEntry {
                id,
                title,
                level: h.level.min(6),
            });
        }
    }
    toc
}

fn flatten_toc(
    entry: &EpubTocEntry<'_>,
    level: u8,
    chapter_hrefs: &[String],
    out: &mut Vec<TocEntry>,
) {
    for child in entry.iter() {
        let label = child.label().trim();
        if label.is_empty() {
            flatten_toc(&child, level, chapter_hrefs, out);
            continue;
        }
        let id = child
            .href()
            .map(|h| href_to_chapter_id(h.as_ref(), chapter_hrefs))
            .unwrap_or_else(|| format!("chapter-{}", out.len()));
        out.push(TocEntry {
            id,
            title: label.to_string(),
            level: level.min(6),
        });
        flatten_toc(&child, level.saturating_add(1), chapter_hrefs, out);
    }
}

fn href_to_chapter_id(href: &str, chapter_hrefs: &[String]) -> String {
    let key = normalize_href(href);
    if let Some((idx, _)) = chapter_hrefs.iter().enumerate().find(|(_, h)| {
        **h == key || h.ends_with(&key) || key.ends_with(h.as_str())
    }) {
        return format!("chapter-{idx}");
    }
    // fallback: use fragment as id if present
    if let Some(frag) = href.split('#').nth(1) {
        if !frag.is_empty() {
            return frag.to_string();
        }
    }
    "chapter-0".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_example_epub() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("samples")
            .join("example.epub");
        if !path.exists() {
            return;
        }
        let parsed = parse(&path).expect("parse epub");
        assert!(parsed.html.contains("epub-chapter"));
        assert!(parsed.title.is_some());
    }

    /// Single-file EPUB with sparse NCX ("only Prologue") but many h1 chapters in body.xhtml.
    #[test]
    fn parse_vesna_for_snow_queen_toc() {
        let path = std::env::var("VESNA_EPUB")
            .map(PathBuf::from)
            .unwrap_or_else(|_| {
                PathBuf::from(r"C:\Users\Mafka\Documents\книги\Весна для Снежной Королевы.epub")
            });
        if !path.exists() {
            // NFD filename variants — try scan via parent
            let parent = path.parent().unwrap_or(Path::new("."));
            let found = std::fs::read_dir(parent).ok().and_then(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.path())
                    .find(|p| {
                        p.extension().and_then(|x| x.to_str()) == Some("epub")
                            && p.file_name()
                                .and_then(|n| n.to_str())
                                .map(|n| n.contains("Весна") || n.contains("Снеж"))
                                .unwrap_or(false)
                    })
            });
            let Some(path) = found else {
                eprintln!("skip: Vesna EPUB not found at {path:?}");
                return;
            };
            let parsed = parse(&path).expect("parse vesna");
            assert!(
                parsed.toc.len() >= 50,
                "expected rich TOC from headings, got {} entries: {:?}",
                parsed.toc.len(),
                parsed.toc.iter().take(5).map(|t| &t.title).collect::<Vec<_>>()
            );
            assert!(
                parsed.toc.iter().any(|t| t.title.contains("Пролог")),
                "missing prologue"
            );
            assert!(
                parsed.toc.iter().any(|t| t.title.contains("Эпилог") || t.title.contains("Генерал")),
                "missing later chapters"
            );
            return;
        }
        let parsed = parse(&path).expect("parse vesna");
        assert!(
            parsed.toc.len() >= 50,
            "expected rich TOC from headings, got {} entries: {:?}",
            parsed.toc.len(),
            parsed.toc.iter().take(8).map(|t| &t.title).collect::<Vec<_>>()
        );
    }
}
