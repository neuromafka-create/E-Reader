use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BookFormat {
    Txt,
    Markdown,
    Epub,
    Fb2,
}

impl BookFormat {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Txt => "txt",
            Self::Markdown => "markdown",
            Self::Epub => "epub",
            Self::Fb2 => "fb2",
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "txt" => Some(Self::Txt),
            "md" | "markdown" => Some(Self::Markdown),
            "epub" => Some(Self::Epub),
            "fb2" => Some(Self::Fb2),
            _ => None,
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "txt" => Some(Self::Txt),
            "markdown" => Some(Self::Markdown),
            "epub" => Some(Self::Epub),
            "fb2" => Some(Self::Fb2),
            _ => None,
        }
    }

    pub fn is_readable(&self) -> bool {
        matches!(
            self,
            Self::Txt | Self::Markdown | Self::Epub | Self::Fb2
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Book {
    pub id: String,
    pub path: String,
    pub title: String,
    pub author: Option<String>,
    pub format: BookFormat,
    pub cover_path: Option<String>,
    pub file_size: i64,
    pub modified_at: Option<String>,
    pub added_at: String,
    pub last_opened_at: Option<String>,
    pub progress_percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LibraryRoot {
    pub id: String,
    pub path: String,
    pub added_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub book_id: String,
    pub position: String,
    pub percentage: f64,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Bookmark {
    pub id: String,
    pub book_id: String,
    pub position: String,
    pub label: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TocEntry {
    pub id: String,
    pub title: String,
    pub level: u8,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BookContent {
    pub book: Book,
    pub html: String,
    pub toc: Vec<TocEntry>,
    pub progress: Option<Progress>,
    pub readable: bool,
    pub message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanResult {
    pub added: usize,
    pub updated: usize,
    pub removed: usize,
    pub total: usize,
}

/// Result of importing paths from open-with, CLI, or drag-and-drop.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IngestResult {
    pub files_imported: usize,
    pub folders_added: usize,
    /// Book ids for successfully imported files (order preserved).
    pub book_ids: Vec<String>,
    /// First imported book id, if any (handy for auto-open).
    pub open_book_id: Option<String>,
    pub skipped: usize,
    pub message: String,
}

/// Result of downloading a remote URL into the library.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportFromUrlResult {
    pub success: bool,
    pub open_book_id: Option<String>,
    pub title: Option<String>,
    pub path: Option<String>,
    pub message: String,
    pub open_externally: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReaderSettings {
    pub font_family: String,
    pub font_size: u32,
    pub line_height: f64,
    pub theme: String,
    pub max_width: u32,
    /// UI language code: "ru" | "en"
    #[serde(default = "default_locale")]
    pub locale: String,
}

fn default_locale() -> String {
    "ru".into()
}

impl Default for ReaderSettings {
    fn default() -> Self {
        Self {
            font_family: "Georgia, 'Times New Roman', serif".into(),
            font_size: 18,
            line_height: 1.7,
            theme: "sepia".into(),
            max_width: 720,
            locale: default_locale(),
        }
    }
}
