import { LitElement, html } from "./vendor/lit-core.min.js";
import "./typing.js";
import "./prompt.js";
import "./session.js";

// Marcus Aurelius, Meditations, in George Long's public-domain translation
// (Project Gutenberg #15877), with "thou/thy" modernised to "you/your". Cited as book.section.
// Plain ASCII only, so every character can be typed on a standard keyboard.
const MANTRAS = [
  "Confine yourself to the present.",                                    // 7.29
  "Wipe out the imagination. Stop the pulling of the strings.",          // 7.29
  "Attend to the matter which is before you.",                           // 8.22
  "Direct your attention to what is said.",                              // 7.30
  "Take pleasure in one thing and rest in it.",                          // 6.7
  "Occupy yourself with few things.",                                    // 4.24
  "Do not be whirled about.",                                            // 4.22
  "Let nothing else distract you.",                                      // 8.1
  "Throw away your books; no longer distract yourself.",                 // 2.2
  "Retire into yourself.",                                               // 7.28
  "Look within. Within is the fountain of good.",                        // 7.59
  "The soul is dyed by the thoughts.",                                   // 5.16
  "Do every act of your life as if it were the last.",                   // 2.5
  "Let not future things disturb you.",                                  // 7.8
  "Do not disturb yourself by thinking of the whole of your life.",      // 8.36
  "Every man lives the present time only, and loses only this.",         // 12.26
  "Do not waste the remainder of your life in thoughts about others.",   // 3.4
];
// Short in dev builds (see src/main.rs).
const DEFAULT_DURATION = window.MANTRA_RELEASE ? 20 : 1; // session minutes when none is typed or picked
const PROMPT_SECONDS = window.MANTRA_RELEASE ? 60 : 20; // time to commit before the mantra returns
const SAMPLE = "Wash the dishes 20m"; // placeholder before the first commitment
const PAUSE_MS = 900; // beat between a finished view and the next
const DIM = 0.45; // opacity of the black screen overlay while the mantra waits
const DIM_LEAD_SECONDS = 120; // darken again over this last stretch of the session

// Darken every screen (see src/dim.rs). Keys are [seconds from now, opacity],
// faded linearly from the current opacity. A no-op outside Tauri.
const dim = (...keys) => window.__TAURI__?.core.invoke("dim", { keys });

// Runs the round: typing -> prompt -> session -> typing (a new round).
// A cancelled prompt returns to the same mantra in the same round.
class MantraApp extends LitElement {
  static properties = { view: { reflect: true, attribute: "data-view" }, round: { state: true } };

  createRenderRoot() { return this; }

  constructor() {
    super();
    this.round = 0;
    this.lastCommitment = null;
    this.active = null; // the view taking keys, if any
  }

  render() {
    return html`
      <header>
        <span class="kicker">Mantra</span>
        <span class="kicker">Round ${String(this.round).padStart(2, "0")}</span>
      </header>
      <main>
        <mantra-typing class="view" @typed=${this.typed}></mantra-typing>
        <commitment-prompt class="view" .defaultDuration=${DEFAULT_DURATION} .seconds=${PROMPT_SECONDS}
          @committed=${this.committed} @cancelled=${this.typeMantra}></commitment-prompt>
        <session-timer class="view" @ending=${(e) => dim([e.detail / 1000, DIM])}
          @ended=${() => setTimeout(() => this.newRound(), PAUSE_MS)}></session-timer>
      </main>
    `;
  }

  firstUpdated() {
    this.typing = this.querySelector("mantra-typing");
    this.prompt = this.querySelector("commitment-prompt");
    this.session = this.querySelector("session-timer");
    addEventListener("keydown", (e) => this.active?.key(e));
    this.newRound();
  }

  newRound() {
    // New mantra each round, never the same one twice in a row.
    const others = MANTRAS.filter((m) => m !== this.mantra);
    this.mantra = others[Math.floor(Math.random() * others.length)];
    this.round++;
    this.typeMantra();
  }

  typeMantra() {
    this.prompt.stop();
    this.typing.start(this.mantra);
    this.active = this.typing;
    this.view = "typing";
    dim([0.6, DIM]);
  }

  typed() {
    this.active = null;
    dim([PAUSE_MS / 1000, 0]);
    setTimeout(async () => {
      this.view = "prompt";
      await this.updateComplete; // the entry can't take focus while its view is hidden
      this.prompt.start(this.lastCommitment ?? SAMPLE);
      this.active = this.prompt;
    }, PAUSE_MS);
  }

  committed(e) {
    const { text, commitment, duration } = e.detail;
    this.lastCommitment = text;
    this.active = null;
    this.session.start(commitment, duration);
    this.view = "session";
    const s = duration * 60;
    dim([Math.max(0, s - DIM_LEAD_SECONDS), 0], [s, DIM]);
  }
}

customElements.define("mantra-app", MantraApp);
