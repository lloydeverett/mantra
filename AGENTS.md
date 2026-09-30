# Mantra

**NB: Please ignore markdown files at the root like `TODO.md` unless you are specifically asked to look at them.**

Tauri v2 app. The UI is in `dist/`: styles in `index.html`, light-DOM [Lit](https://lit.dev) components in ES modules (`app.js` runs the round; `typing.js`, `prompt.js`, `session.js` are the views), and the pure parsers: durations in `commitment.js`, tags in `tags.js`. No npm and no build step: Lit is vendored in `dist/vendor/`. Rust in `src/main.rs` opens the window; `src/dim.rs` darkens the screens with a [specialfx](https://github.com/lloydeverett/specialfx) overlay (Windows/macOS only; a no-op on Linux, so the headless driver won't show it). `src/pin.rs` is the Always on Top toggle, and `src/menu.rs` pops up the ⋮ menu at the top right that holds it: a native context menu, hidden on macOS unless `MANTRA_SHOW_MENU=1` (`=0` hides it anywhere).

- Run: `cargo run` (or `just run`; `just --list` for other tasks)
- Test: `just test` runs the parser tests with `node --test`, then `cargo test`.
- Vocabulary (round, commitment, session, ...) is in `CONTEXT.md`.
- Icons: edit `icons/icon.svg`, then `just icons` to regenerate every size. Needs `cargo install tauri-cli --version "^2" --locked`.
- Ask before adding dependencies (crates, packages, CDN scripts).

## Feedback loop

`scripts/drive.py` builds the app, runs it headless (Xvfb + tauri-driver), and drives it over WebDriver. Stdlib only. Linux only, as tauri-driver has no macOS support; its docstring lists the packages.

```python
import sys; sys.path.insert(0, "scripts"); import drive
with drive.session() as app:   # headless=False shows it on the current display
    m = app.js("return document.querySelector('mantra-app').mantra")
    app.keys(m)
    app.keys("Wash up 20m" + drive.ENTER)  # the commitment prompt appears 0.9s after the mantra
    app.shot("target/shot.png")  # then view the PNG
```

On macOS, drive inside the `mantra` incus container instead. It holds a plain copy of the repo (no git) at `/home/ubuntu/mantra`, run as `ubuntu`. Sync the working tree in, run there, pull images back:

```sh
git ls-files -co --exclude-standard | COPYFILE_DISABLE=1 tar c --no-xattrs -T - | incus exec mantra -- su - ubuntu -c 'tar x -C ~/mantra'
incus file push check.py mantra/tmp/check.py   # an ad hoc driver script
incus exec mantra -- su - ubuntu -c 'cd ~/mantra && python3 /tmp/check.py'
incus file pull mantra/home/ubuntu/mantra/target/shot.png target/
```

- `python3 scripts/drive.py`: smoke screenshot to `target/shot.png`. Driver log: `target/drive.log`.
- CSS transitions take about 0.4s. Sleep before a screenshot, or it catches them mid-fade.
- Keys: `drive.ENTER`, `drive.TAB`, `drive.ESCAPE`, `drive.BACKSPACE`, `drive.UP`, `drive.DOWN`.
- Skip the session with `app.js("document.querySelector('session-timer').endIn(300)")`.
