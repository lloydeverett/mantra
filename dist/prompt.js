import { LitElement, html } from "./vendor/lit-core.min.js";
import { parseCommitment, formatDuration, singleLine } from "./commitment.js";
import { replay, emit, textRange } from "./dom.js";

// Auto uses the duration typed in the commitment, else the default.
const AUTO = "auto";
// Durations in minutes, as everywhere else.
const PRESETS = [AUTO, 5, 10, 20, 30, 45, 60, 120];

// State a commitment and pick a duration before the rule drains. Set
// defaultDuration and seconds (the time limit) before start().
// Fires "committed" ({ text, commitment, duration }) or "cancelled" (Escape or timeout).
//
// The entry is a contenteditable="plaintext-only" element, so the browser does
// the editing. Lit never renders inside it; the typed duration is coloured with
// the CSS Custom Highlight API, which leaves its text nodes alone.
class CommitmentPrompt extends LitElement {
  static properties = { text: { state: true }, preset: { state: true }, placeholder: { state: true } };

  createRenderRoot() { return this; }

  async start(placeholder) {
    this.text = "";
    this.preset = 0;
    this.placeholder = placeholder;
    this.open = true;
    clearTimeout(this.timeout);
    this.timeout = setTimeout(() => emit(this, "cancelled"), this.seconds * 1000);
    await this.updateComplete;
    this.entry.replaceChildren();
    this.focus();
    replay(this.querySelector(".rule"), "drain");
  }

  stop() {
    this.open = false;
    clearTimeout(this.timeout);
    this.entry?.blur(); // or the next mantra's keys would type into it
  }

  get entry() {
    return this.querySelector(".entry");
  }

  // Focus the entry with the caret at the end (WebKit would put it at the start).
  focus() {
    this.entry.focus();
    getSelection().selectAllChildren(this.entry);
    getSelection().collapseToEnd();
  }

  // What Enter would commit to: { commitment, duration, match }, with match
  // only when Auto uses a typed duration. A preset keeps the whole text.
  chosen(parsed = parseCommitment(this.text)) {
    const preset = PRESETS[this.preset];
    if (preset !== AUTO) return { commitment: this.text.trim(), duration: preset, match: null };
    return { ...parsed, duration: parsed.duration ?? this.defaultDuration };
  }

  // Keep it one line: Enter is ours, and pasted or dropped line breaks become spaces.
  beforeInput(e) {
    if (e.inputType === "insertParagraph" || e.inputType === "insertLineBreak") return e.preventDefault();
    const data = e.data ?? e.dataTransfer?.getData("text/plain");
    if (data && /[\r\n]/.test(data)) {
      e.preventDefault();
      document.execCommand("insertText", false, singleLine(data)); // unlike setting text, keeps undo
    }
  }

  input() {
    const entry = this.entry;
    // Any line breaks that slip through arrive as <br> or <div>; innerText reads those as newlines.
    if (entry.childElementCount && entry.innerText.trim()) {
      entry.textContent = singleLine(entry.innerText).trimEnd();
      this.focus(); // rewriting the text loses the caret
    }
    // An emptied contenteditable can keep a stray <br>, which would hide the placeholder.
    if (!entry.textContent) entry.replaceChildren();
    this.text = entry.textContent;
  }

  render() {
    if (this.text === undefined) return;
    const auto = parseCommitment(this.text).duration ?? this.defaultDuration;
    return html`
      <span class="kicker">Commit to what's next</span>
      <p class="entry" contenteditable="plaintext-only" spellcheck="false" role="textbox" aria-label="Commitment"
        aria-multiline="false" aria-placeholder=${this.placeholder} data-placeholder=${this.placeholder}
        @beforeinput=${this.beforeInput} @input=${this.input}
        @blur=${() => this.open && requestAnimationFrame(() => this.open && this.focus())}></p>
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

  updated() {
    if (this.text === undefined) return;
    const { match } = this.chosen();
    const range = match && textRange(this.entry, ...match);
    if (range) CSS.highlights.set("duration", new Highlight(range));
    else CSS.highlights.delete("duration");
  }

  // Keys the entry doesn't handle itself. Typing and editing are left to the browser.
  key(e) {
    if (e.isComposing) return;
    const step = (d) => (this.preset = (this.preset + d + PRESETS.length) % PRESETS.length);
    if (e.key === "Escape") {
      this.stop();
      emit(this, "cancelled");
    } else if (e.key === "Enter") {
      e.preventDefault();
      const { commitment, duration } = this.chosen();
      if (!commitment) return;
      this.stop();
      emit(this, "committed", { text: this.text, commitment, duration });
    } else if (e.key === "Tab") step(e.shiftKey ? -1 : 1);
    else if (e.key === "ArrowDown") step(1);
    else if (e.key === "ArrowUp") step(-1);
    else return;
    e.preventDefault();
  }
}

customElements.define("commitment-prompt", CommitmentPrompt);
