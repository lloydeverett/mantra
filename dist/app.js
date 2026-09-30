import { LitElement, html, nothing } from "./vendor/lit-core.min.js";
import "./typing.js";
import "./prompt.js";
import "./session.js";

// Keep to ASCII so these are easy to type.
const MANTRAS = [
    "To strive, to seek, to find, and not to yield.",
    "There is iron in me yet.",
    "Be not afraid of greatness.",
    "Celestial light, shine inward.",
    "Claim yourself for yourself.",
    "I will not be whirled about.",
    "No coward soul is mine.",
    "To live is to fight.",
    "Dare to be wise.",
    "Work conquers all.",
    "Nothing from nothing.",
    "Even higher.",
    "Glory takes a steep and difficult path.",
    "Onwards and upwards."
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
    "Raise a few eyebrows 20m",
];
// Short in dev builds (see src/main.rs).
const DEFAULT_DURATION = window.MANTRA_RELEASE ? 20 : 1; // session minutes when none is typed or picked
const PROMPT_SECONDS = window.MANTRA_RELEASE ? 60 : 20; // time to commit before the mantra returns
const PAUSE_MS = 900; // beat between a finished view and the next
const DIM = 0.45; // opacity of the black screen overlay while the mantra waits
const DIM_LEAD_SECONDS = 120; // darken again over this last stretch of the session
const MENU_GAP = 4; // px between the menu button and the menu it pops up

// Darken every screen (see src/dim.rs). Keys are [seconds from now, opacity],
// faded linearly from the current opacity. A no-op outside Tauri.
const dim = (...keys) => window.__TAURI__?.core.invoke("dim", { keys });

// Pop up the native menu below the button (see src/menu.rs).
const openMenu = (button) => {
    const r = button.getBoundingClientRect();
    window.__TAURI__?.core.invoke("menu", { x: r.left, y: r.bottom + MENU_GAP });
};

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
      ${window.MANTRA_MENU ? html`
        <button class="menu" type="button" title="Menu" aria-label="Menu"
          @mousedown=${(e) => e.preventDefault()} @click=${(e) => openMenu(e.currentTarget)}>
          <svg viewBox="0 0 24 24" aria-hidden="true"><!-- ellipsis-vertical from Lucide (ISC): https://lucide.dev -->
            <circle cx="12" cy="12" r="1" /><circle cx="12" cy="5" r="1" /><circle cx="12" cy="19" r="1" />
          </svg>
        </button>` : nothing}
      <main>
        <mantra-typing class="view" @typed=${this.typed} @redraw=${this.newRound}></mantra-typing>
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
        const { commitment, duration, tags } = e.detail;
        this.active = null;
        this.session.start(commitment, duration, tags);
        this.view = "session";
        const s = duration * 60;
        dim([Math.max(0, s - DIM_LEAD_SECONDS), 0], [s, DIM]);
    }
}

customElements.define("mantra-app", MantraApp);
