//! "Always on top": a checked item in the Window menu on macOS and in the
//! window's system menu on Windows (title bar right-click, Alt+Space, or
//! Shift+right-click on the taskbar button). Nothing on Linux, where Wayland
//! ignores it anyway.
//!
//! Every launch starts unpinned. TODO: it should persist across launches.

use tauri::WebviewWindow;

#[cfg(target_os = "macos")]
pub fn init(window: &WebviewWindow) -> tauri::Result<()> {
    use tauri::Manager;
    use tauri::menu::{CheckMenuItem, Menu, MenuItemKind, PredefinedMenuItem, WINDOW_SUBMENU_ID};

    // Setting a menu replaces Tauri's default one, so start from that.
    let app = window.app_handle();
    let menu = Menu::default(app)?;
    let item = CheckMenuItem::with_id(app, "pin", "Always on Top", true, false, None::<&str>)?;
    if let Some(MenuItemKind::Submenu(window_menu)) = menu.get(WINDOW_SUBMENU_ID) {
        // Minimize, Zoom, [separator, Always on Top,] separator, Close Window
        window_menu.insert_items(&[&PredefinedMenuItem::separator(app)?, &item], 2)?;
    }
    app.set_menu(menu)?;
    let window = window.clone();
    // The item has already toggled its own check by the time the event arrives.
    app.on_menu_event(move |_, event| {
        if event.id() == "pin" {
            let _ = window.set_always_on_top(item.is_checked().unwrap_or(false));
        }
    });
    Ok(())
}

#[cfg(windows)]
pub fn init(window: &WebviewWindow) -> tauri::Result<()> {
    use windows::core::w;
    use windows::Win32::UI::Shell::SetWindowSubclass;
    use windows::Win32::UI::WindowsAndMessaging::{
        GetMenuItemCount, GetMenuItemID, GetSystemMenu, InsertMenuW, MF_BYPOSITION, MF_SEPARATOR,
        MF_STRING, SC_CLOSE,
    };

    let hwnd = window.hwnd()?;
    unsafe {
        let menu = GetSystemMenu(hwnd, false);
        // Restore ... Maximize, [separator, Always on top,] separator, Close
        let count = GetMenuItemCount(Some(menu));
        let close = (0..count).find(|&i| GetMenuItemID(menu, i) == SC_CLOSE).unwrap_or(count);
        let at = close.saturating_sub(1) as u32;
        let _ = InsertMenuW(menu, at, MF_BYPOSITION | MF_SEPARATOR, 0, None);
        let _ = InsertMenuW(menu, at + 1, MF_BYPOSITION | MF_STRING, windows_menu::ID, w!("Always on top"));
        // The window is leaked into the subclass for the life of the app.
        let data = Box::into_raw(Box::new(window.clone())) as usize;
        let _ = SetWindowSubclass(hwnd, Some(windows_menu::proc), 1, data);
    }
    Ok(())
}

#[cfg(windows)]
mod windows_menu {
    use tauri::WebviewWindow;
    use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
    use windows::Win32::UI::Shell::DefSubclassProc;
    use windows::Win32::UI::WindowsAndMessaging::{
        CheckMenuItem, GetMenuState, GetSystemMenu, MF_BYCOMMAND, MF_CHECKED, MF_UNCHECKED,
        WM_SYSCOMMAND,
    };

    // System command IDs must be below 0xF000 with the low four bits clear,
    // which Windows uses internally.
    pub const ID: usize = 0x0010;

    pub unsafe extern "system" fn proc(
        hwnd: HWND,
        msg: u32,
        wparam: WPARAM,
        lparam: LPARAM,
        _id: usize,
        data: usize,
    ) -> LRESULT {
        if msg == WM_SYSCOMMAND && wparam.0 & 0xFFF0 == ID {
            let window = &*(data as *const WebviewWindow);
            let menu = GetSystemMenu(hwnd, false);
            let pinned = GetMenuState(menu, ID as u32, MF_BYCOMMAND) & MF_CHECKED.0 == 0;
            CheckMenuItem(menu, ID as u32, (MF_BYCOMMAND | if pinned { MF_CHECKED } else { MF_UNCHECKED }).0);
            let _ = window.set_always_on_top(pinned);
            return LRESULT(0);
        }
        DefSubclassProc(hwnd, msg, wparam, lparam)
    }
}

#[cfg(not(any(target_os = "macos", windows)))]
pub fn init(_window: &WebviewWindow) -> tauri::Result<()> {
    Ok(())
}
