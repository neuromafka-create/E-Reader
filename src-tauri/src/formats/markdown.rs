use super::ParsedBook;
use crate::error::AppResult;
use crate::models::TocEntry;
use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use std::path::Path;

pub fn parse(path: &Path) -> AppResult<ParsedBook> {
    let raw = std::fs::read_to_string(path)?;
    let mut options = Options::empty();
    options.insert(Options::ENABLE_TABLES);
    options.insert(Options::ENABLE_STRIKETHROUGH);
    options.insert(Options::ENABLE_TASKLISTS);
    options.insert(Options::ENABLE_FOOTNOTES);

    let parser = Parser::new_ext(&raw, options);
    let mut toc = Vec::new();
    let mut heading_count = 0u32;
    let mut events = Vec::new();
    let mut in_heading = false;
    let mut heading_level = 1u8;
    let mut heading_text = String::new();

    for event in parser {
        match &event {
            Event::Start(Tag::Heading { level, .. }) => {
                in_heading = true;
                heading_level = match level {
                    HeadingLevel::H1 => 1,
                    HeadingLevel::H2 => 2,
                    HeadingLevel::H3 => 3,
                    HeadingLevel::H4 => 4,
                    HeadingLevel::H5 => 5,
                    HeadingLevel::H6 => 6,
                };
                heading_text.clear();
                heading_count += 1;
                let id = format!("heading-{heading_count}");
                events.push(Event::Html(
                    format!(r#"<h{heading_level} id="{id}">"#).into(),
                ));
                continue;
            }
            Event::End(TagEnd::Heading(_)) if in_heading => {
                let id = format!("heading-{heading_count}");
                toc.push(TocEntry {
                    id: id.clone(),
                    title: if heading_text.trim().is_empty() {
                        format!("Section {heading_count}")
                    } else {
                        heading_text.trim().to_string()
                    },
                    level: heading_level,
                });
                events.push(Event::Html(format!("</h{heading_level}>").into()));
                in_heading = false;
                continue;
            }
            Event::Text(text) if in_heading => {
                heading_text.push_str(text);
            }
            Event::Code(text) if in_heading => {
                heading_text.push_str(text);
            }
            _ => {}
        }
        events.push(event);
    }

    let mut html = String::from(r#"<article class="markdown-book">"#);
    pulldown_cmark::html::push_html(&mut html, events.into_iter());
    html.push_str("</article>");

    let title = toc
        .iter()
        .find(|e| e.level == 1)
        .map(|e| e.title.clone());

    Ok(ParsedBook {
        html,
        toc,
        title,
        author: None,
        cover: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn markdown_toc_and_title() {
        let dir = std::env::temp_dir().join("e-reader-md-test.md");
        let mut f = std::fs::File::create(&dir).unwrap();
        writeln!(f, "# Title\n\n## Section\n\nBody.").unwrap();
        let parsed = parse(&dir).unwrap();
        assert_eq!(parsed.title.as_deref(), Some("Title"));
        assert!(parsed.toc.iter().any(|t| t.title == "Section"));
        assert!(parsed.html.contains("id=\"heading-1\""));
        let _ = std::fs::remove_file(dir);
    }
}
