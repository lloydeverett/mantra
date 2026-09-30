import { LitElement, html, nothing } from "./vendor/lit-core.min.js";
import { replay, emit } from "./dom.js";
import { clockText } from "./commitment.js";

const FF_MS = 700; // how long End session takes to spin the clock down
const R = 46; // ring radius, in the SVG's 100-unit viewBox
const C = 2 * Math.PI * R;

// Counts down the session under the commitment. Fires "ending" when the user
// ends it early (with the fast-forward's length in ms) and "ended" at zero.
class SessionTimer extends LitElement {
  static properties = { commitment: { state: true }, tags: { state: true }, left: { state: true } };

  createRenderRoot() { return this; }

  // duration: minutes
  start(commitment, duration, tags) {
    this.commitment = commitment;
    this.tags = tags;
    this.total = duration * 60_000;
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

  // Jump to ms before the end, for scripts that skip the session.
  endIn(ms) {
    this.deadline = Date.now() + ms;
  }

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
          <circle class="track" cx="50" cy="50" r=${R} />
          <circle class="arc" cx="50" cy="50" r=${R} stroke-dasharray=${C} stroke-dashoffset=${(C * this.left) / this.total} />
        </svg>
        <div class="clock">
          <span class="time">${clockText(this.left)}</span>
          <span class="kicker">remaining</span>
        </div>
      </div>
      <p class="echo">${this.commitment}</p>
      ${this.tags.length ? html`<p class="tags">${this.tags.map((tag) => html`<span>@${tag}</span>`)}</p>` : nothing}
      <button class="end-session" type="button" title="End session" aria-label="End session" @click=${this.end}>
        <svg viewBox="0 0 24 24" aria-hidden="true">
          <path d="M4 6l8 6-8 6zM12 6l8 6-8 6z" />
        </svg>
      </button>
    `;
  }
}

customElements.define("session-timer", SessionTimer);
