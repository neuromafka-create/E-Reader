/// Extract inner content of the first `<body>...</body>` block.
pub fn extract_body_inner(xhtml: &str) -> String {
    let lower = xhtml.to_ascii_lowercase();
    if let Some(start_tag) = lower.find("<body") {
        if let Some(gt) = xhtml[start_tag..].find('>') {
            let content_start = start_tag + gt + 1;
            if let Some(end_rel) = lower[content_start..].find("</body>") {
                return xhtml[content_start..content_start + end_rel].trim().to_string();
            }
        }
    }
    xhtml.to_string()
}

pub fn escape_html(input: &str) -> String {
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

pub fn mime_from_path(path: &str) -> &'static str {
    let lower = path.to_ascii_lowercase();
    if lower.ends_with(".png") {
        "image/png"
    } else if lower.ends_with(".jpg") || lower.ends_with(".jpeg") {
        "image/jpeg"
    } else if lower.ends_with(".gif") {
        "image/gif"
    } else if lower.ends_with(".webp") {
        "image/webp"
    } else if lower.ends_with(".svg") {
        "image/svg+xml"
    } else if lower.ends_with(".avif") {
        "image/avif"
    } else {
        "application/octet-stream"
    }
}

pub fn extension_for_mime(mime: &str) -> &'static str {
    match mime {
        "image/png" => "png",
        "image/jpeg" | "image/jpg" => "jpg",
        "image/gif" => "gif",
        "image/webp" => "webp",
        "image/svg+xml" => "svg",
        "image/avif" => "avif",
        _ => "bin",
    }
}

/// A heading found in HTML (for TOC).
#[derive(Debug, Clone)]
pub struct HtmlHeading {
    pub level: u8,
    pub text: String,
    pub id: Option<String>,
}

/// All h1–h3 headings in document order.
pub fn extract_headings(html: &str) -> Vec<HtmlHeading> {
    heading_regex()
        .captures_iter(html)
        .filter_map(|caps| heading_from_caps(&caps))
        .collect()
}

/// Ensure every h1–h3 has an `id` (inject generated ones). Returns rewritten HTML + headings.
pub fn ensure_heading_ids(html: &str, id_prefix: &str) -> (String, Vec<HtmlHeading>) {
    let re = heading_regex();
    let mut headings = Vec::new();
    let mut auto = 0u32;
    let mut out = String::with_capacity(html.len() + 64);
    let mut last = 0;

    for caps in re.captures_iter(html) {
        let Some(full) = caps.get(0) else { continue };
        let Some(mut h) = heading_from_caps(&caps) else {
            continue;
        };
        out.push_str(&html[last..full.start()]);

        if h.id.is_none() {
            auto += 1;
            h.id = Some(format!("{id_prefix}h{auto}"));
        }
        let id = h.id.clone().unwrap_or_default();
        let level = h.level;
        let inner = caps.get(3).map(|m| m.as_str()).unwrap_or("");
        let attrs = caps.get(2).map(|m| m.as_str()).unwrap_or("");
        let new_open = if attr_value(attrs, "id").is_some() {
            format!("<h{level}{attrs}>")
        } else {
            let attrs = attrs.trim();
            if attrs.is_empty() {
                format!(r#"<h{level} id="{id}">"#)
            } else {
                format!(r#"<h{level} {attrs} id="{id}">"#)
            }
        };
        out.push_str(&new_open);
        out.push_str(inner);
        out.push_str(&format!("</h{level}>"));
        last = full.end();
        headings.push(h);
    }
    out.push_str(&html[last..]);
    (out, headings)
}

fn heading_regex() -> &'static regex::Regex {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    RE.get_or_init(|| {
        // Match hN ... </hN> for N=1..3 without backreferences by scanning each open tag carefully.
        // This pattern allows mismatched close tags; ensure_heading_ids rewrites with correct close.
        regex::Regex::new(r"(?is)<h([1-3])\b([^>]*)>(.*?)</h[1-3]>").expect("heading list regex")
    })
}

fn heading_from_caps(caps: &regex::Captures<'_>) -> Option<HtmlHeading> {
    let level: u8 = caps.get(1)?.as_str().parse().ok()?;
    let attrs = caps.get(2).map(|m| m.as_str()).unwrap_or("");
    let inner = caps.get(3).map(|m| m.as_str()).unwrap_or("");
    let text = strip_tags(inner);
    let text = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if text.is_empty() {
        return None;
    }
    let id = attr_value(attrs, "id");
    Some(HtmlHeading { level, text, id })
}

/// First heading text in HTML fragment, if any (h1–h3).
pub fn first_heading_text(html: &str) -> Option<String> {
    extract_headings(html).into_iter().next().map(|h| h.text)
}

fn attr_value(attrs: &str, name: &str) -> Option<String> {
    static RE_CACHE: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, regex::Regex>>> =
        std::sync::OnceLock::new();
    let cache = RE_CACHE.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()));
    let mut map = cache.lock().ok()?;
    let re = map.entry(name.to_string()).or_insert_with(|| {
        regex::Regex::new(&format!(
            r#"(?i)\b{name}\s*=\s*(?:\"([^\"]*)\"|'([^']*)'|([^\s>]+))"#
        ))
        .expect("attr regex")
    });
    let caps = re.captures(attrs)?;
    caps.get(1)
        .or_else(|| caps.get(2))
        .or_else(|| caps.get(3))
        .map(|m| m.as_str().to_string())
        .filter(|s| !s.is_empty())
}

/// Strip HTML tags (simple, for titles/TOC).
pub fn strip_tags(html: &str) -> String {
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
    // decode a few common entities
    out.replace("&nbsp;", " ")
        .replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
}
