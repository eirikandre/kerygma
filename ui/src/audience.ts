// Vinduet på den andre skjermen i talervisningen (presenter.ts): bare slidene, i fullskjerm.
// Hovedvinduet bestemmer hvilken slide som vises. Tastene her (også fra en fjernkontroll,
// som sender dem til vinduet som har fokus) sendes tilbake til hovedvinduet.
import { emitTo, listen } from "@tauri-apps/api/event";
import { api } from "./api.ts";
import { type Deck, showDeck } from "./present.ts";
import { PRESENTER_EVENTS, type PresenterInit, presenterKey } from "./presenterEvents.ts";
import "./styles.css";

// Vinduet viser bare slidene, uten menylinjen (som det ellers får på Windows).
void api.setMenuVisible(false).catch(() => {});

let deck: Deck | undefined;
let pending: number | undefined;

const listening = [
  listen<PresenterInit>(PRESENTER_EVENTS.init, async (event) => {
    if (deck) return;
    deck = await showDeck(event.payload.slides, { keyboard: false });
    deck.slide(pending ?? event.payload.index);
  }),
  listen<number>(PRESENTER_EVENTS.slide, (event) => {
    if (deck) deck.slide(event.payload);
    else pending = event.payload;
  }),
];

document.addEventListener(
  "keydown",
  (event) => {
    const key = presenterKey(event);
    if (!key) return;
    event.preventDefault();
    event.stopPropagation();
    void emitTo("main", PRESENTER_EVENTS.key, key);
  },
  true,
);

// Hovedvinduet sender slidene når det vet at vi lytter.
void Promise.all(listening).then(() => emitTo("main", PRESENTER_EVENTS.ready));
