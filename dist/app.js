import { LitElement, html } from "./vendor/lit-core.min.js";
import "./typing.js";
import "./prompt.js";
import "./session.js";

// Keep to ASCII so these are easy to type.
const MANTRAS = [
    "I will not be whirled about.",
    "I will give my whole attention to one thing.",
    "I am going to finish what I started before I start something new.",
    "I will decide what matters today, and let the rest wait.",
    "I will spend my attention with purpose.",
    "I will guard the hours that are mine.",
    "I am going to start what I planned to start.",
    "I am going to work on the hard thing first.",
    "I will say no to good things to make room for the best.",
    "I will choose where my attention goes.",
    "I am going to close what I do not need open.",
    "I am going to notice when I drift, and come back.",
];
// Placeholders for the commitment prompt. Each ends in a duration to show that one can be typed.
const SAMPLES = [
    "Paint the town red 20m",
    "Stir the pot 20m",
    "Ruffle feathers 20m",
    "Kick up a fuss 20m",
    "Set the cat among the pigeons 20m",
    "Push buttons 20m",
    "Make a scene 20m",
    "Poke the bear 20m",
    "Muddy the waters 20m",
    "Bring the house down 20m",
    "Knock 'em dead 20m",
    "Throw your weight around 20m",
    "Push the envelope 20m",
    "Make a splash 20m",
    "Play devil's advocate 20m",
    "Raise a few eyebrows 20m",
];
// Short in dev builds (see src/main.rs).
const DEFAULT_DURATION = window.MANTRA_RELEASE ? 20 : 1; // session minutes when none is typed or picked
const PROMPT_SECONDS = window.MANTRA_RELEASE ? 60 : 20; // time to commit before the mantra returns
const PAUSE_MS = 900; // beat between a finished view and the next
const DIM = 0.45; // opacity of the black screen overlay while the mantra waits
const DIM_LEAD_SECONDS = 120; // darken again over this last stretch of the session

// Darken every screen (see src/dim.rs). Keys are [seconds from now, opacity],
// faded linearly from the current opacity. A no-op outside Tauri.
const dim = (...keys) => window.__TAURI__?.core.invoke("dim", { keys });

// Any element of list other than last, so nothing shows twice in a row.
const pick = (list, last) => {
    const others = list.filter((x) => x !== last);
    return others[Math.floor(Math.random() * others.length)];
};

// The webview's own menu offers Reload and the like. The entry keeps its menu for Paste.
addEventListener("contextmenu", (e) => e.target.closest?.("[contenteditable]") || e.preventDefault());

// Runs the round: typing -> prompt -> session -> typing (a new round).
// A cancelled prompt returns to the same mantra in the same round.
class MantraApp extends LitElement {
    static properties = { view: { reflect: true, attribute: "data-view" } };

    createRenderRoot() { return this; }

    constructor() {
        super();
        this.active = null; // the view taking keys, if any
    }

    render() {
        return html`
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
        this.mantra = pick(MANTRAS, this.mantra);
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
            this.sample = pick(SAMPLES, this.sample);
            this.prompt.start(this.sample);
            this.active = this.prompt;
        }, PAUSE_MS);
    }

    committed(e) {
        const { commitment, duration } = e.detail;
        this.active = null;
        this.session.start(commitment, duration);
        this.view = "session";
        const s = duration * 60;
        dim([Math.max(0, s - DIM_LEAD_SECONDS), 0], [s, DIM]);
    }
}

customElements.define("mantra-app", MantraApp);
