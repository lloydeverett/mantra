//! "Always on top": a checked item in the Window menu on macOS and in the
//! window's system menu on Windows (title bar right-click, Alt+Space, or
//! Shift+right-click on the taskbar button), and in the ⋮ menu (menu.rs)
//! where that's shown. Their checks follow the window, whichever toggles it.
//!
//! Every launch starts unpinned. TODO: it should persist across launches.

use tauri::{Manager, WebviewWindow};

/// The item's menu ID, wherever it is.
pub const ID: &str = "pin";

#[cfg(target_os = "macos")]
pub const LABEL: &str = "Always on Top";
#[cfg(windows)]
pub const LABEL: &str = "Always on top";
// Wayland ignores it.
#[cfg(not(any(target_os = "macos", windows)))]
pub const LABEL: &str = "Always on top (X11 only)";

pub fn init(window: &WebviewWindow) -> tauri::Result<()> {
    platform::init(window)?;
    let w = window.clone();
    window.app_handle().on_menu_event(move |_, event| {
        if event.id() == ID {
            toggle(&w);
        }
    });
    Ok(())
}

fn toggle(window: &WebviewWindow) {
    let pinned = !window.is_always_on_top().unwrap_or(false);
    let _ = window.set_always_on_top(pinned);
    platform::check(window, pinned);
}

#[cfg(target_os = "macos")]
mod platform {
    use super::{ID, LABEL};
    use tauri::menu::{CheckMenuItem, Menu, MenuItemKind, PredefinedMenuItem, WINDOW_SUBMENU_ID};
    use tauri::{Manager, WebviewWindow};

    pub fn init(window: &WebviewWindow) -> tauri::Result<()> {
        // Setting a menu replaces Tauri's default one, so start from that.
        let app = window.app_handle();
        let menu = Menu::default(app)?;
        let item = CheckMenuItem::with_id(app, ID, LABEL, true, false, None::<&str>)?;
        if let Some(MenuItemKind::Submenu(window_menu)) = menu.get(WINDOW_SUBMENU_ID) {
            // Minimize, Zoom, [separator, Always on Top,] separator, Close Window
            window_menu.insert_items(&[&PredefinedMenuItem::separator(app)?, &item], 2)?;
        }
        app.set_menu(menu)?;
        Ok(())
    }

    // The Window menu's item toggles its own check when clicked, but not when the ⋮ menu's is.
    pub fn check(window: &WebviewWindow, pinned: bool) {
        let Some(MenuItemKind::Submenu(window_menu)) =
            window.app_handle().menu().and_then(|menu| menu.get(WINDOW_SUBMENU_ID))
        else {
            return;
        };
        if let Some(MenuItemKind::Check(item)) = window_menu.get(ID) {
            let _ = item.set_checked(pinned);
        }
    }
}

#[cfg(windows)]
mod platform {
    use tauri::WebviewWindow;
    use windows::core::w;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
    use windows::Win32::UI::WindowsAndMessaging::{
        CheckMenuItem, GetMenuItemCount, GetMenuItemID, GetSystemMenu, InsertMenuW, MF_BYCOMMAND,
        MF_BYPOSITION, MF_CHECKED, MF_SEPARATOR, MF_STRING, MF_UNCHECKED, SC_CLOSE, WM_SYSCOMMAND,
    };

    // System command IDs must be below 0xF000 with the low four bits clear,
    // which Windows uses internally.
    const SYSCOMMAND_ID: usize = 0x0010;

    pub fn init(window: &WebviewWindow) -> tauri::Result<()> {
        let hwnd = window.hwnd()?;
        unsafe {
            let menu = GetSystemMenu(hwnd, false);
            // Restore ... Maximize, [separator, Always on top,] separator, Close
            let count = GetMenuItemCount(Some(menu));
            let close = (0..count).find(|&i| GetMenuItemID(menu, i) == SC_CLOSE).unwrap_or(count);
            let at = close.saturating_sub(1) as u32;
            let _ = InsertMenuW(menu, at, MF_BYPOSITION | MF_SEPARATOR, 0, None);
            let _ = InsertMenuW(menu, at + 1, MF_BYPOSITION | MF_STRING, SYSCOMMAND_ID, w!("Always on top"));
            // The window is leaked into the subclass for the life of the app.
            let data = Box::into_raw(Box::new(window.clone())) as usize;
            let _ = SetWindowSubclass(hwnd, Some(proc), 1, data);
        }
        Ok(())
    }

    pub fn check(window: &WebviewWindow, pinned: bool) {
        let Ok(hwnd) = window.hwnd() else { return };
        unsafe {
            let menu = GetSystemMenu(hwnd, false);
            CheckMenuItem(menu, SYSCOMMAND_ID as u32, (MF_BYCOMMAND | if pinned { MF_CHECKED } else { MF_UNCHECKED }).0);
        }
    }

    unsafe extern "system" fn proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        data: usize,
    ) -> LRESULT {
        if msg == WM_SYSCOMMAND && wparam.0 & 0xFFF0 == SYSCOMMAND_ID {
            super::toggle(&*(data as *const WebviewWindow));
            return LRESULT(0);
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
    }
}

// Only the ⋮ menu has the item.
#[cfg(not(any(target_os = "macos", windows)))]
mod platform {
    use tauri::WebviewWindow;

    pub fn init(_window: &WebviewWindow) -> tauri::Result<()> {
        Ok(())
    }

    pub fn check(_window: &WebviewWindow, _pinned: bool) {}
}
