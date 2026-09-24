# Mantra

A desktop app that has you type a short mantra, then sit with it for a minute before the next one.

![demo](docs/demo.gif)

- Type the mantra shown. Case is ignored; a wrong key shakes the line and you retry.
- When you finish, a 60-second rest timer runs, then a new mantra appears.

Mantras and rest length are at the top of the script in `dist/index.html` (`MANTRAS`, `REST_SECONDS`).

## Build

Needs Rust and the [Tauri v2 prerequisites](https://v2.tauri.app/start/prerequisites/).

```sh
cargo run            # or: just run
cargo tauri build    # installers; needs: cargo install tauri-cli --version "^2" --locked
```

`just --list` shows the other tasks.

## Layout

- `dist/index.html`: all UI and logic (plain HTML/CSS/JS, no npm)
- `src/main.rs`: opens the window
- `icons/icon.svg`: source icon; `just icons` regenerates the rest
- `scripts/drive.py`: headless WebDriver harness (see `AGENTS.md`)
- `scripts/demo.py`: records `docs/demo.gif` headlessly (`just demo`, needs ffmpeg)
