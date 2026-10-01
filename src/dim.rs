//! Screen dimming: a black specialfx overlay across every monitor, whose
//! opacity follows a schedule set from the page with the `dim` command,
//! scaled by a mask that a snooze (snooze.rs) fades out and back in, and by
//! another that fades out when the While Waiting policy (policy.rs) doesn't dim.
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

    /// Replaces the schedule, starting from the opacity at `now`.
    fn retarget(&mut self, now: SystemTime, keys: Vec<(f32, f32)>) {
        *self = Fade { from: self.alpha_at(now), start: now, keys };
    }
}

/// The overlay's opacity: the page's schedule, times the snooze's mask and the policy's.
struct Screen {
    fade: Fade,
    snooze: Fade,
    policy: Fade,
}

impl Screen {
    fn alpha_at(&self, now: SystemTime) -> f32 {
        self.fade.alpha_at(now) * self.snooze.alpha_at(now) * self.policy.alpha_at(now)
    }

    fn done_at(&self, now: SystemTime) -> bool {
        self.fade.done_at(now) && self.snooze.done_at(now) && self.policy.done_at(now)
    }
}

pub struct Dimmer {
    screen: Mutex<Screen>,
    changed: Condvar,
}

impl Dimmer {
    /// Replaces the snooze's mask's schedule, as `dim` does the page's.
    pub fn snooze(&self, keys: Vec<(f32, f32)>) {
        self.screen.lock().unwrap().snooze.retarget(SystemTime::now(), keys);
        self.changed.notify_one();
    }

    /// Replaces the policy's mask's schedule, likewise.
    pub fn policy(&self, keys: Vec<(f32, f32)>) {
        self.screen.lock().unwrap().policy.retarget(SystemTime::now(), keys);
        self.changed.notify_one();
    }
}

/// Creates the (transparent) overlay and the thread that animates it.
/// Call from `setup`: specialfx needs the overlay created on the main thread.
pub fn init(app: &AppHandle) {
    let dimmer = Arc::new(Dimmer {
        screen: Mutex::new(Screen {
            fade: Fade { from: 0.0, start: SystemTime::now(), keys: Vec::new() },
            snooze: Fade { from: 1.0, start: SystemTime::now(), keys: Vec::new() },
            policy: Fade { from: 1.0, start: SystemTime::now(), keys: Vec::new() },
        }),
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
    dimmer.screen.lock().unwrap().fade.retarget(SystemTime::now(), keys);
    dimmer.changed.notify_one();
}

fn animate(mut overlay: Overlay, dimmer: Arc<Dimmer>) {
    let mut sent = None;
    let mut screen = dimmer.screen.lock().unwrap();
    loop {
        let now = SystemTime::now();
        // The overlay only has 8-bit alpha, so only send actual changes.
        let a = (screen.alpha_at(now).clamp(0.0, 1.0) * 255.0).round() as u8;
        if sent != Some(a) {
            sent = Some(a);
            let _ = overlay.set_color(Color::from_rgba8(0, 0, 0, a));
        }
        screen = if screen.done_at(now) {
            dimmer.changed.wait(screen).unwrap()
        } else {
            dimmer.changed.wait_timeout(screen, FRAME).unwrap().0
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(start: SystemTime, secs: f32) -> SystemTime {
        start + Duration::from_secs_f32(secs)
    }

    fn fade(start: SystemTime, from: f32, keys: &[(f32, f32)]) -> Fade {
        Fade { from, start, keys: keys.to_vec() }
    }

    #[test]
    fn snooze_scales_the_schedule() {
        let t0 = SystemTime::now();
        let screen = Screen {
            fade: fade(t0, 0.5, &[]),
            snooze: fade(t0, 1.0, &[(10.0, 0.0), (20.0, 0.0), (30.0, 1.0)]),
            policy: fade(t0, 1.0, &[]),
        };
        assert_eq!(screen.alpha_at(t0), 0.5);
        assert_eq!(screen.alpha_at(at(t0, 5.0)), 0.25);
        assert_eq!(screen.alpha_at(at(t0, 15.0)), 0.0);
        assert_eq!(screen.alpha_at(at(t0, 40.0)), 0.5);
    }

    #[test]
    fn done_only_when_all_are() {
        let t0 = SystemTime::now();
        let screen = Screen {
            fade: fade(t0, 0.0, &[(5.0, 0.5)]),
            snooze: fade(t0, 1.0, &[(10.0, 0.0)]),
            policy: fade(t0, 1.0, &[(15.0, 0.0)]),
        };
        assert!(!screen.done_at(at(t0, 7.0)));
        assert!(!screen.done_at(at(t0, 11.0)));
        assert!(screen.done_at(at(t0, 16.0)));
    }

    #[test]
    fn policy_scales_it_too() {
        let t0 = SystemTime::now();
        let screen = Screen {
            fade: fade(t0, 0.5, &[]),
            snooze: fade(t0, 0.5, &[]),
            policy: fade(t0, 1.0, &[(10.0, 0.0)]),
        };
        assert_eq!(screen.alpha_at(t0), 0.25);
        assert_eq!(screen.alpha_at(at(t0, 5.0)), 0.125);
        assert_eq!(screen.alpha_at(at(t0, 10.0)), 0.0);
    }
}
