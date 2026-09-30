import { LitElement, html, nothing } from "./vendor/lit-core.min.js";
import { clockText } from "./commitment.js";

const invoke = (cmd) => window.__TAURI__?.core.invoke(cmd);

// Counts down a snooze (see src/snooze.rs), and cancels it when clicked. Shows
// nothing unless one is running. src/snooze.rs calls show() whenever one starts
// or stops; it's asked here only for one already running when the page loads.
class SnoozeCountdown extends LitElement {
  static properties = { left: { state: true } };

  createRenderRoot() { return this; }

  connectedCallback() {
    super.connectedCallback();
    invoke("snoozed")?.then((until) => this.show(until));
  }

  // until: when it runs out, in ms since the epoch, or null if it's stopped.
  show(until) {
    this.until = until;
    clearTimeout(this.timer);
    this.tick();
  }

  // Wall-clock, like the session timer, and woken at each second it shows.
  tick = () => {
    this.left = this.until ? Math.max(0, this.until - Date.now()) : 0;
    if (this.left > 0) this.timer = setTimeout(this.tick, this.left % 1000 || 1000);
  };

  render() {
    if (!this.left) return nothing;
    return html`
      <button type="button" title="Cancel snooze" aria-label="Cancel snooze" tabindex="-1"
        @mousedown=${(e) => e.preventDefault()} @click=${() => invoke("unsnooze")}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><!-- bell-off from Lucide (ISC): https://lucide.dev -->
          <path d="M10.268 21a2 2 0 0 0 3.464 0" />
          <path d="M17 17H4a1 1 0 0 1-.74-1.673C4.59 13.956 6 12.499 6 8a6 6 0 0 1 .258-1.742" />
          <path d="m2 2 20 20" />
          <path d="M8.668 3.01A6 6 0 0 1 18 8c0 2.687.77 4.653 1.707 6.05" />
        </svg>
        <span class="time">${clockText(this.left)}</span>
        <svg viewBox="0 0 24 24" aria-hidden="true"><!-- x from Lucide (ISC) -->
          <path d="M18 6 6 18" /><path d="m6 6 12 12" />
        </svg>
      </button>
    `;
  }
}

customElements.define("snooze-countdown", SnoozeCountdown);
