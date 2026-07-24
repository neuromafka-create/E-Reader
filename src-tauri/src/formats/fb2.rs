use super::html_util::escape_html;
use super::{BookMeta, CoverImage, ParsedBook};
use crate::error::{AppError, AppResult};
use crate::models::TocEntry;
use fb2::{
    Author, FictionBook, Image, InlineImage, Section, SectionPart, StyleElement, Title, TitleElement,
};
use std::collections::HashMap;
use std::io::BufReader;
use std::path::Path;

pub fn parse(path: &Path) -> AppResult<ParsedBook> {
    let book = load(path)?;
    let title_info = &book.description.title_info;
    let title = Some(title_info.book_title.value.clone()).filter(|s| !s.trim().is_empty());
    let author = first_author(&title_info.authors);

    let binaries = binary_map(&book);
    let cover = cover_from_book(&book, &binaries);

    let mut toc = Vec::new();
    let mut html = String::from(r#"<article class="fb2-book">"#);

    if let Some(body) = book.bodies.first() {
        if let Some(body_title) = &body.title {
            html.push_str(&render_title(body_title, 1, None, &mut toc, &binaries));
        }
        for (i, section) in body.sections.iter().enumerate() {
            html.push_str(&render_section(section, 1, i, &mut toc, &binaries));
        }
    }
    html.push_str("</article>");

    Ok(ParsedBook {
        html,
        toc,
        title,
        author,
        cover,
    })
}

pub fn metadata(path: &Path) -> AppResult<BookMeta> {
    let book = load(path)?;
    let title_info = &book.description.title_info;
    let title = if title_info.book_title.value.trim().is_empty() {
        super::title_from_path(path)
    } else {
        title_info.book_title.value.clone()
    };
    let author = first_author(&title_info.authors);
    let binaries = binary_map(&book);
    Ok(BookMeta {
        title,
        author,
        cover: cover_from_book(&book, &binaries),
    })
}

fn load(path: &Path) -> AppResult<FictionBook> {
    let file = std::fs::File::open(path)?;
    let reader = BufReader::new(file);
    quick_xml::de::from_reader(reader).map_err(|e| AppError::msg(format!("FB2 parse error: {e}")))
}

fn binary_map(book: &FictionBook) -> HashMap<String, (String, String)> {
    // id -> (mime, base64 content already in file)
    let mut map = HashMap::new();
    for bin in &book.binaries {
        let id = bin.id.trim_start_matches('#').to_string();
        let content = bin.content.split_whitespace().collect::<String>();
        if !content.is_empty() {
            map.insert(id, (bin.content_type.clone(), content));
        }
    }
    map
}

fn cover_from_book(
    book: &FictionBook,
    binaries: &HashMap<String, (String, String)>,
) -> Option<CoverImage> {
    let href = book
        .description
        .title_info
        .cover_page
        .as_ref()
        .and_then(|c| c.images.first())
        .and_then(|img| img.href.as_ref())
        .map(|h| h.trim_start_matches('#').to_string())?;
    let (mime, b64) = binaries.get(&href)?;
    use base64::{engine::general_purpose::STANDARD as B64, Engine};
    let bytes = B64.decode(b64).ok()?;
    Some(CoverImage {
        bytes,
        mime: mime.clone(),
    })
}

fn first_author(authors: &[Author]) -> Option<String> {
    authors.first().map(format_author).filter(|s| !s.is_empty())
}

fn format_author(author: &Author) -> String {
    match author {
        Author::Verbose(v) => {
            let mut parts = Vec::new();
            if !v.first_name.value.trim().is_empty() {
                parts.push(v.first_name.value.trim());
            }
            if let Some(m) = &v.middle_name {
                if !m.value.trim().is_empty() {
                    parts.push(m.value.trim());
                }
            }
            if !v.last_name.value.trim().is_empty() {
                parts.push(v.last_name.value.trim());
            }
            if parts.is_empty() {
                v.nickname
                    .as_ref()
                    .map(|n| n.value.trim().to_string())
                    .unwrap_or_default()
            } else {
                parts.join(" ")
            }
        }
        Author::Anonymous(a) => a
            .nickname
            .as_ref()
            .map(|n| n.value.trim().to_string())
            .unwrap_or_else(|| "Anonymous".into()),
    }
}

fn render_section(
    section: &Section,
    level: u8,
    index: usize,
    toc: &mut Vec<TocEntry>,
    binaries: &HashMap<String, (String, String)>,
) -> String {
    let id = section
        .id
        .clone()
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| format!("section-{level}-{index}"));

    let mut html = format!(r#"<section class="fb2-section" id="{id}">"#);

    if let Some(content) = &section.content {
        if let Some(title) = &content.title {
            html.push_str(&render_title(title, level, Some(&id), toc, binaries));
        } else if let Some(fallback) = fallback_section_title(content) {
            // Many FB2s omit <title> and only use subtitle / short first line.
            toc.push(TocEntry {
                id: id.clone(),
                title: fallback,
                level,
            });
        }
        if let Some(image) = &content.image {
            html.push_str(&render_image(image, binaries));
        }
        for part in &content.content {
            html.push_str(&render_part(part, binaries));
        }
        for (i, nested) in content.sections.iter().enumerate() {
            html.push_str(&render_section(
                nested,
                level.saturating_add(1).min(6),
                i,
                toc,
                binaries,
            ));
        }
    }

    html.push_str("</section>");
    html
}

/// Title fallback when FB2 section has no formal <title>.
fn fallback_section_title(content: &fb2::SectionContent) -> Option<String> {
    use fb2::SectionPart;
    for part in &content.content {
        match part {
            SectionPart::Subtitle(p) => {
                let t = style_elements_plain(&p.elements);
                if !t.is_empty() {
                    return Some(t);
                }
            }
            SectionPart::Paragraph(p) => {
                let t = style_elements_plain(&p.elements);
                // Short first paragraph often acts as a chapter heading in sloppy FB2s.
                if !t.is_empty() && t.chars().count() <= 80 {
                    return Some(t);
                }
                break;
            }
            SectionPart::EmptyLine => continue,
            _ => break,
        }
    }
    None
}

fn style_elements_plain(elements: &[StyleElement]) -> String {
    let mut out = String::new();
    for el in elements {
        match el {
            StyleElement::Text(t) => out.push_str(t),
            StyleElement::Strong(s)
            | StyleElement::Emphasis(s)
            | StyleElement::Strikethrough(s)
            | StyleElement::Subscript(s)
            | StyleElement::Superscript(s)
            | StyleElement::Code(s) => out.push_str(&style_elements_plain(&s.elements)),
            StyleElement::Style(s) => out.push_str(&style_elements_plain(&s.elements)),
            StyleElement::Link(link) => {
                // best-effort: only plain text children if any via Display-less walk
                let _ = link;
            }
            StyleElement::Image(_) => {}
        }
    }
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn render_title(
    title: &Title,
    level: u8,
    id: Option<&str>,
    toc: &mut Vec<TocEntry>,
    binaries: &HashMap<String, (String, String)>,
) -> String {
    let mut text = String::new();
    let mut inner = String::new();
    for el in &title.elements {
        match el {
            TitleElement::Paragraph(p) => {
                let rendered = render_style_elements(&p.elements, binaries);
                if !text.is_empty() {
                    text.push(' ');
                }
                text.push_str(&strip_tags(&rendered));
                inner.push_str(&format!("<div class=\"fb2-title-line\">{rendered}</div>"));
            }
            TitleElement::EmptyLine => inner.push_str("<br/>"),
        }
    }

    if let Some(section_id) = id {
        if !text.trim().is_empty() {
            toc.push(TocEntry {
                id: section_id.to_string(),
                title: text.trim().to_string(),
                level,
            });
        }
    }

    let tag = format!("h{}", level.clamp(1, 6));
    format!("<{tag} class=\"fb2-title\">{inner}</{tag}>")
}

fn render_part(part: &SectionPart, binaries: &HashMap<String, (String, String)>) -> String {
    match part {
        SectionPart::Paragraph(p) => {
            format!("<p>{}</p>", render_style_elements(&p.elements, binaries))
        }
        SectionPart::Subtitle(p) => format!(
            "<h3 class=\"fb2-subtitle\">{}</h3>",
            render_style_elements(&p.elements, binaries)
        ),
        SectionPart::EmptyLine => "<p class=\"fb2-empty\">&nbsp;</p>".into(),
        SectionPart::Image(img) => render_image(img, binaries),
        SectionPart::Cite(cite) => {
            let mut html = String::from("<blockquote class=\"fb2-cite\">");
            for el in &cite.elements {
                match el {
                    fb2::CiteElement::Paragraph(p) => {
                        html.push_str(&format!(
                            "<p>{}</p>",
                            render_style_elements(&p.elements, binaries)
                        ));
                    }
                    fb2::CiteElement::Poem(poem) => html.push_str(&render_poem(poem, binaries)),
                    fb2::CiteElement::Subtitle(p) => {
                        html.push_str(&format!(
                            "<h4>{}</h4>",
                            render_style_elements(&p.elements, binaries)
                        ));
                    }
                    fb2::CiteElement::EmptyLine => html.push_str("<br/>"),
                    fb2::CiteElement::Table(_) => html.push_str("<!-- table omitted -->"),
                }
            }
            for author in &cite.text_authors {
                html.push_str(&format!(
                    "<footer>{}</footer>",
                    render_style_elements(&author.elements, binaries)
                ));
            }
            html.push_str("</blockquote>");
            html
        }
        SectionPart::Poem(poem) => render_poem(poem, binaries),
        SectionPart::Table(_) => "<p><em>[table]</em></p>".into(),
    }
}

fn render_poem(poem: &fb2::Poem, binaries: &HashMap<String, (String, String)>) -> String {
    let mut html = String::from(r#"<div class="fb2-poem">"#);
    if let Some(title) = &poem.title {
        let mut dummy = Vec::new();
        html.push_str(&render_title(title, 3, None, &mut dummy, binaries));
    }
    for stanza in &poem.stanzas {
        match stanza {
            fb2::PoemStanza::Subtitle(p) => {
                html.push_str(&format!(
                    "<h4 class=\"fb2-subtitle\">{}</h4>",
                    render_style_elements(&p.elements, binaries)
                ));
            }
            fb2::PoemStanza::Stanza(st) => {
                html.push_str(r#"<div class="fb2-stanza">"#);
                for line in &st.lines {
                    html.push_str(&format!(
                        "<div class=\"fb2-verse\">{}</div>",
                        render_style_elements(&line.elements, binaries)
                    ));
                }
                html.push_str("</div>");
            }
        }
    }
    for author in &poem.text_authors {
        html.push_str(&format!(
            "<footer>{}</footer>",
            render_style_elements(&author.elements, binaries)
        ));
    }
    html.push_str("</div>");
    html
}

fn render_style_elements(
    elements: &[StyleElement],
    binaries: &HashMap<String, (String, String)>,
) -> String {
    let mut out = String::new();
    for el in elements {
        match el {
            StyleElement::Text(t) => out.push_str(&escape_html(t)),
            StyleElement::Strong(s) => {
                out.push_str(&format!(
                    "<strong>{}</strong>",
                    render_style_elements(&s.elements, binaries)
                ));
            }
            StyleElement::Emphasis(s) => {
                out.push_str(&format!(
                    "<em>{}</em>",
                    render_style_elements(&s.elements, binaries)
                ));
            }
            StyleElement::Strikethrough(s) => {
                out.push_str(&format!(
                    "<s>{}</s>",
                    render_style_elements(&s.elements, binaries)
                ));
            }
            StyleElement::Subscript(s) => {
                out.push_str(&format!(
                    "<sub>{}</sub>",
                    render_style_elements(&s.elements, binaries)
                ));
            }
            StyleElement::Superscript(s) => {
                out.push_str(&format!(
                    "<sup>{}</sup>",
                    render_style_elements(&s.elements, binaries)
                ));
            }
            StyleElement::Code(s) => {
                out.push_str(&format!(
                    "<code>{}</code>",
                    render_style_elements(&s.elements, binaries)
                ));
            }
            StyleElement::Style(s) => {
                out.push_str(&render_style_elements(&s.elements, binaries));
            }
            StyleElement::Link(link) => {
                let href = link.href.as_deref().unwrap_or("#");
                let inner = render_style_link_elements(&link.elements, binaries);
                out.push_str(&format!(
                    r#"<a href="{}">{}</a>"#,
                    escape_html(href),
                    inner
                ));
            }
            StyleElement::Image(img) => out.push_str(&render_inline_image(img, binaries)),
        }
    }
    out
}

fn render_image(image: &Image, binaries: &HashMap<String, (String, String)>) -> String {
    let alt = image.alt.as_deref().unwrap_or("");
    if let Some(href) = &image.href {
        let key = href.trim_start_matches('#');
        if let Some((mime, b64)) = binaries.get(key) {
            return format!(
                r#"<figure class="fb2-image"><img src="data:{mime};base64,{b64}" alt="{}"/></figure>"#,
                escape_html(alt)
            );
        }
    }
    String::new()
}

fn render_style_link_elements(
    elements: &[fb2::StyleLinkElement],
    binaries: &HashMap<String, (String, String)>,
) -> String {
    let mut out = String::new();
    for el in elements {
        match el {
            fb2::StyleLinkElement::Text(t) => out.push_str(&escape_html(t)),
            fb2::StyleLinkElement::Strong { elements } => {
                out.push_str(&format!(
                    "<strong>{}</strong>",
                    render_style_link_elements(elements, binaries)
                ));
            }
            fb2::StyleLinkElement::Emphasis { elements } => {
                out.push_str(&format!(
                    "<em>{}</em>",
                    render_style_link_elements(elements, binaries)
                ));
            }
            fb2::StyleLinkElement::Style { elements } => {
                out.push_str(&render_style_link_elements(elements, binaries));
            }
            fb2::StyleLinkElement::Strikethrough { elements } => {
                out.push_str(&format!(
                    "<s>{}</s>",
                    render_style_link_elements(elements, binaries)
                ));
            }
            fb2::StyleLinkElement::Subscript { elements } => {
                out.push_str(&format!(
                    "<sub>{}</sub>",
                    render_style_link_elements(elements, binaries)
                ));
            }
            fb2::StyleLinkElement::Superscript { elements } => {
                out.push_str(&format!(
                    "<sup>{}</sup>",
                    render_style_link_elements(elements, binaries)
                ));
            }
            fb2::StyleLinkElement::Code { elements } => {
                out.push_str(&format!(
                    "<code>{}</code>",
                    render_style_link_elements(elements, binaries)
                ));
            }
            fb2::StyleLinkElement::Image(img) => out.push_str(&render_inline_image(img, binaries)),
        }
    }
    out
}

fn render_inline_image(
    image: &InlineImage,
    binaries: &HashMap<String, (String, String)>,
) -> String {
    let alt = image.alt.as_deref().unwrap_or("");
    if let Some(href) = &image.href {
        let key = href.trim_start_matches('#');
        if let Some((mime, b64)) = binaries.get(key) {
            return format!(
                r#"<img class="fb2-inline-image" src="data:{mime};base64,{b64}" alt="{}"/>"#,
                escape_html(alt)
            );
        }
    }
    String::new()
}

fn strip_tags(html: &str) -> String {
    let mut out = String::new();
    let mut in_tag = false;
    for ch in html.chars() {
        match ch {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(ch),
            _ => {}
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn parse_sample_fb2() {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("samples")
            .join("sample.fb2");
        let parsed = parse(&path).expect("parse fb2");
        assert!(parsed.html.contains("FictionBook") || parsed.html.contains("fb2"));
        assert_eq!(parsed.author.as_deref(), Some("Ada Lovelace"));
        assert!(!parsed.toc.is_empty());
    }
}
