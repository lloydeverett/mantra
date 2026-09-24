# Mantra

Tauri v2 app. All UI and logic are in `dist/index.html` (plain HTML/CSS/JS, no npm). Rust in `src/main.rs` only opens the window.

- Run: `cargo run`
- Ask before adding dependencies (crates, packages, CDN scripts).

## Feedback loop

`scripts/drive.py` builds the app, runs it headless (Xvfb + tauri-driver), and drives it over WebDriver. Stdlib only.

```python
import sys; sys.path.insert(0, "scripts"); import drive
with drive.session() as app:   # headless=False shows it on the WSLg desktop
    m = app.js("return mantra")
    app.keys(m)
    app.shot("target/shot.png")  # then view the PNG
```

- `python3 scripts/drive.py`: smoke screenshot to `target/shot.png`. Driver log: `target/drive.log`.
- CSS transitions take about 0.4s. Sleep before a screenshot, or it catches them mid-fade.
- Skip the rest timer with `app.js("deadline = Date.now() + 300")`.
