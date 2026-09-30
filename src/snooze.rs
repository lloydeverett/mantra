//! Snooze: holds off the screen dimming (dim.rs) for a while. The round carries
//! on underneath, and when the snooze ends the dimming fades back in to wherever
//! the page's schedule has got to. Started from the Snooze menu: in the app menu
//! and the dock menu on macOS, and in the ⋮ menu (menu.rs) where that's shown.
//! Cancelled from there, or from the countdown the page shows (dist/snooze.js).
//!
//! Every launch starts unsnoozed.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tauri::menu::{MenuItem, PredefinedMenuItem, Submenu};
use tauri::{AppHandle, Manager, Runtime, State};

use crate::dim::Dimmer;

/// The durations on offer, in minutes. Dev builds add a short one to try it with.
#[cfg(debug_assertions)]
const MINUTES: &[u32] = &[1, 5, 10, 20, 30, 45, 60, 120];
#[cfg(not(debug_assertions))]
const MINUTES: &[u32] = &[5, 10, 20, 30, 45, 60, 120];

/// Seconds the dimming takes to fade out as a snooze starts, and back in as it ends.
const FADE: f32 = 1.0;

/// Menu IDs: the submenu, "snooze:<minutes>" for each duration, and Cancel.
const SUBMENU_ID: &str = "snooze";
const PREFIX: &str = "snooze:";
const CANCEL_ID: &str = "snooze:cancel";

const TITLE: &str = "Snooze";
#[cfg(target_os = "macos")]
const CANCEL: &str = "Cancel Snooze";
#[cfg(not(target_os = "macos"))]
const CANCEL: &str = "Cancel snooze";

/// When the snooze runs out, if one is running.
#[derive(Default)]
pub struct Snooze {
    until: Mutex<Option<SystemTime>>,
}

#[derive(Debug, PartialEq)]
enum Action {
    Start(u32),
    Cancel,
}

fn id(minutes: u32) -> String {
    format!("{PREFIX}{minutes}")
}

/// What a menu item does, from its ID.
fn action(id: &str) -> Option<Action> {
    match id.strip_prefix(PREFIX)? {
        "cancel" => Some(Action::Cancel),
        m => m.parse().ok().filter(|m| MINUTES.contains(m)).map(Action::Start),
    }
}

/// "5 minutes", "1 hour"; title case on macOS.
fn label(minutes: u32) -> String {
    let (n, unit) = if minutes % 60 == 0 { (minutes / 60, "hour") } else { (minutes, "minute") };
    let unit = if cfg!(target_os = "macos") { unit[..1].to_uppercase() + &unit[1..] } else { unit.into() };
    format!("{n} {unit}{}", if n == 1 { "" } else { "s" })
}

/// Call from `setup`, after dim::init and pin::init (which sets the app menu).
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(Snooze::default());
    platform::init(app)?;
    app.on_menu_event(|app, event| match action(event.id().as_ref()) {
        Some(Action::Start(minutes)) => start(app, minutes),
        Some(Action::Cancel) => cancel(app),
        None => {}
    });
    Ok(())
}

/// The Snooze submenu, for the app menu and the ⋮ menu.
pub fn submenu<R: Runtime, M: Manager<R>>(manager: &M) -> tauri::Result<Submenu<R>> {
    let snoozed = until_ms(manager.state()).is_some();
    let submenu = Submenu::with_id(manager, SUBMENU_ID, TITLE, true)?;
    for &minutes in MINUTES {
        submenu.append(&MenuItem::with_id(manager, id(minutes), label(minutes), true, None::<&str>)?)?;
    }
    submenu.append(&PredefinedMenuItem::separator(manager)?)?;
    submenu.append(&MenuItem::with_id(manager, CANCEL_ID, CANCEL, snoozed, None::<&str>)?)?;
    Ok(submenu)
}

/// Starts a snooze, or starts it over.
fn start(app: &AppHandle, minutes: u32) {
    let until = SystemTime::now() + Duration::from_secs(minutes as u64 * 60);
    *app.state::<Snooze>().until.lock().unwrap() = Some(until);
    let secs = minutes as f32 * 60.0;
    app.state::<Arc<Dimmer>>().mask(vec![(FADE, 0.0), (secs, 0.0), (secs + FADE, 1.0)]);
    publish(app);
    // The mask fades back in by itself; this tells the menus once it runs out.
    // Polled against the wall clock, as sleep()'s clock stops while a Mac sleeps.
    let app = app.clone();
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        let snooze = app.state::<Snooze>();
        let mut current = snooze.until.lock().unwrap();
        if *current != Some(until) {
            return; // cancelled or started over
        }
        if SystemTime::now() >= until {
            *current = None;
            drop(current);
            // On the main thread, like the menus' and the page's calls, so the
            // page hears of each change in the order they happen.
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || publish(&handle));
            return;
        }
    });
}

fn cancel(app: &AppHandle) {
    if app.state::<Snooze>().until.lock().unwrap().take().is_some() {
        app.state::<Arc<Dimmer>>().mask(vec![(FADE, 1.0)]);
        publish(app);
    }
}

/// When the snooze runs out, in ms since the Unix epoch, or None if none is running.
fn until_ms(snooze: State<Snooze>) -> Option<u64> {
    let until = (*snooze.until.lock().unwrap())?;
    Some(until.duration_since(UNIX_EPOCH).unwrap_or_default().as_millis() as u64)
}

/// Brings the menus' Cancel items and the page's countdown up to date.
fn publish(app: &AppHandle) {
    let ms = until_ms(app.state());
    platform::cancellable(app, ms.is_some());
    if let Some(window) = app.get_webview_window("main") {
        let ms = ms.map_or("null".into(), |ms| ms.to_string());
        let _ = window.eval(format!("document.querySelector('snooze-countdown')?.show({ms})"));
    }
}

