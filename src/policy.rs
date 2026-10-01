//! While Waiting: what the screens do from the mantra's appearing until a
//! session starts. The policy is one of:
//!
//! - None: nothing.
//! - Dim: the screen dimming (dim.rs) follows the page's schedule, which
//!   darkens the screens until the mantra's typed, and again as each
//!   session nears its end.
//! - Hide: every other app's windows are hidden (specialfx leaves system UI
//!   alone, and on macOS Stickies stays too) until the session starts.
//! - Dim then Hide: as Dim, and once the round has waited 5 minutes, as Hide
//!   too until the session starts.
//!
//! Picked from the Effects menu: in the app menu and the dock menu on
//! macOS, and in the ⋮ menu (menu.rs) where that's shown. A snooze (snooze.rs)
//! holds off either.
//!
//! Every launch starts on Dim.

use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

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
    DimThenHide,
}

const ALL: [Policy; 4] = [Policy::None, Policy::Dim, Policy::Hide, Policy::DimThenHide];

/// How long Dim then Hide waits before it hides. Its labels say so too.
const HIDE_AFTER: Duration = Duration::from_secs(5 * 60);

/// Seconds the dimming takes to fade out or in as the policy changes.
const FADE: f32 = 1.0;

/// The submenu's ID. Its items' are "policy:<name>".
const SUBMENU_ID: &str = "policy";

const TITLE: &str = "Effects";

/// Apps that Hide leaves alone, beyond specialfx's own exemptions.
const EXEMPT: &[&str] = if cfg!(target_os = "macos") { &["com.apple.Stickies"] } else { &[] };

impl Policy {
    fn id(self) -> &'static str {
        match self {
            Policy::None => "policy:none",
            Policy::Dim => "policy:dim",
            Policy::Hide => "policy:hide",
            Policy::DimThenHide => "policy:dim-then-hide",
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
            Policy::DimThenHide if cfg!(target_os = "macos") => "Dim for 5m \u{2192} Hide Other Apps",
            Policy::DimThenHide => "Dim for 5m \u{2192} hide other windows",
        }
    }

    fn dims(self) -> bool {
        matches!(self, Policy::Dim | Policy::DimThenHide)
    }
}

/// What decides whether other apps are hidden.
#[derive(Default)]
struct Hiding {
    policy: Policy,
    /// Since when the round has been waiting: from the mantra's appearing
    /// until a session starts, as the page says.
    waiting: Option<SystemTime>,
    snoozed: bool,
}

impl Hiding {
    fn hide_at(&self, now: SystemTime) -> bool {
        let Some(since) = self.waiting else { return false };
        let waited = now.duration_since(since).unwrap_or_default();
        let hides = match self.policy {
            Policy::None | Policy::Dim => false,
            Policy::Hide => true,
            Policy::DimThenHide => waited >= HIDE_AFTER,
        };
        hides && !self.snoozed
    }

    /// The page says waiting (again, if a prompt is cancelled) or not.
    fn wait(&mut self, waiting: bool, now: SystemTime) {
        self.waiting = if waiting { self.waiting.or(Some(now)) } else { None };
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

/// The Effects submenu, for the app menu and the ⋮ menu.
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
    app.state::<Arc<Dimmer>>().policy(vec![(FADE, if policy.dims() { 1.0 } else { 0.0 })]);
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
    let result = if hiding.hide_at(SystemTime::now()) {
        let exempt = EXEMPT.iter().map(|&app| app.into()).collect();
        specialfx::hide_others(&HideOthersOptions { exempt, ..Default::default() })
    } else {
        specialfx::show_others()
    };
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
    let mut started = None;
    update(&app, |hiding| {
        let before = hiding.waiting;
        hiding.wait(waiting, SystemTime::now());
        started = hiding.waiting.filter(|_| before.is_none());
    });
    if let Some(since) = started {
        recheck_after(app, since);
    }
}

/// Brings hiding up to date once this wait has gone on for HIDE_AFTER, for
/// Dim then Hide (and in case it's picked meanwhile). Polled against the wall
/// clock, as sleep()'s clock stops while a Mac sleeps.
fn recheck_after(app: AppHandle, since: SystemTime) {
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_secs(1));
        if app.state::<Mutex<Hiding>>().lock().unwrap().waiting != Some(since) {
            return; // the session started
        }
        if SystemTime::now().duration_since(since).unwrap_or_default() >= HIDE_AFTER {
            // On the main thread, like the menus' and the page's calls.
            let handle = app.clone();
            let _ = app.run_on_main_thread(move || update(&handle, |_| {}));
            return;
        }
    });
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
    use std::time::{Duration, UNIX_EPOCH};

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

    fn at(secs: u64) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(secs)
    }

    fn hiding(policy: Policy, waiting: Option<u64>, snoozed: bool) -> Hiding {
        Hiding { policy, waiting: waiting.map(at), snoozed }
    }

    #[test]
    fn starts_dimming_without_hiding() {
        let hiding = Hiding { waiting: Some(at(0)), ..Hiding::default() };
        assert_eq!(hiding.policy, Policy::Dim);
        assert!(!hiding.hide_at(at(0)));
    }

    #[test]
    fn hides_while_waiting_under_hide() {
        assert!(hiding(Policy::Hide, Some(0), false).hide_at(at(0)));
    }

    #[test]
    fn shows_during_a_session_or_snooze() {
        assert!(!hiding(Policy::Hide, None, false).hide_at(at(0)));
        assert!(!hiding(Policy::Hide, Some(0), true).hide_at(at(0)));
    }

    #[test]
    fn none_and_dim_never_hide() {
        assert!(!hiding(Policy::None, Some(0), false).hide_at(at(3600)));
        assert!(!hiding(Policy::Dim, Some(0), false).hide_at(at(3600)));
    }

    #[test]
    fn dim_then_hide_hides_once_waiting_long_enough() {
        let hiding = hiding(Policy::DimThenHide, Some(100), false);
        assert!(!hiding.hide_at(at(100)));
        assert!(!hiding.hide_at(at(100 + 299)));
        assert!(hiding.hide_at(at(100 + 300)));
    }

    #[test]
    fn dim_then_hide_shows_during_a_session_or_snooze() {
        assert!(!hiding(Policy::DimThenHide, None, false).hide_at(at(3600)));
        assert!(!hiding(Policy::DimThenHide, Some(0), true).hide_at(at(3600)));
    }

    #[test]
    fn dim_then_hide_dims() {
        assert!(Policy::Dim.dims());
        assert!(Policy::DimThenHide.dims());
        assert!(!Policy::None.dims());
        assert!(!Policy::Hide.dims());
    }

    #[test]
    fn waiting_starts_once_and_ends_with_the_session() {
        let mut hiding = Hiding::default();
        hiding.wait(true, at(10));
        hiding.wait(true, at(20)); // a cancelled prompt: still the same wait
        assert_eq!(hiding.waiting, Some(at(10)));
        hiding.wait(false, at(30));
        assert_eq!(hiding.waiting, None);
        hiding.wait(true, at(40));
        assert_eq!(hiding.waiting, Some(at(40)));
    }
}
