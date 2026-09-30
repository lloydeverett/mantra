//! While Waiting: what the screens do from the mantra's appearing until a
//! session starts. The policy is one of:
//!
//! - None: nothing.
//! - Dim: the screen dimming (dim.rs) follows the page's schedule, which
//!   darkens the screens until the mantra's typed, and again as each
//!   session nears its end.
//! - Hide: every other app's windows are hidden (specialfx leaves system UI
//!   alone) until the session starts.
//!
//! Picked from the While Waiting menu: in the app menu and the dock menu on
//! macOS, and in the ⋮ menu (menu.rs) where that's shown. A snooze (snooze.rs)
//! holds off either.
//!
//! Every launch starts on Dim.

use std::sync::{Arc, Mutex};

use specialfx::HideOthersOptions;
use tauri::menu::{CheckMenuItem, Submenu};
use tauri::{AppHandle, Manager, Runtime};

use crate::dim::Dimmer;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Policy {
    None,
    #[default]
    Dim,
    Hide,
}

const ALL: [Policy; 3] = [Policy::None, Policy::Dim, Policy::Hide];

/// Seconds the dimming takes to fade out or in as the policy changes.
const FADE: f32 = 1.0;

/// The submenu's ID. Its items' are "policy:<name>".
const SUBMENU_ID: &str = "policy";

#[cfg(target_os = "macos")]
const TITLE: &str = "While Waiting";
#[cfg(not(target_os = "macos"))]
const TITLE: &str = "While waiting";

impl Policy {
    fn id(self) -> &'static str {
        match self {
            Policy::None => "policy:none",
            Policy::Dim => "policy:dim",
            Policy::Hide => "policy:hide",
        }
    }

    fn from_id(id: &str) -> Option<Policy> {
        ALL.into_iter().find(|policy| policy.id() == id)
    }

    // macOS hides whole apps, as Cmd-H does; Windows minimizes windows.
    fn label(self) -> &'static str {
        match self {
            Policy::None => "None",
            Policy::Dim if cfg!(target_os = "macos") => "Dim Screens",
            Policy::Dim => "Dim screens",
            Policy::Hide if cfg!(target_os = "macos") => "Hide Other Apps",
            Policy::Hide => "Hide other windows",
        }
    }
}

/// What decides whether other apps are hidden.
#[derive(Default)]
struct Hiding {
    policy: Policy,
    /// From the mantra's appearing until a session starts, as the page says.
    waiting: bool,
    snoozed: bool,
}

impl Hiding {
    fn hide(&self) -> bool {
        self.policy == Policy::Hide && self.waiting && !self.snoozed
    }
}

/// Call from `setup`, after dim::init and pin::init (which sets the app menu),
/// and before snooze::init, so this menu comes first.
pub fn init(app: &AppHandle) -> tauri::Result<()> {
    app.manage(Mutex::new(Hiding::default()));
    platform::init(app)?;
    app.on_menu_event(|app, event| {
        if let Some(policy) = Policy::from_id(event.id().as_ref()) {
            set(app, policy);
        }
    });
    Ok(())
}

/// The While Waiting submenu, for the app menu and the ⋮ menu.
pub fn submenu<R: Runtime, M: Manager<R>>(manager: &M) -> tauri::Result<Submenu<R>> {
    let current = manager.state::<Mutex<Hiding>>().lock().unwrap().policy;
    let submenu = Submenu::with_id(manager, SUBMENU_ID, TITLE, true)?;
    for policy in ALL {
        submenu.append(&CheckMenuItem::with_id(manager, policy.id(), policy.label(), true, policy == current, None::<&str>)?)?;
    }
    Ok(submenu)
}

fn set(app: &AppHandle, policy: Policy) {
    update(app, |hiding| hiding.policy = policy);
    app.state::<Arc<Dimmer>>().policy(vec![(FADE, if policy == Policy::Dim { 1.0 } else { 0.0 })]);
    // A clicked item toggles its own check, even when it was already checked.
    platform::check(app, policy);
}

/// Tells hiding whether a snooze is running. For snooze.rs.
pub fn snoozed(app: &AppHandle, snoozed: bool) {
    update(app, |hiding| hiding.snoozed = snoozed);
}

