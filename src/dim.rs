//! Screen dimming: a black specialfx overlay across every monitor, whose
//! opacity follows a schedule set from the page with the `dim` command.
//!
//! The fade runs here rather than in the page because the webview may throttle
//! its frames while it's in the background, which is exactly when the break
//! is running.

use std::cell::RefCell;
use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, Instant};

use specialfx::{Color, Overlay, OverlayOptions};
use tauri::{AppHandle, Manager, State};

const FRAME: Duration = Duration::from_millis(16);

thread_local! {
    // macOS only lets the overlay be touched on the main thread, so it lives there.
    static OVERLAY: RefCell<Option<Overlay>> = const { RefCell::new(None) };
}

/// Piecewise-linear opacity: from `from` at `start` through each
/// `(seconds after start, opacity)` key in turn, then holding the last.
struct Fade {
    from: f32,
    start: Instant,
    keys: Vec<(f32, f32)>,
}

impl Fade {
    fn alpha_at(&self, now: Instant) -> f32 {
        let t = now.duration_since(self.start).as_secs_f32();
        let (mut t0, mut a0) = (0.0, self.from);
        for &(t1, a1) in &self.keys {
            if t < t1 {
                return a0 + (a1 - a0) * (t - t0) / (t1 - t0);
            }
            (t0, a0) = (t1, a1);
        }
        a0
    }

    fn done_at(&self, now: Instant) -> bool {
        let t = now.duration_since(self.start).as_secs_f32();
        self.keys.last().is_none_or(|&(end, _)| t >= end)
    }
}

pub struct Dimmer {
    fade: Mutex<Fade>,
    changed: Condvar,
}

/// Creates the (transparent) overlay and the thread that animates it.
/// Call from `setup`, which runs on the main thread.
pub fn init(app: &AppHandle) {
    match Overlay::new(OverlayOptions { color: Color::TRANSPARENT, ..Default::default() }) {
        Ok(overlay) => OVERLAY.with(|o| *o.borrow_mut() = Some(overlay)),
        // No backend on Linux yet; `dim` then does nothing.
        Err(e) => eprintln!("screen dimming unavailable: {e}"),
    }
    let dimmer = Arc::new(Dimmer {
        fade: Mutex::new(Fade { from: 0.0, start: Instant::now(), keys: Vec::new() }),
        changed: Condvar::new(),
    });
    app.manage(dimmer.clone());
    let app = app.clone();
    std::thread::spawn(move || animate(app, dimmer));
}

/// Replaces the schedule, starting from the current opacity.
/// `keys` is `[[seconds from now, opacity], ...]`.
#[tauri::command]
pub fn dim(keys: Vec<(f32, f32)>, dimmer: State<Arc<Dimmer>>) {
    let now = Instant::now();
    let mut fade = dimmer.fade.lock().unwrap();
    *fade = Fade { from: fade.alpha_at(now), start: now, keys };
    dimmer.changed.notify_one();
}

fn animate(app: AppHandle, dimmer: Arc<Dimmer>) {
    let mut sent = None;
    let mut fade = dimmer.fade.lock().unwrap();
    loop {
        let now = Instant::now();
        // The overlay only has 8-bit alpha, so only send actual changes.
        let a = (fade.alpha_at(now).clamp(0.0, 1.0) * 255.0).round() as u8;
        if sent != Some(a) {
            sent = Some(a);
            let _ = app.run_on_main_thread(move || {
                OVERLAY.with(|o| {
                    if let Some(overlay) = o.borrow_mut().as_mut() {
                        let _ = overlay.set_color(Color::from_rgba8(0, 0, 0, a));
                    }
                })
            });
        }
        fade = if fade.done_at(now) {
            dimmer.changed.wait(fade).unwrap()
        } else {
            dimmer.changed.wait_timeout(fade, FRAME).unwrap().0
        };
    }
}
