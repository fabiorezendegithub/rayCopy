//! rayCopy entry point. The Tauri command layer (commands.rs) is owned by the app agent.
mod commands;
mod highlight;
mod model;
mod render;
mod theme;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            commands::highlight,
            commands::detect_language,
            commands::format_code,
            commands::export_png,
            commands::export_svg,
            commands::copy_png,
        ])
        .run(tauri::generate_context!())
        .expect("error while running rayCopy");
}
