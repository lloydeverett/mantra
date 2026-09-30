import { LitElement, html } from "./vendor/lit-core.min.js";
import { parseCommitment, formatDuration, singleLine } from "./commitment.js";
import { TAGS, parseTags, toggleTag } from "./tags.js";
import { replay, emit, textRange } from "./dom.js";

// Auto uses the duration typed in the commitment, else the default.
const AUTO = "auto";
// Durations in minutes, as everywhere else.
const PRESETS = [AUTO, 5, 10, 20, 30, 45, 60, 120];

// State a commitment and pick a duration and tags before the rule drains. Set
// defaultDuration and seconds (the time limit) before start().
// Fires "committed" ({ commitment, duration, tags }) or "cancelled" (Escape or timeout).
//
// The entry is a contenteditable="plaintext-only" element, so the browser does
// the editing. Lit never renders inside it; the typed duration is coloured with
// the CSS Custom Highlight API, which leaves its text nodes alone. Tags are only
// ever text in the entry: their chips edit it, and show what it holds.
class CommitmentPrompt extends LitElement {
  static properties = { text: { state: true }, preset: { state: true }, placeholder: { state: true } };

  createRenderRoot() { return this; }

  connectedCallback() {
    super.connectedCallback();
    // Clicks elsewhere (the presets included) mustn't take focus, or the caret, from the entry.
    window.addEventListener("mousedown", (e) => this.open && !this.entry.contains(e.target) && e.preventDefault());
  }

  async start(placeholder) {
    this.text = "";
    this.preset = 0;
    this.placeholder = placeholder;
    this.open = true; // from start() to stop(); nothing renders from it, so it isn't reactive
    clearTimeout(this.timeout);
    this.timeout = setTimeout(() => emit(this, "cancelled"), this.seconds * 1000);
    await this.updateComplete;
    this.entry.replaceChildren();
    this.entry.focus();
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

  get started() {
    return this.text !== undefined;
  }

  // What Enter would commit to: { commitment, duration, match, tags }, with match
  // only when Auto uses a typed duration. A preset keeps a typed duration in the
  // text; tags always come out of it.
  chosen() {
    const preset = PRESETS[this.preset];
    const parsed = preset === AUTO ? parseCommitment(this.text) : { commitment: this.text, duration: preset, match: null };
    const { text, tags } = parseTags(parsed.commitment);
    return { ...parsed, commitment: text, duration: parsed.duration ?? this.defaultDuration, tags };
  }

  // Add the tag to the end of the text, or take every copy of it out. The edits go
  // through execCommand so they undo like typing, and the caret keeps its place.
  toggle(tag) {
    const entry = this.entry, sel = getSelection();
    let caret = this.text.length; // read before focus(), which may move it
    if (sel.rangeCount && entry.contains(sel.focusNode)) {
      const before = document.createRange();
      before.setStart(entry, 0);
      before.setEnd(sel.focusNode, sel.focusOffset);
      caret = before.toString().length;
    }
    entry.focus();
    for (const [start, end, insert] of toggleTag(this.text, tag)) {
      sel.removeAllRanges();
      sel.addRange(this.range(start, end));
      document.execCommand(insert ? "insertText" : "delete", false, insert);
      if (start < caret) caret -= Math.min(end, caret) - start;
    }
    sel.removeAllRanges();
    sel.addRange(this.range(caret, caret));
  }

  // A Range over characters [start, end) of the entry, or at its end if it's empty.
  range(start, end) {
    const range = textRange(this.entry, start, end);
    if (range) return range;
    const empty = document.createRange();
    empty.selectNodeContents(this.entry);
    empty.collapse(false);
    return empty;
  }

  // Keep it one line: Enter is ours, and pasted line breaks become spaces.
  beforeInput(e) {
    if (e.inputType === "insertParagraph" || e.inputType === "insertLineBreak") return e.preventDefault();
    const data = e.data ?? e.dataTransfer?.getData("text/plain");
    if (!data || !/[\r\n]/.test(data)) return;
    e.preventDefault();
    // A drop can't be re-inserted where it landed, so multi-line drops are refused.
    if (e.inputType !== "insertFromDrop") document.execCommand("insertText", false, singleLine(data)); // keeps undo
  }

  input() {
    const entry = this.entry;
    // Line breaks that slip past beforeInput arrive as <br> or <div>. Swap each for a space
    // in place, which leaves the caret where it was. A lone <br> at the end only holds
    // an empty last line open, and goes with the text below.
    for (let el; (el = [...entry.children].find((c) => !(c.tagName === "BR" && c === entry.lastChild))); )
      el.replaceWith(" ", ...el.childNodes);
    // An emptied contenteditable can keep a stray <br>, which would hide the placeholder.
    if (!entry.textContent) entry.replaceChildren();
    this.text = entry.textContent;
  }

  render() {
    if (!this.started) return;
    const auto = parseCommitment(this.text).duration ?? this.defaultDuration;
    const { tags } = parseTags(this.text);
    return html`
      <span class="kicker">Commit to what's next</span>
      <p class="field"><span class="entry" contenteditable="plaintext-only" spellcheck="false" role="textbox"
        aria-label="Commitment" aria-multiline="false" aria-placeholder=${this.placeholder}
        @beforeinput=${this.beforeInput} @input=${this.input}></span><span class="placeholder" aria-hidden="true"
        ?hidden=${this.text} @mousedown=${() => this.entry.focus()}>${this.placeholder}</span></p>
      <div class="rule" style="--t: ${this.seconds}s" title="Time left to commit"></div>
      <div class="presets" role="radiogroup" aria-label="Duration">
        ${PRESETS.map((m, i) => html`
          <button type="button" class="preset" role="radio" aria-checked=${i === this.preset} tabindex="-1" @click=${() => (this.preset = i)}>
            ${m === AUTO ? html`Auto <span class="auto">${formatDuration(auto)}</span>` : formatDuration(m)}
          </button>`)}
      </div>
      <div class="tag-chips" role="group" aria-label="Tags">
        ${TAGS.map((tag) => html`
          <button type="button" class="preset tag-chip" aria-pressed=${tags.includes(tag)} tabindex="-1" @click=${() => this.toggle(tag)}>@${tag}</button>`)}
      </div>
    `;
  }

  updated() {
    if (!this.started) return;
    const { match } = this.chosen();
    const range = match && textRange(this.entry, ...match);
    if (range) CSS.highlights.set("duration", new Highlight(range));
    else CSS.highlights.delete("duration");
    const tags = parseTags(this.text).matches.map(({ match }) => textRange(this.entry, ...match)).filter(Boolean);
    if (tags.length) CSS.highlights.set("tag", new Highlight(...tags));
    else CSS.highlights.delete("tag");
  }

  // Keys the entry doesn't handle itself. Typing and editing are left to the browser.
  key(e) {
    if (e.isComposing || e.keyCode === 229) return; // WebKit can send the Enter ending an IME composition after it
    const step = (d) => (this.preset = (this.preset + d + PRESETS.length) % PRESETS.length);
    if (e.key === "Escape") {
      this.stop();
      emit(this, "cancelled");
    } else if (e.key === "Enter") {
      e.preventDefault();
      const { commitment, duration, tags } = this.chosen();
      if (!commitment) return;
      this.stop();
      emit(this, "committed", { commitment, duration, tags });
    } else if (e.key === "Tab") step(e.shiftKey ? -1 : 1);
    else if (e.key === "ArrowDown") step(1);
    else if (e.key === "ArrowUp") step(-1);
    else return;
    e.preventDefault();
  }
}

customElements.define("commitment-prompt", CommitmentPrompt);
