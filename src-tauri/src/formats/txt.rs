use super::ParsedBook;
use crate::error::AppResult;
use std::path::Path;

pub fn parse(path: &Path) -> AppResult<ParsedBook> {
    let raw = std::fs::read(path)?;
    let text = decode_text(&raw);
    let html = text_to_html(&text);
    Ok(ParsedBook {
        html,
        toc: Vec::new(),
        title: None,
        author: None,
        cover: None,
    })
}

fn decode_text(bytes: &[u8]) -> String {
    // UTF-8 first, then Windows-1251-ish fallback via lossy UTF-8 for MVP.
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => String::from_utf8_lossy(bytes).into_owned(),
    }
}

fn text_to_html(text: &str) -> String {
    let mut html = String::from(r#"<article class="txt-book">"#);
    let mut paragraph = String::new();

    for line in text.lines() {
        let trimmed = line.trim_end();
        if trimmed.is_empty() {
            if !paragraph.is_empty() {
                html.push_str("<p>");
                html.push_str(&escape_html(&paragraph));
                html.push_str("</p>");
                paragraph.clear();
            }
        } else {
            if !paragraph.is_empty() {
                paragraph.push(' ');
            }
            paragraph.push_str(trimmed.trim());
        }
    }

    if !paragraph.is_empty() {
        html.push_str("<p>");
        html.push_str(&escape_html(&paragraph));
        html.push_str("</p>");
    }

    if html == r#"<article class="txt-book">"# {
        html.push_str("<p></p>");
    }

    html.push_str("</article>");
    html
}

fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn txt_paragraphs() {
        let dir = std::env::temp_dir().join("e-reader-txt-test.txt");
        let mut f = std::fs::File::create(&dir).unwrap();
        writeln!(f, "Hello\nworld\n\nSecond.").unwrap();
        let parsed = parse(&dir).unwrap();
        assert!(parsed.html.contains("<p>Hello world</p>"));
        assert!(parsed.html.contains("<p>Second.</p>"));
        let _ = std::fs::remove_file(dir);
    }
}
