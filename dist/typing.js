import { LitElement, html, nothing } from "./vendor/lit-core.min.js";
import { replay, emit } from "./dom.js";

// Type the mantra exactly (case ignored). Fires "typed" once it's complete,
// or "redraw" when another mantra is asked for.
class MantraTyping extends LitElement {
  static properties = { mantra: { state: true }, pos: { state: true } };

  createRenderRoot() { return this; }

  start(mantra) {
    this.mantra = mantra;
    this.pos = 0;
  }

  render() {
    if (!this.mantra) return nothing;
    const chars = Array.from(this.mantra);
    return html`
      <span class="kicker">Type the mantra</span>
      <p class="mantra" role="img" aria-label=${this.mantra}>${chars.map((c, i) =>
        html`<span class="ch ${i < this.pos ? "inked" : i === this.pos ? "current" : ""}" aria-hidden="true">${c}</span>`
      )}</p>
      <div class="rule" style="--p: ${this.pos / chars.length}"></div>
      <button class="redraw" type="button" title="Draw another mantra" aria-label="Draw another mantra"
        @mousedown=${(e) => e.preventDefault()} @click=${this.redraw}>
        <svg viewBox="0 0 24 24" aria-hidden="true"><!-- dice-5 from Lucide (ISC): https://lucide.dev -->
          <rect width="18" height="18" x="3" y="3" rx="2" ry="2" />
          <path d="M16 8h.01M8 8h.01M8 16h.01M16 16h.01M12 12h.01" />
        </svg>
      </button>
    `;
  }

  // The button never takes focus, so Enter and Space can't press it mid-mantra.
  redraw(e) {
    replay(e.currentTarget, "spin");
    emit(this, "redraw");
  }

  key(e) {
    if (e.ctrlKey || e.metaKey) return;
    const chars = Array.from(this.mantra);
    if (e.key === "Backspace") {
      e.preventDefault();
      if (this.pos > 0) this.pos--;
      return;
    }
    if (e.repeat || e.key.length !== 1) return;
    e.preventDefault();
    if (e.key.toLowerCase() !== chars[this.pos].toLowerCase()) {
      replay(this.querySelectorAll(".ch")[this.pos], "miss");
      replay(this.querySelector(".mantra"), "shake");
      return;
    }
    if (++this.pos === chars.length) emit(this, "typed");
  }
}

customElements.define("mantra-typing", MantraTyping);
