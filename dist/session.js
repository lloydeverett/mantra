import { LitElement, html, nothing } from "./vendor/lit-core.min.js";
import { replay, emit } from "./dom.js";

const FF_MS = 700; // how long End session takes to spin the clock down
const C = 2 * Math.PI * 46; // ring circumference

const clockText = (ms) => {
  const s = Math.ceil(ms / 1000), h = Math.floor(s / 3600), m = Math.floor(s / 60) % 60;
  const ss = String(s % 60).padStart(2, "0");
  return h ? `${h}:${String(m).padStart(2, "0")}:${ss}` : `${m}:${ss}`;
};

// Counts down the session under the commitment. Fires "ending" when the user
// ends it early (with the fast-forward's length in ms) and "ended" at zero.
class SessionTimer extends LitElement {
  static properties = { commitment: { state: true }, left: { state: true } };

  createRenderRoot() { return this; }

  start(commitment, minutes) {
    this.commitment = commitment;
    this.total = minutes * 60_000;
    this.deadline = Date.now() + this.total;
    this.ff = null;
    this.tick();
  }

  // Driven by a wall-clock deadline, so it stays correct if frames are paused while hidden.
  tick = () => {
    let left = Math.max(0, this.deadline - Date.now());
    if (this.ff) {
      // Fast-forwarding: race the remaining time down to zero, accelerating as it goes.
      const t = Math.min(1, (Date.now() - this.ff.at) / FF_MS);
      left = Math.min(left, this.ff.from * (1 - t * t));
    }
    this.left = left;
    if (left > 0) requestAnimationFrame(this.tick);
    else emit(this, "ended");
  };

  end(e) {
    e.currentTarget.blur(); // so a later Space can't press it again
    if (!this.left || this.ff) return;
    replay(e.currentTarget, "zoom");
    if (matchMedia("(prefers-reduced-motion: reduce)").matches) this.deadline = Date.now();
    else this.ff = { from: Math.max(0, this.deadline - Date.now()), at: Date.now() };
    emit(this, "ending", FF_MS);
  }

  render() {
    if (this.commitment === undefined) return nothing;
    return html`
      <span class="kicker" aria-hidden="true">&nbsp;</span> <!-- Empty spacer: keeps the ring where it was. -->
      <div class="ring">
        <svg viewBox="0 0 100 100" aria-hidden="true">
          <circle class="track" cx="50" cy="50" r="46" />
          <circle class="arc" cx="50" cy="50" r="46" stroke-dasharray=${C} stroke-dashoffset=${(C * this.left) / this.total} />
        </svg>
        <div class="clock">
          <span class="time">${clockText(this.left)}</span>
          <span class="kicker">remaining</span>
        </div>
      </div>
      <p class="echo">${this.commitment}</p>
      <button class="skip" type="button" title="End session" aria-label="End session" @click=${this.end}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 6l8 6-8 6zM12 6l8 6-8 6z" />
        </svg>
      </button>
    `;
  }
}

customElements.define("session-timer", SessionTimer);
