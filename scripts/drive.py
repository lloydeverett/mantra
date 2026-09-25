#!/usr/bin/env python3
"""Drive the real app over WebDriver for quick checks. Stdlib only.

Needs: apt `webkitgtk-webdriver xvfb`, and `cargo install tauri-driver`.

    import drive
    with drive.session() as app:
        app.js("return document.title")
        app.keys("hello")
        app.shot("target/shot.png")

Run directly for a smoke screenshot: python3 scripts/drive.py [out.png]
"""
import base64
import contextlib
import json
import os
import socket
import subprocess
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
APP = ROOT / "target" / "debug" / "Mantra"
LOG = ROOT / "target" / "drive.log"
PORT = 4444
DISPLAY = ":99"


def _req(method, path, body=None):
    data = json.dumps(body).encode() if body is not None else None
    req = urllib.request.Request(
        f"http://127.0.0.1:{PORT}{path}", data, {"Content-Type": "application/json"}, method=method
    )
    try:
        with urllib.request.urlopen(req) as r:
            return json.load(r)["value"]
    except urllib.error.HTTPError as e:
        raise RuntimeError(f"{method} {path}: {e.read().decode()}") from None


def _wait(ready, what):
    for _ in range(100):
        if ready():
            return
        time.sleep(0.1)
    raise TimeoutError(f"{what} did not start, see {LOG}")


def _port_open():
    with contextlib.suppress(OSError), socket.create_connection(("127.0.0.1", PORT), 0.2):
        return True
    return False


class App:
    def __init__(self, sid):
        self.sid = sid

    def _(self, method, path, body=None):
        return _req(method, f"/session/{self.sid}{path}", body)

    def js(self, script, *args):
        return self._("POST", "/execute/sync", {"script": script, "args": list(args)})

    def keys(self, text, delay=0):
        """Type text, pausing delay ms after each key."""
        actions = [
            a
            for ch in text
            for a in ({"type": "keyDown", "value": ch}, {"type": "keyUp", "value": ch}, {"type": "pause", "duration": delay})
        ]
        self._("POST", "/actions", {"actions": [{"type": "key", "id": "kbd", "actions": actions}]})

    def shot(self, path):
        Path(path).write_bytes(base64.b64decode(self._("GET", "/screenshot")))
        return path


@contextlib.contextmanager
def session(headless=True):
    subprocess.run(["cargo", "build", "--quiet"], cwd=ROOT, check=True)
    env = dict(os.environ)
    procs = []
    log = open(LOG, "w")
    try:
        if headless:
            # ponytail: fixed display number, pick a free one if parallel runs are ever needed
            # -nolisten unix: WSLg mounts /tmp/.X11-unix read-only; the abstract socket still works.
            # -displayfd: Xvfb prints the display number once it accepts connections.
            xvfb = subprocess.Popen(
                ["Xvfb", DISPLAY, "-displayfd", "1", "-nolisten", "unix", "-screen", "0", "1280x800x24"],
                stdout=subprocess.PIPE, stderr=log,
            )
            procs.append(xvfb)
            xvfb.stdout.readline()
            env.update(DISPLAY=DISPLAY, GDK_BACKEND="x11")
            env.pop("WAYLAND_DISPLAY", None)
        procs.append(subprocess.Popen(["tauri-driver", "--port", str(PORT)], env=env, stdout=log, stderr=log))
        _wait(_port_open, "tauri-driver")
        caps = {"browserName": "wry", "tauri:options": {"application": str(APP)}}
        sid = _req("POST", "/session", {"capabilities": {"alwaysMatch": caps}})["sessionId"]
        try:
            yield App(sid)
        finally:
            _req("DELETE", f"/session/{sid}")
    finally:
        for p in reversed(procs):
            p.terminate()
            p.wait()
        log.close()


if __name__ == "__main__":
    with session() as app:
        print(app.js("return document.title"))
        print(app.shot(sys.argv[1] if len(sys.argv) > 1 else ROOT / "target" / "shot.png"))
