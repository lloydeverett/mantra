#!/usr/bin/env python3
"""Record docs/demo.gif: type a mantra in the headless app while ffmpeg grabs the Xvfb display.

Needs drive.py's setup plus apt `ffmpeg`. Run: python3 scripts/demo.py
"""
import subprocess
import time

import drive

RAW = drive.ROOT / "target" / "demo.mkv"
OUT = drive.ROOT / "docs" / "demo.gif"
KEY_MS = 110
VIEW_FADE = 1.2  # 0.9s JS delay before switching views, then the CSS fade

with drive.session() as app:
    r = app._("GET", "/window/rect")
    time.sleep(1)  # let the first view fade in
    rec = subprocess.Popen(
        ["ffmpeg", "-y", "-loglevel", "error", "-f", "x11grab", "-draw_mouse", "0", "-framerate", "30",
         "-video_size", f"{r['width']}x{r['height']}", "-i", f"{drive.DISPLAY}+{r['x']},{r['y']}",
         "-c:v", "ffv1", str(RAW)],
        stdin=subprocess.PIPE,
    )
    time.sleep(1)

    m = app.js("return mantra")
    split = m.index(" ") + 1  # one typo after the first word
    app.keys(m[:split], KEY_MS)
    app.keys("z" if m[split].lower() != "z" else "x")
    time.sleep(0.8)
    app.keys(m[split:], KEY_MS)

    time.sleep(VIEW_FADE + 3)  # a few seconds of the rest timer
    # Fast-forward the ring rather than jumping to 0:00.
    app.js("const t = setInterval(() => (deadline -= 2000) < Date.now() && clearInterval(t), 30)")
    time.sleep(VIEW_FADE + 1.5)  # next round appears, then the GIF loops
    rec.communicate(b"q")

OUT.parent.mkdir(exist_ok=True)
subprocess.run(
    ["ffmpeg", "-y", "-loglevel", "error", "-i", str(RAW), "-vf",
     "fps=15,scale=600:-1:flags=lanczos,split[a][b];[a]palettegen=stats_mode=diff[p];"
     "[b][p]paletteuse=dither=bayer:bayer_scale=5:diff_mode=rectangle",
     "-loop", "0", str(OUT)],
    check=True,
)
print(OUT)
