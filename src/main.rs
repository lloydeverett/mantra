#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod dim;
mod menu;
mod pin;

use tauri::{webview::PageLoadEvent, window::Color, Manager, Theme, WebviewWindow, WindowEvent};

// Match --background in dist/index.html so the strip exposed while the
// webview catches up with a resize is the same colour as the page. On macOS
// the webview paints its own default background over the window's unless
// the macos-private-api feature lets this turn that off.
fn paint(window: &WebviewWindow, theme: Theme) {
    let color = match theme {
        Theme::Dark => Color(0x14, 0x14, 0x13, 0xff),
        _ => Color(0xea, 0xea, 0xe8, 0xff),
    };
    let _ = window.set_background_color(Some(color));
}

fn main() {
    // Tell the page whether this is a release build, so it can pick the full break,
    // and whether to show the ⋮ menu (see src/menu.rs).
    let show_menu = menu::shown(std::env::var("MANTRA_SHOW_MENU").ok().as_deref());
    let build = tauri::plugin::Builder::<tauri::Wry>::new("build")
        .js_init_script(format!(
            "window.MANTRA_RELEASE = {}; window.MANTRA_MENU = {show_menu};",
            !cfg!(debug_assertions)
        ))
        .build();
    tauri::Builder::default()
        .plugin(build)
        .setup(|app| {
            dim::init(app.handle());
            let window = app.get_webview_window("main").unwrap();
            paint(&window, window.theme().unwrap_or(Theme::Light));
            pin::init(&window)?;
            let w = window.clone();
            window.on_window_event(move |event| {
                if let WindowEvent::ThemeChanged(theme) = event {
                    paint(&w, *theme);
                }
            });
            Ok(())
        })
        // The window starts hidden (tauri.conf.json) so it doesn't sit empty while
        // the webview starts up and loads the page. Show it once there's a page to see,
        // and focus it: on macOS showing it late doesn't bring the app to the front.
        .on_page_load(|webview, payload| {
            if payload.event() == PageLoadEvent::Finished {
                let window = webview.window();
                let _ = window.show();
                let _ = window.set_focus();
            }
        })
        .invoke_handler(tauri::generate_handler![dim::dim, menu::menu])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
