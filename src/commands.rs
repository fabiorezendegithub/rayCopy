//! Tauri commands registered in main.rs: highlighting, formatting, and export (save/copy).

use crate::model::{Language, Snapshot, Token};
use crate::{highlight, render};

#[tauri::command]
pub fn highlight(snap: Snapshot) -> Vec<Vec<Token>> {
    highlight::highlight_lines(&snap.sanitized())
}

#[tauri::command]
pub fn detect_language(code: String) -> Language {
    highlight::detect_language(&code)
}

#[tauri::command]
pub fn format_code(code: String, language: Language) -> String {
    highlight::format_code(&code, language)
}

/// Saves the snapshot as PNG through a native save dialog.
/// Returns the saved path, or None if the user cancelled.
#[tauri::command]
pub async fn export_png(snap: Snapshot) -> Result<Option<String>, String> {
    let snap = snap.sanitized();
    let bytes = render::render_png(&snap)?;
    save_bytes(&snap, "png", "PNG", &bytes).await
}

/// Saves the snapshot as SVG through a native save dialog.
/// Returns the saved path, or None if the user cancelled.
#[tauri::command]
pub async fn export_svg(snap: Snapshot) -> Result<Option<String>, String> {
    let snap = snap.sanitized();
    let svg = render::build_svg(&snap);
    save_bytes(&snap, "svg", "SVG", svg.as_bytes()).await
}

/// Copies the rendered PNG to the system clipboard as an image.
#[tauri::command]
pub async fn copy_png(snap: Snapshot) -> Result<(), String> {
    let snap = snap.sanitized();
    let png_bytes = render::render_png(&snap)?;
    tauri::async_runtime::spawn_blocking(move || copy_image_to_clipboard(&png_bytes))
        .await
        .map_err(|e| e.to_string())?
}

async fn save_bytes(
    snap: &Snapshot,
    extension: &str,
    filter_name: &str,
    bytes: &[u8],
) -> Result<Option<String>, String> {
    let file_name = format!("{}.{}", file_stem(&snap.title), extension);
    let handle = rfd::AsyncFileDialog::new()
        .set_file_name(&file_name)
        .add_filter(filter_name, &[extension])
        .save_file()
        .await;

    match handle {
        Some(handle) => {
            let path = handle.path().to_path_buf();
            std::fs::write(&path, bytes).map_err(|e| format!("could not write file: {e}"))?;
            Ok(Some(path.display().to_string()))
        }
        None => Ok(None),
    }
}

/// Turns the title into a safe file name stem. Falls back to "code".
fn file_stem(title: &str) -> String {
    let cleaned: String = title
        .trim()
        .chars()
        .map(|c| match c {
            '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|' => '_',
            c if c.is_control() => '_',
            c => c,
        })
        .collect();
    if cleaned.trim_matches(|c: char| c == '.' || c.is_whitespace()).is_empty() {
        "code".to_string()
    } else {
        cleaned
    }
}

/// Decodes the PNG bytes into RGBA pixels and places them on the clipboard.
fn copy_image_to_clipboard(png_bytes: &[u8]) -> Result<(), String> {
    let decoder = png::Decoder::new(std::io::Cursor::new(png_bytes));
    let mut reader = decoder.read_info().map_err(|e| format!("png decode failed: {e}"))?;
    let mut buffer = vec![0u8; reader.output_buffer_size()];
    let info = reader
        .next_frame(&mut buffer)
        .map_err(|e| format!("png decode failed: {e}"))?;

    if info.color_type != png::ColorType::Rgba || info.bit_depth != png::BitDepth::Eight {
        return Err("unexpected PNG format for clipboard".into());
    }
    buffer.truncate(info.buffer_size());

    let mut clipboard = arboard::Clipboard::new().map_err(|e| format!("clipboard unavailable: {e}"))?;
    clipboard
        .set_image(arboard::ImageData {
            width: info.width as usize,
            height: info.height as usize,
            bytes: std::borrow::Cow::Owned(buffer),
        })
        .map_err(|e| format!("could not copy image: {e}"))
}

#[cfg(test)]
mod tests {
    use super::file_stem;

    #[test]
    fn file_stem_sanitizes_and_falls_back() {
        assert_eq!(file_stem("hello/world"), "hello_world");
        assert_eq!(file_stem("   "), "code");
        assert_eq!(file_stem(""), "code");
        assert_eq!(file_stem("Untitled-1"), "Untitled-1");
    }
}






