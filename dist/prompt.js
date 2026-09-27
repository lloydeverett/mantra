import { LitElement, html } from "./vendor/lit-core.min.js";
import { parseCommitment, formatDuration } from "./commitment.js";
import { replay, emit } from "./dom.js";

// Auto uses the duration typed in the commitment, else the default.
const AUTO = "auto";
// Durations in minutes, as everywhere else.
const PRESETS = [AUTO, 5, 10, 20, 30, 45, 60, 120];

// State a commitment and pick a duration before the rule drains. Set
// defaultDuration and seconds (the time limit) before start().
// Fires "committed" ({ text, commitment, duration }) or "cancelled" (Escape or timeout).
class CommitmentPrompt extends LitElement {
  static properties = { text: { state: true }, preset: { state: true }, placeholder: { state: true } };

  createRenderRoot() { return this; }

  start(placeholder) {
    this.text = "";
    this.preset = 0;
    this.placeholder = placeholder;
    clearTimeout(this.timeout);
    this.timeout = setTimeout(() => emit(this, "cancelled"), this.seconds * 1000);
    this.updateComplete.then(() => replay(this.querySelector(".rule"), "drain"));
  }

  stop() {
    clearTimeout(this.timeout);
  }

  // What Enter would commit to: { commitment, duration, match }, with match
  // only when Auto uses a typed duration. A preset keeps the whole text.
  chosen(parsed = parseCommitment(this.text)) {
    const preset = PRESETS[this.preset];
    if (preset !== AUTO) return { commitment: this.text.trim(), duration: preset, match: null };
    return { ...parsed, duration: parsed.duration ?? this.defaultDuration };
  }

  render() {
    if (this.text === undefined) return;
    const t = this.text;
    const parsed = parseCommitment(t);
    const { match } = this.chosen(parsed);
    const auto = parsed.duration ?? this.defaultDuration;
    return html`
      <span class="kicker">Commit to what's next</span>
      <p class="entry" role="textbox" aria-label="Commitment" aria-placeholder=${this.placeholder}>${
        t
          ? match
            ? html`${t.slice(0, match[0])}<span class="duration">${t.slice(...match)}</span>${t.slice(match[1])}<span class="caret"></span>`
            : html`${t}<span class="caret"></span>`
          : html`<span class="caret"></span><span class="placeholder">${this.placeholder}</span>`
      }</p>
      <div class="rule" style="--t: ${this.seconds}s" title="Time left to commit"></div>
      <div class="presets" role="radiogroup" aria-label="Duration">
        ${PRESETS.map((m, i) => html`
          <button type="button" class="preset" role="radio" aria-checked=${i === this.preset} tabindex="-1"
            @mousedown=${(e) => e.preventDefault()} @click=${() => (this.preset = i)}>
            ${m === AUTO ? html`Auto <span class="auto">${formatDuration(auto)}</span>` : formatDuration(m)}
          </button>`)}
      </div>
    `;
  }

  key(e) {
    const step = (d) => (this.preset = (this.preset + d + PRESETS.length) % PRESETS.length);
    if (e.key === "Escape") {
      this.stop();
      emit(this, "cancelled");
    } else if (e.key === "Enter") {
      const { commitment, duration } = this.chosen();
      if (!commitment) return;
      this.stop();
      emit(this, "committed", { text: this.text, commitment, duration });
    } else if (e.key === "Tab") step(e.shiftKey ? -1 : 1);
    else if (e.key === "ArrowDown") step(1);
    else if (e.key === "ArrowUp") step(-1);
    else if (e.key === "Backspace") {
      // The entry isn't a native input, so these editing shortcuts are ours to provide.
      if (e.ctrlKey || e.metaKey) this.text = "";
      else if (e.altKey) this.text = this.text.replace(/\S*\s*$/, "");
      else this.text = this.text.slice(0, -1);
    } else if (e.key.length === 1 && !e.ctrlKey && !e.metaKey) {
      this.text += e.key;
    } else return;
    e.preventDefault();
  }
}

customElements.define("commitment-prompt", CommitmentPrompt);