/// Snoozes for this many minutes, for scripts (the page's menus are native).
#[tauri::command]
pub fn snooze(app: AppHandle, minutes: u32) {
    start(&app, minutes);
}

#[tauri::command]
pub fn unsnooze(app: AppHandle) {
    cancel(&app);
}

/// When the snooze runs out, for the page when it loads: see `until_ms`.
#[tauri::command]
pub fn snoozed(snooze: State<Snooze>) -> Option<u64> {
    until_ms(snooze)
}

#[cfg(target_os = "macos")]
mod platform {
    use std::cell::RefCell;

    use muda::ContextMenu;
    use objc2::runtime::{AnyObject, Imp, Sel};
    use objc2::{class, msg_send, sel};
    use tauri::menu::{MenuItemKind, WINDOW_SUBMENU_ID};
    use tauri::AppHandle;

    use super::{id, label, CANCEL, CANCEL_ID, MINUTES, SUBMENU_ID, TITLE};

    thread_local! {
        // The dock menu and its Cancel item. muda's menus live on the main thread.
        static DOCK: RefCell<Option<(muda::Menu, muda::MenuItem)>> = const { RefCell::new(None) };
    }

    pub fn init(app: &AppHandle) -> tauri::Result<()> {
        // In the app menu between View and Window, where app menus go.
        if let Some(menu) = app.menu() {
            let items = menu.items()?;
            let at = items.iter().position(|item| item.id() == WINDOW_SUBMENU_ID).unwrap_or(items.len());
            menu.insert(&super::submenu(app)?, at)?;
        }
        dock()
    }

    // Tauri has no dock menu, so this one is built with muda, which Tauri's menus
    // are built on (so its items' events reach on_menu_event all the same), and
    // handed to AppKit by the app delegate's applicationDockMenu:, which tao's
    // delegate doesn't have, so it gets one added.
    fn dock() -> tauri::Result<()> {
        let submenu = muda::Submenu::with_id(SUBMENU_ID, TITLE, true);
        for &minutes in MINUTES {
            submenu.append(&muda::MenuItem::with_id(id(minutes), label(minutes), true, None))?;
        }
        let cancel = muda::MenuItem::with_id(CANCEL_ID, CANCEL, false, None);
        submenu.append_items(&[&muda::PredefinedMenuItem::separator(), &cancel])?;
        let menu = muda::Menu::new();
        menu.append(&submenu)?;
        DOCK.with_borrow_mut(|dock| *dock = Some((menu, cancel)));
        unsafe {
            let app: *mut AnyObject = msg_send![class!(NSApplication), sharedApplication];
            let delegate: *mut AnyObject = msg_send![app, delegate];
            let Some(delegate) = delegate.as_ref() else { return Ok(()) };
            let imp: Imp = std::mem::transmute(
                dock_menu as extern "C-unwind" fn(&AnyObject, Sel, *mut AnyObject) -> *mut AnyObject,
            );
            let class = delegate.class() as *const _ as *mut _;
            if !objc2::ffi::class_addMethod(class, sel!(applicationDockMenu:), imp, c"@@:@".as_ptr()).as_bool() {
                eprintln!("dock menu unavailable: the app delegate already has one");
            }
            // NSApplication may look over its delegate's methods only when it's set.
            let _: () = msg_send![app, setDelegate: delegate];
        }
        Ok(())
    }

    extern "C-unwind" fn dock_menu(_: &AnyObject, _: Sel, _: *mut AnyObject) -> *mut AnyObject {
        DOCK.with_borrow(|dock| dock.as_ref().map_or(std::ptr::null_mut(), |(menu, _)| menu.ns_menu().cast()))
    }

    pub fn cancellable(app: &AppHandle, cancellable: bool) {
        if let Some(MenuItemKind::Submenu(submenu)) = app.menu().and_then(|menu| menu.get(SUBMENU_ID)) {
            if let Some(MenuItemKind::MenuItem(item)) = submenu.get(CANCEL_ID) {
                let _ = item.set_enabled(cancellable);
            }
        }
        let _ = app.run_on_main_thread(move || {
            DOCK.with_borrow(|dock| {
                if let Some((_, cancel)) = dock {
                    cancel.set_enabled(cancellable);
                }
            })
        });
    }
}

// Only the ⋮ menu has it, and that's built afresh each time.
#[cfg(not(target_os = "macos"))]
mod platform {
    use tauri::AppHandle;

    pub fn init(_app: &AppHandle) -> tauri::Result<()> {
        Ok(())
    }

    pub fn cancellable(_app: &AppHandle, _cancellable: bool) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_ids_round_trip() {
        for &minutes in MINUTES {
            assert_eq!(action(&id(minutes)), Some(Action::Start(minutes)));
        }
        assert_eq!(action(CANCEL_ID), Some(Action::Cancel));
    }

    #[test]
    fn other_ids_do_nothing() {
        for other in ["pin", "snooze", "snooze:", "snooze:7", "snooze:x", "5"] {
            assert_eq!(action(other), None, "{other}");
        }
    }

    #[test]
    fn labels_read_as_minutes_or_hours() {
        for (minutes, text) in [(1, "1 minute"), (5, "5 minutes"), (45, "45 minutes"), (60, "1 hour"), (120, "2 hours")] {
            assert_eq!(label(minutes).to_lowercase(), text);
        }
    }

    #[test]
    fn labels_are_title_case_on_macos_only() {
        assert_eq!(label(5), if cfg!(target_os = "macos") { "5 Minutes" } else { "5 minutes" });
    }
}
