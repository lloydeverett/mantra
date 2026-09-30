//! The ⋮ button's menu in the window's top right corner: a native context
//! menu, popped up where the page asks, with Always on Top, then Snooze. Off
//! by default on macOS, where the app menu already has everything and the
//! window looks cleaner without it.
//! MANTRA_SHOW_MENU=1 or =0 forces it on or off on any platform.

use tauri::menu::{CheckMenuItem, Menu, PredefinedMenuItem};
use tauri::{LogicalPosition, WebviewWindow};

use crate::{pin, snooze};

/// Whether the page shows the ⋮ button, given MANTRA_SHOW_MENU.
pub fn shown(var: Option<&str>) -> bool {
    match var.map(str::to_ascii_lowercase).as_deref() {
        None => !cfg!(target_os = "macos"),
        Some("" | "0" | "false" | "no" | "off") => false,
        Some(_) => true,
    }
}

/// Pops the menu up with its corner at (x, y), in CSS pixels from the page's
/// top left. Built afresh each time, so it shows the window's current state.
/// Its items' events go to the app's menu handlers, like the app menu's.
/// Async so it runs off the main thread, which popping up waits on.
/// Returns once the menu closes, when the window takes keyboard focus back
/// (WebView2 may not take it back by itself), so typing carries on.
#[tauri::command]
pub async fn menu(window: WebviewWindow, x: f64, y: f64) -> tauri::Result<()> {
    let pinned = window.is_always_on_top()?;
    let item = CheckMenuItem::with_id(&window, pin::ID, pin::LABEL, true, pinned, None::<&str>)?;
    let separator = PredefinedMenuItem::separator(&window)?;
    let menu = Menu::with_items(&window, &[&item, &separator, &snooze::submenu(&window)?])?;
    window.popup_menu_at(&menu, LogicalPosition::new(x, y))?;
    window.set_focus()
}

#[cfg(test)]
mod tests {
    use super::shown;

    #[test]
    fn defaults_to_off_on_macos_only() {
        assert_eq!(shown(None), !cfg!(target_os = "macos"));
    }

    #[test]
    fn variable_forces_it_on_or_off() {
        for on in ["1", "true", "TRUE", "yes"] {
            assert!(shown(Some(on)), "{on}");
        }
        for off in ["0", "false", "False", "no", ""] {
            assert!(!shown(Some(off)), "{off}");
        }
    }
}