fn update(app: &AppHandle, change: impl FnOnce(&mut Hiding)) {
    let state = app.state::<Mutex<Hiding>>();
    let mut hiding = state.lock().unwrap();
    change(&mut hiding);
    // Both are no-ops when there's no change to make.
    let result = if hiding.hide() { specialfx::hide_others(&HideOthersOptions::default()) } else { specialfx::show_others() };
    if let Err(e) = result {
        eprintln!("window hiding unavailable: {e}");
    }
}

/// Brings back any hidden windows. Call as the app exits.
pub fn exit() {
    let _ = specialfx::show_others();
}

/// Whether the round is waiting: from the mantra's appearing until a session starts.
#[tauri::command]
pub fn waiting(app: AppHandle, waiting: bool) {
    update(&app, |hiding| hiding.waiting = waiting);
}

#[cfg(target_os = "macos")]
mod platform {
    use std::cell::RefCell;

    use tauri::menu::{MenuItemKind, WINDOW_SUBMENU_ID};
    use tauri::AppHandle;

    use super::{Policy, ALL, SUBMENU_ID, TITLE};

    thread_local! {
        // The dock menu's items. muda's menus live on the main thread.
        static DOCK: RefCell<Vec<(Policy, muda::CheckMenuItem)>> = const { RefCell::new(Vec::new()) };
    }

    pub fn init(app: &AppHandle) -> tauri::Result<()> {
        // In the app menu between View and Window, where app menus go.
        if let Some(menu) = app.menu() {
            let items = menu.items()?;
            let at = items.iter().position(|item| item.id() == WINDOW_SUBMENU_ID).unwrap_or(items.len());
            menu.insert(&super::submenu(app)?, at)?;
        }
        let submenu = muda::Submenu::with_id(SUBMENU_ID, TITLE, true);
        for policy in ALL {
            let item = muda::CheckMenuItem::with_id(policy.id(), policy.label(), true, policy == Policy::default(), None);
            submenu.append(&item)?;
            DOCK.with_borrow_mut(|dock| dock.push((policy, item)));
        }
        crate::dock::append(&submenu)
    }

    pub fn check(app: &AppHandle, current: Policy) {
        if let Some(MenuItemKind::Submenu(submenu)) = app.menu().and_then(|menu| menu.get(SUBMENU_ID)) {
            for policy in ALL {
                if let Some(MenuItemKind::Check(item)) = submenu.get(policy.id()) {
                    let _ = item.set_checked(policy == current);
                }
            }
        }
        let _ = app.run_on_main_thread(move || {
            DOCK.with_borrow(|dock| {
                for (policy, item) in dock {
                    item.set_checked(*policy == current);
                }
            })
        });
    }
}

// Only the ⋮ menu has it, and that's built afresh each time.
#[cfg(not(target_os = "macos"))]
mod platform {
    use tauri::AppHandle;

    use super::Policy;

    pub fn init(_app: &AppHandle) -> tauri::Result<()> {
        Ok(())
    }

    pub fn check(_app: &AppHandle, _policy: Policy) {}
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn menu_ids_round_trip() {
        for policy in ALL {
            assert_eq!(Policy::from_id(policy.id()), Some(policy));
        }
    }

    #[test]
    fn other_ids_do_nothing() {
        for other in ["pin", "policy", "policy:", "policy:x", "snooze:5", "dim"] {
            assert_eq!(Policy::from_id(other), None, "{other}");
        }
    }

    #[test]
    fn starts_dimming_without_hiding() {
        let hiding = Hiding { waiting: true, ..Hiding::default() };
        assert_eq!(hiding.policy, Policy::Dim);
        assert!(!hiding.hide());
    }

    #[test]
    fn hides_while_waiting_under_hide() {
        assert!(Hiding { policy: Policy::Hide, waiting: true, snoozed: false }.hide());
    }

    #[test]
    fn shows_during_a_session_or_snooze() {
        assert!(!Hiding { policy: Policy::Hide, waiting: false, snoozed: false }.hide());
        assert!(!Hiding { policy: Policy::Hide, waiting: true, snoozed: true }.hide());
    }

    #[test]
    fn only_hide_hides() {
        assert!(!Hiding { policy: Policy::None, waiting: true, snoozed: false }.hide());
        assert!(!Hiding { policy: Policy::Dim, waiting: true, snoozed: false }.hide());
    }
}
