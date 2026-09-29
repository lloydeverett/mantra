//! Screen dimming: a black specialfx overlay across every monitor, whose
//! opacity follows a schedule set from the page with the `dim` command.
//!
//! The fade runs here rather than in the page because the webview may throttle
//! its frames while it's in the background, which is exactly when the break
//! is running. It keeps wall-clock time, like the page's session timer, so the
//! two stay in step across sleep (a macOS `Instant` stops while asleep).

use std::sync::{Arc, Condvar, Mutex};
use std::time::{Duration, SystemTime};

use specialfx::{Color, Overlay, OverlayOptions};
use tauri::{AppHandle, Manager, State};

const FRAME: Duration = Duration::from_millis(16);

/// Piecewise-linear opacity: from `from` at `start` through each
/// `(seconds after start, opacity)` key in turn, then holding the last.
struct Fade {
    from: f32,
    start: SystemTime,
    keys: Vec<(f32, f32)>,
}

impl Fade {
    /// Seconds since `start`, or 0 if the clock has since been set back.
    fn elapsed_at(&self, now: SystemTime) -> f32 {
        now.duration_since(self.start).unwrap_or_default().as_secs_f32()
    }

    fn alpha_at(&self, now: SystemTime) -> f32 {
        let t = self.elapsed_at(now);
        let (mut t0, mut a0) = (0.0, self.from);
        for &(t1, a1) in &self.keys {
            if t < t1 {
                return a0 + (a1 - a0) * (t - t0) / (t1 - t0);
            }
            (t0, a0) = (t1, a1);
        }
        a0
    }

    fn done_at(&self, now: SystemTime) -> bool {
        let t = self.elapsed_at(now);
        self.keys.last().is_none_or(|&(end, _)| t >= end)
    }
}

pub struct Dimmer {
    fade: Mutex<Fade>,
    changed: Condvar,
}

/// Creates the (transparent) overlay and the thread that animates it.
/// Call from `setup`: specialfx needs the overlay created on the main thread.
pub fn init(app: &AppHandle) {
    let dimmer = Arc::new(Dimmer {
        fade: Mutex::new(Fade { from: 0.0, start: SystemTime::now(), keys: Vec::new() }),
        changed: Condvar::new(),
    });
    app.manage(dimmer.clone());
    match Overlay::new(OverlayOptions { color: Color::TRANSPARENT, ..Default::default() }) {
        Ok(overlay) => {
            std::thread::spawn(move || animate(overlay, dimmer));
        }
        // No backend on Linux yet; `dim` then does nothing.
        Err(e) => eprintln!("screen dimming unavailable: {e}"),
    }
}

/// Replaces the schedule, starting from the current opacity.
/// `keys` is `[[seconds from now, opacity], ...]`.
#[tauri::command]
pub fn dim(keys: Vec<(f32, f32)>, dimmer: State<Arc<Dimmer>>) {
    let now = SystemTime::now();
    let mut fade = dimmer.fade.lock().unwrap();
    *fade = Fade { from: fade.alpha_at(now), start: now, keys };
    dimmer.changed.notify_one();
}

fn animate(mut overlay: Overlay, dimmer: Arc<Dimmer>) {
    let mut sent = None;
    let mut fade = dimmer.fade.lock().unwrap();
    loop {
        let now = SystemTime::now();
        // The overlay only has 8-bit alpha, so only send actual changes.
        let a = (fade.alpha_at(now).clamp(0.0, 1.0) * 255.0).round() as u8;
        if sent != Some(a) {
            sent = Some(a);
            let _ = overlay.set_color(Color::from_rgba8(0, 0, 0, a));
        }
        fade = if fade.done_at(now) {
            dimmer.changed.wait(fade).unwrap()
        } else {
            dimmer.changed.wait_timeout(fade, FRAME).unwrap().0
        };
    }
}
