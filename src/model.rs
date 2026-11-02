//! Shared data contract between the UI (JS), the highlighter, the renderer and the export commands.
//! Field names are serialized as camelCase so the JS side can send/receive them as-is.

use serde::{Deserialize, Serialize};

/// Languages offered in the UI. Only these four are supported (product requirement).
/// `Auto` means: detect from the code (see highlight::detect_language).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Auto,
    #[serde(rename = "csharp")]
    CSharp,
    Html,
    Css,
    #[serde(rename = "javascript")]
    JavaScript,
}

/// Theme family. Only Candy and Sand are provided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Candy,
    Sand,
}

/// Full editor/export state. Built by the UI and sent with every command.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub code: String,
    pub language: Language,
    pub title: String,
    pub subtitle: String,
    pub theme: Theme,
    /// "Dark mode" switch.
    pub dark: bool,
    /// "Background" switch: when false, the gradient background is not drawn.
    pub background: bool,
    /// "Line numbers" switch.
    pub line_numbers: bool,
    /// One of 16, 32, 64, 128.
    pub padding: u32,
    /// Font id, one of FONT_IDS.
    pub font: String,
    /// 1-based line numbers that are highlighted (focus lines).
    pub highlighted_lines: Vec<usize>,
    /// Optional fixed frame width in px (None = automatic).
    pub width: Option<u32>,
    /// Export scale: 2, 4 or 6.
    pub scale: u32,
    /// Whether the window chrome (traffic lights + title bar) is shown.
    pub window_chrome: bool,
}

impl Snapshot {
    /// Clamps values sent by the UI to the supported options, so the backend never renders
    /// an unknown font, padding or export scale.
    pub fn sanitized(mut self) -> Self {
        if !PADDING_OPTIONS.contains(&self.padding) {
            self.padding = 64;
        }
        if !SCALE_OPTIONS.contains(&self.scale) {
            self.scale = 2;
        }
        if !FONT_IDS.contains(&self.font.as_str()) {
            self.font = FONT_IDS[0].to_string();
        }
        self
    }
}

/// A colored run of text inside a line.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub text: String,
    pub color: String,
    pub italic: bool,
}

pub const FONT_IDS: [&str; 10] = [
    "jetbrains-mono",
    "geist-mono",
    "ibm-plex-mono",
    "fira-code",
    "soehne-mono",
    "roboto-mono",
    "commit-mono",
    "space-mono",
    "source-code-pro",
    "google-sans-code",
];

pub const PADDING_OPTIONS: [u32; 4] = [16, 32, 64, 128];
pub const SCALE_OPTIONS: [u32; 3] = [2, 4, 6];
