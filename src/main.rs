#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use tauri::{window::Color, Manager, Theme, WebviewWindow, WindowEvent};

// Match --background in dist/index.html so the strip exposed while the
// webview catches up with a resize is the same colour as the page.
fn paint(window: &WebviewWindow, theme: Theme) {
    let color = match theme {
        Theme::Dark => Color(0x14, 0x14, 0x13, 0xff),
        _ => Color(0xea, 0xea, 0xe8, 0xff),
    };
    let _ = window.set_background_color(Some(color));
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            paint(&window, window.theme().unwrap_or(Theme::Light));
            let w = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::ThemeChanged(theme) = event {
                    paint(&w, *theme);
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
